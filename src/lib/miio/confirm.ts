import { Effect, Result } from "effect"

import type { MiioError } from "@/lib/miio/error"

const CONFIRMED_ATTEMPTS = 3

const confirmSend = Effect.fn("Miio.confirmSend")(function* confirmSend(
  attempt: () => Effect.Effect<void, MiioError>,
  fallbackError: () => MiioError,
) {
  let lastError = fallbackError()
  for (let count = 1; count <= CONFIRMED_ATTEMPTS; count += 1) {
    const outcome = yield* attempt().pipe(Effect.result)
    if (Result.isSuccess(outcome)) {
      return yield* Effect.void
    }
    if (outcome.failure.kind === "device") {
      return yield* outcome.failure
    }
    lastError = outcome.failure
  }
  return yield* lastError
})

export { confirmSend }
