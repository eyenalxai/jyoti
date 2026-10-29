import { Context, Effect, Layer, Schema } from "effect"

import type { Bulb } from "@/lib/bulbs/schema"
import type { JsonFileError } from "@/lib/json-file"

import { BulbFileSchema } from "@/lib/bulbs/schema"
import { AppConfig } from "@/lib/config"
import { readText, writeText } from "@/lib/json-file"

class BulbStoreError extends Schema.TaggedError<BulbStoreError>()("BulbStoreError", {
  path: Schema.String,
  message: Schema.String,
  cause: Schema.Defect(),
}) {}

interface BulbStoreShape {
  readonly load: () => Effect.Effect<readonly Bulb[], BulbStoreError>
  readonly save: (bulbs: readonly Bulb[]) => Effect.Effect<void, BulbStoreError>
}

const storeError = (error: JsonFileError): BulbStoreError =>
  new BulbStoreError({ path: error.path, message: error.message, cause: error.cause })

class BulbStore extends Context.Service<BulbStore, BulbStoreShape>()(
  "jyoti/lib/bulbs/store/BulbStore",
) {
  static readonly layer = Layer.effect(
    BulbStore,
    Effect.gen(function* layer() {
      const { configPath } = yield* AppConfig
      const file = BulbFileSchema

      const load = Effect.fn("BulbStore.load")(function* load() {
        const text = yield* readText(configPath).pipe(Effect.mapError(storeError))
        if (text === null) {
          return []
        }

        const decoded = yield* Schema.decodeUnknownEffect(Schema.fromJsonString(file))(text).pipe(
          Effect.mapError(
            (cause) =>
              new BulbStoreError({
                path: configPath,
                message: `${configPath} does not match the bulb file format. Fix or remove it, then press r.`,
                cause,
              }),
          ),
        )

        const ids = new Set<string>()
        for (const bulb of decoded.bulbs) {
          if (ids.has(bulb.id)) {
            return yield* new BulbStoreError({
              path: configPath,
              message: `${configPath} has two bulbs with the same ID. Remove one entry and add it again.`,
              cause: new Error("duplicate bulb id"),
            })
          }
          ids.add(bulb.id)
        }

        return decoded.bulbs
      })

      const save = Effect.fn("BulbStore.save")(function* save(bulbs: readonly Bulb[]) {
        const text = yield* Schema.encodeUnknownEffect(Schema.fromJsonString(file))({
          version: 1,
          bulbs,
        }).pipe(
          Effect.mapError(
            (cause) =>
              new BulbStoreError({
                path: configPath,
                message: `Could not encode the bulb file for ${configPath}.`,
                cause,
              }),
          ),
        )
        yield* writeText(configPath, `${text}\n`).pipe(Effect.mapError(storeError))
      })

      return BulbStore.of({ load, save })
    }),
  )
}

export { BulbStore, BulbStoreError, type BulbStoreShape }
