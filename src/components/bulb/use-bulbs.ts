import type { Accessor } from "solid-js"

import { Array as Arr, Effect, Option, Result } from "effect"
import { createMemo, createSignal, onMount } from "solid-js"

import type { Bulb, BulbDraft, BulbId, BulbPatch, BulbState } from "@/lib/bulbs/schema"
import type { BulbSelection } from "@/lib/bulbs/selection"

import { allSelection, bulbSelection, moveSelection, resolveSelection } from "@/lib/bulbs/selection"
import { Bulbs } from "@/lib/bulbs/service"
import { run } from "@/lib/runtime"

interface UseBulbsOptions {
  readonly onError: (message: string) => void
}

interface BulbsModel {
  readonly bulbs: Accessor<readonly Bulb[]>
  readonly states: Accessor<ReadonlyMap<string, BulbState>>
  readonly stateErrors: Accessor<ReadonlyMap<string, string>>
  readonly selection: Accessor<BulbSelection | null>
  readonly selectedId: Accessor<BulbId | null>
  readonly selected: Accessor<Bulb | null>
  readonly selectedState: Accessor<BulbState | undefined>
  readonly selectedError: Accessor<string | undefined>
  readonly canSelectAll: Accessor<boolean>
  readonly allSelected: Accessor<boolean>
  readonly loading: Accessor<boolean>
  readonly refreshing: Accessor<boolean>
  readonly select: (id: BulbId) => void
  readonly selectAll: () => void
  readonly move: (delta: number) => void
  readonly refresh: () => void
  readonly add: (draft: BulbDraft) => Promise<string | null>
  readonly update: (id: BulbId, patch: BulbPatch) => Promise<string | null>
  readonly remove: (id: BulbId) => Promise<string | null>
  readonly patchState: (id: BulbId, patch: Partial<BulbState>) => void
  readonly readOne: (id: BulbId) => Promise<string | null>
}

const useBulbs = (options: UseBulbsOptions): BulbsModel => {
  const [bulbs, setBulbs] = createSignal<readonly Bulb[]>([])
  const [states, setStates] = createSignal<ReadonlyMap<string, BulbState>>(new Map())
  const [stateErrors, setStateErrors] = createSignal<ReadonlyMap<string, string>>(new Map())
  const [selection, setSelection] = createSignal<BulbSelection | null>(null)
  const [loading, setLoading] = createSignal(true)
  const [refreshing, setRefreshing] = createSignal(false)
  let sequence = 0

  const selectedId = createMemo(() => {
    const current = selection()
    return current?.kind === "bulb" ? current.id : null
  })
  const selected = createMemo(() => {
    const id = selectedId()
    return id === null ? null : Option.getOrNull(Arr.findFirst(bulbs(), (bulb) => bulb.id === id))
  })
  const selectedState = createMemo(() => {
    const id = selectedId()
    return id === null ? undefined : states().get(id)
  })
  const selectedError = createMemo(() => {
    const id = selectedId()
    return id === null ? undefined : stateErrors().get(id)
  })
  const canSelectAll = createMemo(() => bulbs().length >= 2)
  const allSelected = createMemo(() => selection()?.kind === "all")

  const patchState = (id: BulbId, patch: Partial<BulbState>) => {
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
          const previous = selection()
          const next = resolveSelection(list, previous)
          yield* Effect.sync(() => {
            setBulbs(list)
            setSelection(next)
            setLoading(false)
          })
          if (next === null) {
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
    setSelection(bulbSelection(id))
    if (!states().has(id) && !stateErrors().has(id)) {
      void readOne(id)
    }
  }

  const selectAll = () => {
    if (canSelectAll()) {
      setSelection(allSelection)
    }
  }

  const move = (delta: number) => {
    const next = moveSelection(bulbs(), selection(), delta)
    if (next === null) {
      return
    }
    if (next.kind === "all") {
      setSelection(next)
      return
    }
    if (next.id !== selectedId()) {
      select(next.id)
    }
  }

  const add = (draft: BulbDraft): Promise<string | null> =>
    run(Bulbs.use((service) => service.add(draft))).then((result) => {
      if (!result.ok) {
        return result.message
      }
      setBulbs((previous) => [...previous, result.value])
      setSelection(bulbSelection(result.value.id))
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
      const current = selection()
      const fallback = remaining[0]
      const shouldFall = current?.kind === "all" ? remaining.length < 2 : selectedId() === id
      if (shouldFall) {
        setSelection(fallback === undefined ? null : bulbSelection(fallback.id))
        if (fallback !== undefined) {
          void readOne(fallback.id)
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
    selection,
    selectedId,
    selected,
    selectedState,
    selectedError,
    canSelectAll,
    allSelected,
    loading,
    refreshing,
    select,
    selectAll,
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
