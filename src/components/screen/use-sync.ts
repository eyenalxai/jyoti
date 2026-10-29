import type { Accessor } from "solid-js"

import { Array as Arr, Effect, Option, Stream } from "effect"
import { createSignal, onCleanup, onMount } from "solid-js"

import type { Notice } from "@/components/ui/toast"
import type { BulbId } from "@/lib/bulbs/schema"
import type { Monitor } from "@/lib/screen/frame"
import type { RestoreReport } from "@/lib/screen/sync/restore"
import type { SyncStatus } from "@/lib/screen/sync/status"

import { cancelFiber, forkSyncEffect, runScreen } from "@/lib/runtime"
import { ScreenSyncEngine } from "@/lib/screen/sync"
import { stoppedMessage } from "@/lib/screen/sync/copy"
import { pickDefaultMonitor } from "@/lib/screen/sync/monitor"
import { idleSyncStatus } from "@/lib/screen/sync/status"

interface UseScreenSyncOptions {
  readonly onNotice: (notice: Notice) => void
  readonly onError: (message: string) => void
}

interface ScreenSyncModel {
  readonly status: Accessor<SyncStatus>
  readonly busy: Accessor<boolean>
  readonly monitors: Accessor<readonly Monitor[]>
  readonly monitorIndex: Accessor<number>
  readonly monitorBusy: Accessor<boolean>
  readonly toggle: () => void
  readonly togglePause: () => void
  readonly stop: () => Promise<RestoreReport | null>
  readonly clearRestore: () => void
  readonly notePowers: (powers: ReadonlyMap<BulbId, boolean | null>) => void
  readonly openMonitors: () => void
  readonly moveMonitor: (delta: number) => void
  readonly chooseMonitor: () => Promise<boolean>
}

const useScreenSyncModel = (options: UseScreenSyncOptions): ScreenSyncModel => {
  const [status, setStatus] = createSignal<SyncStatus>(idleSyncStatus)
  const [busy, setBusy] = createSignal(false)
  const [monitors, setMonitors] = createSignal<readonly Monitor[]>([])
  const [monitorIndex, setMonitorIndex] = createSignal(0)
  const [monitorBusy, setMonitorBusy] = createSignal(false)

  const selectedMonitorName = (): string | null => status().pendingMonitor ?? status().monitor

  const stop = (): Promise<RestoreReport | null> =>
    runScreen(ScreenSyncEngine.use((service) => service.stop())).then((result) => {
      if (!result.ok) {
        options.onError(result.message)
        return null
      }
      return result.value
    })

  const toggle = () => {
    const phase = status().phase
    if (phase === "stopping" || (busy() && phase !== "starting")) {
      return
    }
    if (phase !== "idle") {
      void stop().then((report) => {
        if (report !== null) {
          options.onNotice({
            kind: report.notRestored.length === 0 && report.total > 0 ? "success" : "info",
            message: stoppedMessage(report),
          })
        }
        return null
      })
      return
    }

    setBusy(true)
    void runScreen(ScreenSyncEngine.use((service) => service.start())).then((result) => {
      setBusy(false)
      if (!result.ok) {
        if (result.tag !== "cancelled") {
          options.onError(result.message)
        }
        return null
      }
      const count = result.value.bulbCount
      options.onNotice({
        kind: "success",
        message: `Screen sync on · ${count} ${count === 1 ? "bulb" : "bulbs"} · ${result.value.monitorName}`,
      })
      return null
    })
  }

  const clearRestore = () => {
    if (status().phase !== "idle") {
      return
    }
    void runScreen(ScreenSyncEngine.use((service) => service.clearRestore()))
  }

  const notePowers = (powers: ReadonlyMap<BulbId, boolean | null>) => {
    void runScreen(ScreenSyncEngine.use((service) => service.notePowers(powers))).then((result) => {
      if (!result.ok) {
        options.onError(result.message)
      }
      return null
    })
  }

  const togglePause = () => {
    if (busy()) {
      return
    }
    const phase = status().phase
    if (phase !== "syncing" && phase !== "paused") {
      return
    }

    setBusy(true)
    void runScreen(
      ScreenSyncEngine.use((service) => (phase === "syncing" ? service.pause() : service.resume())),
    ).then((result) => {
      setBusy(false)
      if (!result.ok) {
        options.onError(result.message)
        return null
      }
      options.onNotice({
        kind: "info",
        message: phase === "syncing" ? "Screen sync paused." : "Screen sync resumed.",
      })
      return null
    })
  }

  const chooseIndex = (listed: readonly Monitor[]): number => {
    const preferred = selectedMonitorName()
    if (preferred !== null) {
      const index = Option.getOrElse(
        Arr.findFirstIndex(listed, (monitor) => monitor.name === preferred),
        () => -1,
      )
      if (index >= 0) {
        return index
      }
    }
    const fallback = pickDefaultMonitor(listed)
    if (fallback === null) {
      return 0
    }
    return Math.max(
      0,
      Option.getOrElse(
        Arr.findFirstIndex(listed, (monitor) => monitor.name === fallback.name),
        () => 0,
      ),
    )
  }

  const openMonitors = () => {
    setMonitors([])
    setMonitorBusy(true)
    void runScreen(ScreenSyncEngine.use((service) => service.listMonitors())).then((result) => {
      setMonitorBusy(false)
      if (!result.ok) {
        options.onError(result.message)
        return null
      }
      setMonitors(result.value)
      setMonitorIndex(chooseIndex(result.value))
      return null
    })
  }

  const moveMonitor = (delta: number) => {
    setMonitorIndex((current) => {
      const count = monitors().length
      if (count === 0) {
        return 0
      }
      return (current + delta + count) % count
    })
  }

  const chooseMonitor = (): Promise<boolean> => {
    if (monitorBusy()) {
      return Promise.resolve(false)
    }
    const monitor = monitors()[monitorIndex()]
    if (monitor === undefined) {
      return Promise.resolve(false)
    }

    setMonitorBusy(true)
    return runScreen(ScreenSyncEngine.use((service) => service.selectMonitor(monitor.name))).then(
      (result) => {
        setMonitorBusy(false)
        if (!result.ok) {
          options.onError(result.message)
          return false
        }
        options.onNotice({
          kind: "info",
          message:
            status().phase === "idle"
              ? `Next run captures ${monitor.name}.`
              : `Switching to ${monitor.name}.`,
        })
        return true
      },
    )
  }

  onMount(() => {
    const fiber = forkSyncEffect(
      ScreenSyncEngine.use((service) =>
        service.status.pipe(
          Stream.runForEach((next) =>
            Effect.sync(() => {
              const previous = status()
              setStatus(next)
              const nextError = next.error ?? next.rejection
              const previousError = previous.error ?? previous.rejection
              if (nextError !== null && nextError !== previousError) {
                options.onError(nextError)
              }
            }),
          ),
        ),
      ),
    )
    onCleanup(() => {
      cancelFiber(fiber)
    })
  })

  return {
    status,
    busy,
    monitors,
    monitorIndex,
    monitorBusy,
    toggle,
    togglePause,
    stop,
    clearRestore,
    notePowers,
    openMonitors,
    moveMonitor,
    chooseMonitor,
  }
}

export { useScreenSyncModel, type ScreenSyncModel }
