import type { Accessor } from "solid-js"

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
  readonly shutdown: () => Promise<boolean>
}

const useScreenSyncModel = (options: UseScreenSyncOptions): ScreenSyncModel => {
  const [running, setRunning] = createSignal(false)
  const [busy, setBusy] = createSignal(false)

  const stopRun = (): Promise<boolean> =>
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

    if (running()) {
      setBusy(true)
      void stopRun().then((stopped) => {
        setBusy(false)
        if (stopped) {
          options.onNotice({ kind: "info", message: "Screen sync stopped." })
        }
        return stopped
      })
      return
    }

    setBusy(true)
    void runScreen(ScreenSyncEngine.use((service) => service.start())).then((result) => {
      setBusy(false)
      if (!result.ok) {
        options.onError(result.message)
        return false
      }
      setRunning(true)
      options.onNotice({
        kind: "success",
        message: `Screen sync on · ${result.value.bulbName} · ${result.value.monitorName}`,
      })
      return true
    })
  }

  const shutdown = (): Promise<boolean> => stopRun()

  return { running, busy, toggle, shutdown }
}

export { useScreenSyncModel, type ScreenSyncModel }
