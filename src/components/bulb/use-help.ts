import type { Accessor, Setter } from "solid-js"

import { createSignal } from "solid-js"

import type { HelpContext } from "@/components/bulb/help"
import type { Screen } from "@/components/bulb/modals"

const contextForScreen = (screen: Screen): HelpContext => {
  if (screen === "sync") {
    return "sync"
  }
  if (screen === "color") {
    return "color"
  }
  if (screen === "monitors") {
    return "monitors"
  }
  if (screen === "capture") {
    return "capture"
  }
  if (screen === "remove") {
    return "remove"
  }
  if (screen === "add" || screen === "edit") {
    return "form"
  }
  return "dashboard"
}

interface UseHelpOptions {
  readonly screen: Accessor<Screen>
  readonly setScreen: Setter<Screen>
}

interface HelpModel {
  readonly helpContext: Accessor<HelpContext>
  readonly open: () => void
  readonly close: () => boolean
}

const useHelp = (options: UseHelpOptions): HelpModel => {
  const [helpContext, setHelpContext] = createSignal<HelpContext>("dashboard")
  const [helpReturn, setHelpReturn] = createSignal<Screen>("dashboard")

  const open = () => {
    setHelpReturn(options.screen())
    setHelpContext(contextForScreen(options.screen()))
    options.setScreen("help")
  }

  const close = (): boolean => {
    if (options.screen() !== "help") {
      return false
    }
    options.setScreen(helpReturn())
    return true
  }

  return { helpContext, open, close }
}

export { useHelp, type HelpModel }
