import type { Accessor } from "solid-js"

import { Array as Arr, Effect, Option, Result } from "effect"
import { createMemo, createSignal, onMount } from "solid-js"

import type { Bulb, BulbDraft, BulbId, BulbPatch, BulbState } from "@/lib/bulbs/schema"

import { Bulbs } from "@/lib/bulbs/service"
import { run } from "@/lib/runtime"

interface UseBulbsOptions {
  readonly onError: (message: string) => void
}

interface BulbsModel {
  readonly bulbs: Accessor<readonly Bulb[]>
  readonly states: Accessor<ReadonlyMap<string, BulbState>>
  readonly stateErrors: Accessor<ReadonlyMap<string, string>>
  readonly selectedId: Accessor<BulbId | null>
  readonly selected: Accessor<Bulb | null>
  readonly selectedState: Accessor<BulbState | undefined>
  readonly selectedError: Accessor<string | undefined>
  readonly loading: Accessor<boolean>
  readonly refreshing: Accessor<boolean>
  readonly select: (id: BulbId) => void
  readonly move: (delta: number) => void
  readonly refresh: () => void
  readonly add: (draft: BulbDraft) => Promise<string | null>
  readonly update: (id: BulbId, patch: BulbPatch) => Promise<string | null>
  readonly remove: (id: BulbId) => Promise<string | null>
  readonly patchState: (id: string, patch: Partial<BulbState>) => void
  readonly readOne: (id: BulbId) => Promise<string | null>
}

