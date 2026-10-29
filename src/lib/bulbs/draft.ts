import { Effect, Schema } from "effect"

import type { Bulb, BulbId, FieldIssue } from "@/lib/bulbs/schema"

import { BulbInvalid } from "@/lib/bulbs/error"
import { BulbIdSchema } from "@/lib/bulbs/schema"

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

export { firstIssue, newId, withModel }
