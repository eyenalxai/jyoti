import { Context, Effect, Layer, Schema } from "effect"

import type { JsonFileError } from "@/lib/json-file"

import { AppConfig } from "@/lib/config"
import { readText, writeText } from "@/lib/json-file"
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

type PreferencesFile = typeof PreferencesFileSchema.Type

interface PreferencesState {
  readonly syncGain: number
}

const defaultPreferences: PreferencesState = { syncGain: SYNC_GAIN_DEFAULT }

interface PreferencesShape {
  readonly load: () => Effect.Effect<PreferencesState>
  readonly save: (preferences: PreferencesState) => Effect.Effect<void, PreferencesError>
}

const preferencesError = (error: JsonFileError): PreferencesError =>
  new PreferencesError({ path: error.path, message: error.message, cause: error.cause })

class Preferences extends Context.Service<Preferences, PreferencesShape>()(
  "jyoti/lib/preferences",
) {
  static readonly layer = Layer.effect(
    Preferences,
    Effect.gen(function* layer() {
      const { preferencesPath } = yield* AppConfig

      // A preference file must never block the app; a missing or broken one falls back to the default.
      const load = Effect.fn("Preferences.load")(function* load() {
        const text = yield* readText(preferencesPath).pipe(Effect.catch(() => Effect.succeed(null)))
        if (text === null) {
          return defaultPreferences
        }

        const decoded = yield* Schema.decodeUnknownEffect(
          Schema.fromJsonString(PreferencesFileSchema),
        )(text).pipe(Effect.catch(() => Effect.succeed(null)))
        if (decoded === null) {
          return defaultPreferences
        }

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
        yield* writeText(preferencesPath, `${text}\n`).pipe(Effect.mapError(preferencesError))
      })

      return Preferences.of({ load, save })
    }),
  )
}

export { Preferences, PreferencesError, PreferencesFileSchema, defaultPreferences }
export type { PreferencesFile, PreferencesShape, PreferencesState }
