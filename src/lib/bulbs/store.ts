import { Context, Effect, Layer, Schema } from "effect"
import { chmod, mkdir, rename } from "node:fs/promises"
import path from "node:path"

import type { Bulb } from "@/lib/bulbs/schema"

import { BulbFileSchema } from "@/lib/bulbs/schema"
import { AppConfig } from "@/lib/config"

class BulbStoreError extends Schema.TaggedError<BulbStoreError>()("BulbStoreError", {
  path: Schema.String,
  message: Schema.String,
  cause: Schema.Defect(),
}) {}

interface BulbStoreShape {
  readonly load: () => Effect.Effect<readonly Bulb[], BulbStoreError>
  readonly save: (bulbs: readonly Bulb[]) => Effect.Effect<void, BulbStoreError>
}

class BulbStore extends Context.Service<BulbStore, BulbStoreShape>()(
  "jyoti/lib/bulbs/store/BulbStore",
) {
  static readonly layer = Layer.effect(
    BulbStore,
    Effect.gen(function* layer() {
      const { configPath } = yield* AppConfig
      const file = BulbFileSchema

      const load = Effect.fn("BulbStore.load")(function* load() {
        const exists = yield* Effect.promise(() => Bun.file(configPath).exists())
        if (!exists) {
          return []
        }

        const text = yield* Effect.tryPromise({
          try: () => Bun.file(configPath).text(),
          catch: (cause) =>
            new BulbStoreError({
              path: configPath,
              message: `Could not read ${configPath}. Check its permissions and try again.`,
              cause,
            }),
        })

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
        const encoded = `${text}\n`
        const temporary = `${configPath}.tmp`

        yield* Effect.tryPromise({
          try: () => mkdir(path.dirname(configPath), { recursive: true }),
          catch: (cause) =>
            new BulbStoreError({
              path: configPath,
              message: `Could not create the folder for ${configPath}. Check its permissions.`,
              cause,
            }),
        })

        yield* Effect.tryPromise({
          try: () => Bun.write(temporary, encoded),
          catch: (cause) =>
            new BulbStoreError({
              path: configPath,
              message: `Could not write ${configPath}. Check its permissions.`,
              cause,
            }),
        })

        yield* Effect.tryPromise({
          try: () => chmod(temporary, 0o600),
          catch: (cause) =>
            new BulbStoreError({
              path: configPath,
              message: `Could not set permissions on ${configPath}.`,
              cause,
            }),
        })

        yield* Effect.tryPromise({
          try: () => rename(temporary, configPath),
          catch: (cause) =>
            new BulbStoreError({
              path: configPath,
              message: `Could not replace ${configPath}. Check its permissions.`,
              cause,
            }),
        })
      })

      return BulbStore.of({ load, save })
    }),
  )
}

export { BulbStore, BulbStoreError, type BulbStoreShape }
