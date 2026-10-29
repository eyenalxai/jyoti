import { Duration, Effect } from "effect"

import { Bulbs } from "@/lib/bulbs/service"
import { dispose, run } from "@/lib/runtime"

interface TestCase {
  readonly label: string
  readonly mode: "color" | "temperature"
  readonly value: string | number
}

const tests: readonly TestCase[] = [
  { label: "white document · 4000 K", mode: "temperature", value: 4000 },
  { label: "white document · 6500 K", mode: "temperature", value: 6500 },
  { label: "movie beach · dominant", mode: "color", value: "#7090c3" },
  { label: "movie fire · dominant", mode: "color", value: "#c87660" },
  { label: "movie fire · histogram", mode: "color", value: "#d26f57" },
  { label: "steps red · capped", mode: "color", value: "#d26f59" },
  { label: "steps red · scene colour", mode: "color", value: "#c0392b" },
  { label: "steps blue · capped", mode: "color", value: "#4195d9" },
]

const targetName = process.argv[2] ?? "Lightbulb 1"

const program = Effect.gen(function* () {
  const bulbs = yield* Bulbs
  const all = yield* bulbs.list()
  const bulb = all.find((entry) => entry.name.toLowerCase() === targetName.toLowerCase())
  if (bulb === undefined) {
    return yield* Effect.fail(
      new Error(`No bulb named ${targetName}. Found: ${all.map((entry) => entry.name).join(", ")}`),
    )
  }

  const before = yield* bulbs.readState(bulb.id)
  yield* Effect.sync(() => {
    console.log(`bulb: ${bulb.name}`)
    console.log(
      `before: power=${String(before.power)} brightness=${String(before.brightness)} temperature=${String(before.temperature)} color=${String(before.color)} mode=${before.mode}`,
    )
    console.log("test starts in 1 s; watch the bulb, not the terminal")
  })
  yield* Effect.sleep(Duration.millis(1000))

  const restore = Effect.gen(function* () {
    yield* bulbs.setBrightness(bulb.id, 100)
    yield* bulbs.setTemperature(bulb.id, 4000)
    if (before.power === false) {
      yield* bulbs.setPower(bulb.id, false)
    }
    yield* Effect.sync(() => console.log("restored: 100% / 4000 K"))
  }).pipe(
    Effect.catch((error) =>
      Effect.sync(() => {
        console.error(`restore failed: ${String(error)}`)
      }),
    ),
  )

  yield* Effect.gen(function* () {
    yield* bulbs.setPower(bulb.id, true)
    yield* bulbs.setBrightness(bulb.id, 30)

    for (let index = 0; index < tests.length; index += 1) {
      const test = tests[index] as TestCase
      yield* Effect.sync(() => console.log(`${index + 1}/${tests.length} ${test.label}`))
      if (test.mode === "temperature") {
        yield* bulbs.setTemperature(bulb.id, test.value as number)
      } else {
        yield* bulbs.setColor(bulb.id, test.value as string)
      }
      yield* Effect.sleep(Duration.millis(2500))
    }
  }).pipe(Effect.ensuring(restore))
})

const result = await run(program)
await dispose()
if (!result.ok) {
  console.error(result.message)
  process.exitCode = 1
}
