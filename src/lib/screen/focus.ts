import { Array as Arr, Config, Effect, Option, Schema } from "effect"
import { createConnection } from "node:net"
import path from "node:path"

const SOCKET_TIMEOUT_MS = 300

const HyprlandMonitorSchema = Schema.Struct({
  name: Schema.String,
  focused: Schema.Boolean,
})
const HyprlandMonitorsSchema = Schema.Array(HyprlandMonitorSchema)

const readSocket = (socketPath: string): Effect.Effect<string, Error> =>
  Effect.raceFirst(
    Effect.callback<string, Error>((resume) => {
      const chunks: Buffer[] = []
      const socket = createConnection(socketPath)

      socket.on("connect", () => {
        socket.write("j/monitors")
      })
      socket.on("data", (chunk: Buffer) => {
        chunks.push(chunk)
      })
      socket.on("end", () => {
        resume(Effect.succeed(Buffer.concat(chunks).toString("utf8")))
      })
      socket.on("error", (error: Error) => {
        resume(Effect.fail(error))
      })

      return Effect.sync(() => {
        socket.destroy()
      })
    }),
    Effect.sleep(SOCKET_TIMEOUT_MS).pipe(
      Effect.flatMap(() => Effect.fail(new Error("Hyprland did not answer."))),
    ),
  )

const parseFocused = (text: string): string | null => {
  const decoded = Schema.decodeUnknownOption(Schema.fromJsonString(HyprlandMonitorsSchema))(text)
  if (Option.isNone(decoded)) {
    return null
  }
  return Option.match(
    Arr.findFirst(decoded.value, (monitor) => monitor.focused),
    {
      onNone: () => null,
      onSome: (monitor) => monitor.name,
    },
  )
}

const socketPaths = (signature: string, runtimeDir: Option.Option<string>): readonly string[] => {
  const primary = Option.match(runtimeDir, {
    onNone: () => null,
    onSome: (directory) => path.join(directory, "hypr", signature, ".socket.sock"),
  })
  const fallback = path.join("/tmp/hypr", signature, ".socket.sock")
  return primary === null || primary === fallback ? [fallback] : [primary, fallback]
}

const focusedMonitor = (
  signature: Option.Option<string>,
  runtimeDir: Option.Option<string>,
): Effect.Effect<string | null> => {
  if (Option.isNone(signature)) {
    return Effect.succeed(null)
  }
  const paths = socketPaths(signature.value, runtimeDir)
  return Effect.gen(function* readFocus() {
    for (const socketPath of paths) {
      const reply = yield* readSocket(socketPath).pipe(Effect.option)
      if (Option.isSome(reply)) {
        return parseFocused(reply.value)
      }
    }
    return null
  })
}

const readFocusConfig = (): Effect.Effect<{
  readonly signature: Option.Option<string>
  readonly runtimeDir: Option.Option<string>
}> =>
  Effect.gen(function* config() {
    const signature = yield* Config.option(
      Config.NonEmptyString("HYPRLAND_INSTANCE_SIGNATURE"),
    ).pipe(Effect.orElseSucceed(() => Option.none<string>()))
    const runtimeDir = yield* Config.option(Config.NonEmptyString("XDG_RUNTIME_DIR")).pipe(
      Effect.orElseSucceed(() => Option.none<string>()),
    )
    return { signature, runtimeDir }
  })

export { focusedMonitor, readFocusConfig }
