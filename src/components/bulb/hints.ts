import type { Pane } from "@/components/bulb/pane"
import type { Hint } from "@/components/ui/key"

interface FooterInput {
  readonly pane: Pane
  readonly hasBulbs: boolean
  readonly canSelectAll: boolean
  readonly error: boolean
  readonly syncActive: boolean
}

const footerHints = (input: FooterInput): readonly Hint[] => {
  if (!input.hasBulbs) {
    return [
      { keys: "a", label: "add" },
      { keys: "?", label: "keys" },
      { keys: "q", label: "quit" },
    ]
  }

  const hints: Hint[] = []
  if (input.pane === "bulbs") {
    hints.push({ keys: "↑↓", label: "select" }, { keys: "tab", label: "controls" })
  } else {
    hints.push({ keys: "tab", label: "bulbs" })
  }
  if (input.canSelectAll) {
    hints.push({ keys: "A", label: "all" })
  }
  if (input.error) {
    hints.push({ keys: "r", label: "retry" })
  }
  if (!input.syncActive) {
    hints.push({ keys: "w", label: "white" }, { keys: "c", label: "colour" })
  }
  if (!input.syncActive) {
    hints.push(
      { keys: "a", label: "add" },
      { keys: "e", label: "edit" },
      { keys: "d", label: "remove" },
    )
  }
  hints.push(
    { keys: "r", label: "refresh" },
    { keys: "s", label: input.syncActive ? "stop" : "sync" },
  )
  if (input.syncActive) {
    hints.push({ keys: "x", label: "pause" })
  }
  hints.push({ keys: "?", label: "keys" }, { keys: "q", label: "quit" })
  return hints
}

export { footerHints, type FooterInput }
