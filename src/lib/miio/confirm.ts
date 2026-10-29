import { Effect, Result } from "effect"

import type { MiioError } from "@/lib/miio/error"

const CONFIRMED_ATTEMPTS = 3

const confirmSend = <A>(
  attempt: () => Effect.Effect<A, MiioError>,
  fallbackError: () => MiioError,
): Effect.Effect<A, MiioError> =>
  Effect.fn("Miio.confirmSend")(function* confirmSendEffect() {
    let lastError = fallbackError()
    for (let count = 1; count <= CONFIRMED_ATTEMPTS; count += 1) {
      const outcome = yield* attempt().pipe(Effect.result)
      if (Result.isSuccess(outcome)) {
        return outcome.success
      }
      if (outcome.failure.kind === "device") {
        return yield* outcome.failure
      }
      lastError = outcome.failure
    }
    return yield* lastError
  })()

export { confirmSend }
