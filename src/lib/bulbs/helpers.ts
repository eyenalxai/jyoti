import { Array as Arr, Effect, Option, Schema } from "effect"

import type { Bulb, BulbId, FieldIssue } from "@/lib/bulbs/schema"

import { BulbInvalid, BulbNotFound } from "@/lib/bulbs/error"
import { BulbIdSchema } from "@/lib/bulbs/schema"

const findBulb = (bulbs: readonly Bulb[], id: BulbId): Effect.Effect<Bulb, BulbNotFound> => {
  const bulb = Option.getOrUndefined(Arr.findFirst(bulbs, (entry) => entry.id === id))
  return bulb === undefined
    ? Effect.fail(new BulbNotFound({ id, message: "That bulb is no longer in the list." }))
    : Effect.succeed(bulb)
}

const firstIssue = (issues: readonly FieldIssue[]): BulbInvalid => {
  const issue = issues[0]
  if (issue === undefined) {
    return new BulbInvalid({ field: "name", message: "The bulb details are incomplete." })
  }
  return new BulbInvalid({ field: issue.field, message: issue.message })
}

const newId = Effect.gen(function* newId() {
  const raw = yield* Effect.sync(() => Bun.randomUUIDv7())
  return yield* Schema.decodeUnknownEffect(BulbIdSchema)(raw).pipe(Effect.orDie)
})

const withModel = (
  base: { readonly id: BulbId; readonly name: string; readonly address: string },
  model: string,
): Bulb => (model.length > 0 ? { ...base, model } : base)

export { findBulb, firstIssue, newId, withModel }
