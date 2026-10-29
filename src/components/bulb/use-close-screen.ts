import type { Accessor, Setter } from "solid-js"

import type { ColorSessionModel } from "@/components/bulb/color/use-session"
import type { Screen } from "@/components/bulb/modals"
import type { HelpModel } from "@/components/bulb/use-help"
import type { Bulb } from "@/lib/bulbs/schema"

interface UseCloseScreenOptions {
  readonly screen: Accessor<Screen>
  readonly busy: Accessor<boolean>
  readonly setScreen: Setter<Screen>
  readonly colorSession: ColorSessionModel
  readonly setActiveBulb: Setter<Bulb | null>
  readonly help: HelpModel
}

const useCloseScreen =
  (options: UseCloseScreenOptions): (() => void) =>
  () => {
    if (options.busy()) {
      return
    }
    if (options.help.close()) {
      return
    }
    if (options.screen() === "monitors") {
      options.setScreen("sync")
      return
    }
    if (options.screen() === "color") {
      if (options.colorSession.dirty()) {
        options.colorSession.revert()
      }
      options.colorSession.reset()
    }
    options.setScreen("dashboard")
    options.setActiveBulb(null)
  }

export { useCloseScreen }
