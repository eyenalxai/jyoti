import type { Accessor } from "solid-js"

import { Effect } from "effect"
import { createSignal } from "solid-js"

import type { Notice } from "@/components/ui/status"

import { runScreen } from "@/lib/runtime"
import { ScreenSyncEngine } from "@/lib/screen/sync"

interface UseScreenSyncOptions {
  readonly onNotice: (notice: Notice) => void
  readonly onError: (message: string) => void
}

interface ScreenSyncModel {
  readonly running: Accessor<boolean>
  readonly busy: Accessor<boolean>
  readonly toggle: () => void
  readonly stop: () => Promise<boolean>
}

const useScreenSyncModel = (options: UseScreenSyncOptions): ScreenSyncModel => {
  const [running, setRunning] = createSignal(false)
  const [busy, setBusy] = createSignal(false)

  const stop = (): Promise<boolean> =>
    runScreen(ScreenSyncEngine.use((service) => service.stop())).then((result) => {
      if (!result.ok) {
        options.onError(result.message)
        return false
      }
      setRunning(false)
      return true
    })

  const toggle = () => {
    if (busy()) {
      return
    }

    setBusy(true)
    void runScreen(
      ScreenSyncEngine.use((service) =>
        Effect.gen(function* toggleSync() {
          if (yield* service.running()) {
            yield* service.stop()
            return { running: false as const }
          }
          return { running: true as const, info: yield* service.start() }
        }),
      ),
    ).then((result) => {
      setBusy(false)
      if (!result.ok) {
        options.onError(result.message)
        return false
      }
      if (result.value.running) {
        setRunning(true)
        options.onNotice({
          kind: "success",
          message: `Screen sync on · ${result.value.info.bulbName} · ${result.value.info.monitorName}`,
        })
      } else {
        setRunning(false)
        options.onNotice({ kind: "info", message: "Screen sync stopped." })
      }
      return true
    })
  }

  return { running, busy, toggle, stop }
}

export { useScreenSyncModel, type ScreenSyncModel }
