import type { BulbState } from "@/lib/bulbs/schema"

const removeFromMap = <Value>(
  previous: ReadonlyMap<string, Value>,
  id: string,
): ReadonlyMap<string, Value> => {
  if (!previous.has(id)) {
    return previous
  }
  const next = new Map(previous)
  next.delete(id)
  return next
}

const patchStateMap = (
  previous: ReadonlyMap<string, BulbState>,
  id: string,
  patch: Partial<BulbState>,
): ReadonlyMap<string, BulbState> => {
  const current = previous.get(id)
  if (current === undefined) {
    return previous
  }
  const next: BulbState = {
    power: patch.power === undefined ? current.power : patch.power,
    brightness: patch.brightness === undefined ? current.brightness : patch.brightness,
    temperature: patch.temperature === undefined ? current.temperature : patch.temperature,
    color: patch.color === undefined ? current.color : patch.color,
    mode: patch.mode ?? current.mode,
  }
  return new Map(previous).set(id, next)
}

export { patchStateMap, removeFromMap }