const useBulbs = (options: UseBulbsOptions): BulbsModel => {
  const [bulbs, setBulbs] = createSignal<readonly Bulb[]>([])
  const [states, setStates] = createSignal<ReadonlyMap<string, BulbState>>(new Map())
  const [stateErrors, setStateErrors] = createSignal<ReadonlyMap<string, string>>(new Map())
  const [selectedId, setSelectedId] = createSignal<BulbId | null>(null)
  const [loading, setLoading] = createSignal(true)
  const [refreshing, setRefreshing] = createSignal(false)
  let sequence = 0

  const selected = createMemo(() =>
    Option.getOrNull(Arr.findFirst(bulbs(), (bulb) => bulb.id === selectedId())),
  )
  const selectedState = createMemo(() => {
    const id = selectedId()
    return id === null ? undefined : states().get(id)
  })
  const selectedError = createMemo(() => {
    const id = selectedId()
    return id === null ? undefined : stateErrors().get(id)
  })

  const patchState = (id: string, patch: Partial<BulbState>) => {
    setStates((previous) => {
      const current = previous.get(id)
      if (current === undefined) {
        return previous
      }
      const next: BulbState = {
        power: patch.power ?? current.power,
        brightness: patch.brightness ?? current.brightness,
        temperature: patch.temperature ?? current.temperature,
        color: patch.color ?? current.color,
        mode: patch.mode ?? current.mode,
      }
      return new Map(previous).set(id, next)
    })
  }

  const clearState = (id: string) => {
    setStates((previous) => {
      if (!previous.has(id)) {
        return previous
      }
      const next = new Map(previous)
      next.delete(id)
      return next
    })
    setStateErrors((previous) => {
      if (!previous.has(id)) {
        return previous
      }
      const next = new Map(previous)
      next.delete(id)
      return next
    })
  }

  const readOne = (id: BulbId): Promise<string | null> =>
    run(Bulbs.use((service) => service.readState(id))).then((result) => {
      if (result.ok) {
        setStates((previous) => new Map(previous).set(id, result.value))
        setStateErrors((previous) => {
          if (!previous.has(id)) {
            return previous
          }
          const next = new Map(previous)
          next.delete(id)
          return next
        })
        return null
      }

      setStates((previous) => {
        if (!previous.has(id)) {
          return previous
        }
        const next = new Map(previous)
        next.delete(id)
        return next
      })
      setStateErrors((previous) => new Map(previous).set(id, result.message))
      return result.message
    })

  const refresh = () => {
    sequence += 1
    const token = sequence
    setRefreshing(true)
    void run(
      Bulbs.use((service) =>
        Effect.gen(function* refreshBulbs() {
          const list = yield* service.list()
          if (token !== sequence) {
            return
          }
          const previousId = selectedId()
          const kept = Option.getOrNull(Arr.findFirst(list, (bulb) => bulb.id === previousId))
          const nextId = kept?.id ?? list[0]?.id ?? null
          yield* Effect.sync(() => {
            setBulbs(list)
            setSelectedId(nextId)
            setLoading(false)
          })
          if (nextId === null) {
            yield* Effect.sync(() => {
              setStates(new Map())
              setStateErrors(new Map())
              setRefreshing(false)
            })
            return
          }

          const reads = yield* Effect.forEach(
            list,
            (bulb) =>
              service.readState(bulb.id).pipe(
                Effect.result,
                Effect.map((outcome) => [bulb.id, outcome] as const),
              ),
            { concurrency: 4 },
          )
          if (token !== sequence) {
            return
          }
          const nextStates = new Map<string, BulbState>()
          const nextErrors = new Map<string, string>()
          for (const [id, outcome] of reads) {
            if (Result.isSuccess(outcome)) {
              nextStates.set(id, outcome.success)
            } else {
              nextErrors.set(id, outcome.failure.message)
            }
          }
          yield* Effect.sync(() => {
            setStates(nextStates)
            setStateErrors(nextErrors)
            setRefreshing(false)
          })
        }),
      ),
    ).then((result) => {
      if (result.ok) {
        return null
      }
      setLoading(false)
      setRefreshing(false)
      options.onError(result.message)
      return null
    })
  }

  const select = (id: BulbId) => {
    setSelectedId(id)
    if (!states().has(id) && !stateErrors().has(id)) {
      void readOne(id)
    }
  }

  const move = (delta: number) => {
    const list = bulbs()
    if (list.length === 0) {
      return
    }
    const current = Option.getOrElse(
      Arr.findFirstIndex(list, (bulb) => bulb.id === selectedId()),
      () => -1,
    )
    const index = current === -1 ? 0 : Math.min(list.length - 1, Math.max(0, current + delta))
    const next = list[index]
    if (next !== undefined && next.id !== selectedId()) {
      select(next.id)
    }
  }

  const add = (draft: BulbDraft): Promise<string | null> =>
    run(Bulbs.use((service) => service.add(draft))).then((result) => {
      if (!result.ok) {
        return result.message
      }
      setBulbs((previous) => [...previous, result.value])
      setSelectedId(result.value.id)
      void readOne(result.value.id)
      return null
    })

  const update = (id: BulbId, patch: BulbPatch): Promise<string | null> =>
    run(Bulbs.use((service) => service.update(id, patch))).then((result) => {
      if (!result.ok) {
        return result.message
      }
      setBulbs((previous) => Arr.map(previous, (bulb) => (bulb.id === id ? result.value : bulb)))
      void readOne(id)
      return null
    })

  const remove = (id: BulbId): Promise<string | null> =>
    run(Bulbs.use((service) => service.remove(id))).then((result) => {
      if (!result.ok) {
        return result.message
      }
      const remaining = Arr.filter(bulbs(), (bulb) => bulb.id !== id)
      setBulbs(remaining)
      clearState(id)
      if (selectedId() === id) {
        const next = remaining[0]
        setSelectedId(next?.id ?? null)
        if (next !== undefined) {
          void readOne(next.id)
        }
      }
      return null
    })

  onMount(() => {
    refresh()
  })

  return {
    bulbs,
    states,
    stateErrors,
    selectedId,
    selected,
    selectedState,
    selectedError,
    loading,
    refreshing,
    select,
    move,
    refresh,
    add,
    update,
    remove,
    patchState,
    readOne,
  }
}

export { useBulbs, type BulbsModel }
