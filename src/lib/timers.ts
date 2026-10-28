import { Effect, Fiber } from "effect"

type Timer = Fiber.Fiber<void>

const after = (millis: number, action: () => void): Timer =>
  Effect.runFork(Effect.sleep(millis).pipe(Effect.flatMap(() => Effect.sync(action))))

const cancel = (timer: Timer): void => {
  Effect.runFork(Fiber.interrupt(timer))
}

export { after, cancel, type Timer }
