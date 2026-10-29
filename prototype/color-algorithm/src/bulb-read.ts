import { Effect, Result } from "effect"

import { Bulbs } from "@/lib/bulbs/service"
import { dispose, run } from "@/lib/runtime"

const program = Effect.gen(function* () {
  const bulbs = yield* Bulbs
  const all = yield* bulbs.list()
  for (const bulb of all) {
    const state = yield* Effect.result(bulbs.readState(bulb.id))
    if (Result.isSuccess(state)) {
      const value = state.success
      console.log(
        `${bulb.name}: power=${String(value.power)} brightness=${String(value.brightness)} temperature=${String(value.temperature)} color=${String(value.color)} mode=${value.mode}`,
      )
    } else {
      console.log(`${bulb.name}: unreachable (${state.failure.message})`)
    }
  }
})

const result = await run(program)
await dispose()
if (!result.ok) {
  console.error(result.message)
  process.exitCode = 1
}
