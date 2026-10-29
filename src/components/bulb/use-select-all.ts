import type { ControlFocus } from "@/components/bulb/controls"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { Notice } from "@/components/ui/toast"

interface UseSelectAllOptions {
  readonly model: BulbsModel
  readonly setFocus: (focus: ControlFocus) => void
  readonly notify: (notice: Notice) => void
}

const useSelectAll =
  (options: UseSelectAllOptions): (() => void) =>
  () => {
    if (!options.model.canSelectAll()) {
      options.notify({
        kind: "info",
        message:
          options.model.bulbs().length === 0
            ? "Add a bulb first."
            : "Add a second bulb to control all of them.",
      })
      return
    }
    options.model.selectAll()
    options.setFocus("power")
  }

export { useSelectAll }
