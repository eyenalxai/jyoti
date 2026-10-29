import { Effect, Schema } from "effect"
import { chmod, mkdir, rename } from "node:fs/promises"
import path from "node:path"

class JsonFileError extends Schema.TaggedError<JsonFileError>()("JsonFileError", {
  path: Schema.String,
  message: Schema.String,
  cause: Schema.Defect(),
}) {}

const readText = (filePath: string): Effect.Effect<string | null, JsonFileError> =>
  Effect.gen(function* readTextEffect() {
    const exists = yield* Effect.promise(() => Bun.file(filePath).exists())
    if (!exists) {
      return null
    }
    return yield* Effect.tryPromise({
      try: () => Bun.file(filePath).text(),
      catch: (cause) =>
        new JsonFileError({
          path: filePath,
          message: `Could not read ${filePath}. Check its permissions and try again.`,
          cause,
        }),
    })
  })

const writeText = (filePath: string, text: string): Effect.Effect<void, JsonFileError> =>
  Effect.gen(function* writeTextEffect() {
    const temporary = `${filePath}.tmp`

    yield* Effect.tryPromise({
      try: () => mkdir(path.dirname(filePath), { recursive: true }),
      catch: (cause) =>
        new JsonFileError({
          path: filePath,
          message: `Could not create the folder for ${filePath}. Check its permissions.`,
          cause,
        }),
    })

    yield* Effect.tryPromise({
      try: () => Bun.write(temporary, text),
      catch: (cause) =>
        new JsonFileError({
          path: filePath,
          message: `Could not write ${filePath}. Check its permissions.`,
          cause,
        }),
    })

    yield* Effect.tryPromise({
      try: () => chmod(temporary, 0o600),
      catch: (cause) =>
        new JsonFileError({
          path: filePath,
          message: `Could not set permissions on ${filePath}.`,
          cause,
        }),
    })

    yield* Effect.tryPromise({
      try: () => rename(temporary, filePath),
      catch: (cause) =>
        new JsonFileError({
          path: filePath,
          message: `Could not replace ${filePath}. Check its permissions.`,
          cause,
        }),
    })
  })

export { JsonFileError, readText, writeText }
