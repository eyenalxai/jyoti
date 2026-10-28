import { Context, Effect, Layer, Option, Schema, String as Str } from "effect"

import { AppConfig } from "@/lib/config"

class KeyringError extends Schema.TaggedError<KeyringError>()("KeyringError", {
  operation: Schema.Literals(["read", "write", "delete", "start"]),
  account: Schema.String,
  message: Schema.String,
  cause: Schema.Defect(),
}) {}

interface KeyringShape {
  readonly get: (account: string) => Effect.Effect<Option.Option<string>, KeyringError>
  readonly set: (account: string, secret: string) => Effect.Effect<void, KeyringError>
  readonly remove: (account: string) => Effect.Effect<void, KeyringError>
}

interface ToolResult {
  readonly stdout: string
  readonly stderr: string
  readonly code: number
}

const runTool = (args: readonly string[], input?: string): Promise<ToolResult> => {
  const proc = Bun.spawn(["secret-tool", ...args], {
    stdin: input === undefined ? "ignore" : "pipe",
    stdout: "pipe",
    stderr: "pipe",
  })

  if (input !== undefined && proc.stdin !== undefined) {
    void proc.stdin.write(input)
    void proc.stdin.end()
  }

  return Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]).then(([stdout, stderr, code]) => ({ stdout, stderr, code }))
}

const unavailable = (
  operation: KeyringError["operation"],
  account: string,
  cause: unknown,
): KeyringError =>
  new KeyringError({
    operation,
    account,
    message:
      "Could not reach the OS keyring. Install libsecret-tools, unlock the keyring and try again.",
    cause,
  })

class Keyring extends Context.Service<Keyring, KeyringShape>()(
  "jyoti/lib/keyring/service/Keyring",
) {
  static readonly layer = Layer.effect(
    Keyring,
    Effect.gen(function* layer() {
      const { keyringService } = yield* AppConfig
      const attributes = (account: string): readonly string[] => [
        "service",
        keyringService,
        "account",
        account,
      ]

      const get = Effect.fn("Keyring.get")(function* get(account: string) {
        const result = yield* Effect.tryPromise({
          try: () => runTool(["lookup", ...attributes(account)]),
          catch: (cause) => unavailable("read", account, cause),
        })

        if (result.code === 0) {
          const secret = Str.trim(result.stdout)
          return secret.length === 0 ? Option.none<string>() : Option.some(secret)
        }

        if (result.code === 1 && Str.trim(result.stderr).length === 0) {
          return Option.none<string>()
        }

        return yield* new KeyringError({
          operation: "read",
          account,
          message: "The OS keyring refused to read this bulb's token. Unlock it and try again.",
          cause: new Error(Str.trim(result.stderr) || "secret-tool lookup failed"),
        })
      })

      const set = Effect.fn("Keyring.set")(function* set(account: string, secret: string) {
        const result = yield* Effect.tryPromise({
          try: () => runTool(["store", `--label=jyoti ${account}`, ...attributes(account)], secret),
          catch: (cause) => unavailable("write", account, cause),
        })

        if (result.code !== 0) {
          return yield* new KeyringError({
            operation: "write",
            account,
            message: "The OS keyring refused to save this bulb's token. Unlock it and try again.",
            cause: new Error(Str.trim(result.stderr) || "secret-tool store failed"),
          })
        }
        return yield* Effect.void
      })

      const remove = Effect.fn("Keyring.remove")(function* remove(account: string) {
        const result = yield* Effect.tryPromise({
          try: () => runTool(["clear", ...attributes(account)]),
          catch: (cause) => unavailable("delete", account, cause),
        })

        if (result.code !== 0 && Str.trim(result.stderr).length > 0) {
          return yield* new KeyringError({
            operation: "delete",
            account,
            message: "The OS keyring refused to delete this bulb's token.",
            cause: new Error(Str.trim(result.stderr)),
          })
        }
        return yield* Effect.void
      })

      return Keyring.of({ get, set, remove })
    }),
  )
}

export { Keyring, KeyringError }
