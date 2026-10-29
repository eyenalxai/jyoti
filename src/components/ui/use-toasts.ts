import { createSignal } from "solid-js"

import type { Notice } from "@/components/ui/toast"

const useToasts = () => {
  const [toasts, setToasts] = createSignal<readonly Notice[]>([])

  const notify = (notice: Notice) => {
    setToasts((previous) => [...previous, notice])
  }

  const dismiss = () => {
    setToasts((previous) => previous.slice(1))
  }

  return { toasts, notify, dismiss }
}

export { useToasts }
