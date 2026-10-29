import { Context, Effect, Layer, Schema } from "effect"
import { chmod, mkdir, rename } from "node:fs/promises"
import path from "node:path"

import { AppConfig } from "@/lib/config"
import { SYNC_GAIN_DEFAULT, normalizeSyncGain } from "@/lib/screen/sync/gain"

class PreferencesError extends Schema.TaggedError<PreferencesError>()("PreferencesError", {
  path: Schema.String,
  message: Schema.String,
  cause: Schema.Defect(),
}) {}

const PreferencesFileSchema = Schema.Struct({
  version: Schema.Literal(1),
  syncGain: Schema.Number,
})

interface PreferencesState {
  readonly syncGain: number
}

const defaultPreferences: PreferencesState = { syncGain: SYNC_GAIN_DEFAULT }

interface PreferencesShape {
  readonly load: () => Effect.Effect<PreferencesState, PreferencesError>
  readonly save: (preferences: PreferencesState) => Effect.Effect<void, PreferencesError>
}

class Preferences extends Context.Service<Preferences, PreferencesShape>()(
  "jyoti/lib/preferences",
) {
  static readonly layer = Layer.effect(
    Preferences,
    Effect.gen(function* layer() {
      const { preferencesPath } = yield* AppConfig

      const load = Effect.fn("Preferences.load")(function* load() {
        const exists = yield* Effect.promise(() => Bun.file(preferencesPath).exists())
        if (!exists) {
          return defaultPreferences
        }

        const text = yield* Effect.tryPromise({
          try: () => Bun.file(preferencesPath).text(),
          catch: (cause) =>
            new PreferencesError({
              path: preferencesPath,
              message: `Could not read ${preferencesPath}. Check its permissions and try again.`,
              cause,
            }),
        })

        const decoded = yield* Schema.decodeUnknownEffect(
          Schema.fromJsonString(PreferencesFileSchema),
        )(text).pipe(
          Effect.mapError(
            (cause) =>
              new PreferencesError({
                path: preferencesPath,
                message: `${preferencesPath} does not match the preferences format.`,
                cause,
              }),
          ),
        )

        return { syncGain: normalizeSyncGain(decoded.syncGain) }
      })

      const save = Effect.fn("Preferences.save")(function* save(preferences: PreferencesState) {
        const text = yield* Schema.encodeUnknownEffect(
          Schema.fromJsonString(PreferencesFileSchema),
        )({
          version: 1,
          syncGain: normalizeSyncGain(preferences.syncGain),
        }).pipe(
          Effect.mapError(
            (cause) =>
              new PreferencesError({
                path: preferencesPath,
                message: `Could not encode the preferences for ${preferencesPath}.`,
                cause,
              }),
          ),
        )
        const encoded = `${text}\n`
        const temporary = `${preferencesPath}.tmp`

        yield* Effect.tryPromise({
          try: () => mkdir(path.dirname(preferencesPath), { recursive: true }),
          catch: (cause) =>
            new PreferencesError({
              path: preferencesPath,
              message: `Could not create the folder for ${preferencesPath}. Check its permissions.`,
              cause,
            }),
        })

        yield* Effect.tryPromise({
          try: () => Bun.write(temporary, encoded),
          catch: (cause) =>
            new PreferencesError({
              path: preferencesPath,
              message: `Could not write ${preferencesPath}. Check its permissions.`,
              cause,
            }),
        })

        yield* Effect.tryPromise({
          try: () => chmod(temporary, 0o600),
          catch: (cause) =>
            new PreferencesError({
              path: preferencesPath,
              message: `Could not set permissions on ${preferencesPath}.`,
              cause,
            }),
        })

        yield* Effect.tryPromise({
          try: () => rename(temporary, preferencesPath),
          catch: (cause) =>
            new PreferencesError({
              path: preferencesPath,
              message: `Could not replace ${preferencesPath}. Check its permissions.`,
              cause,
            }),
        })
      })

      return Preferences.of({ load, save })
    }),
  )
}

export { Preferences, PreferencesError, defaultPreferences }
export type { PreferencesShape }
