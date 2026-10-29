import { useTerminalDimensions } from "@opentui/solid"
import { Show, createMemo } from "solid-js"

import type { CapturePreviewModel } from "@/components/screen/use-preview"
import type { ScreenSyncModel } from "@/components/screen/use-sync"
import type { Bulb, BulbDraft } from "@/lib/bulbs/schema"

import { Form } from "@/components/bulb/form"
import { Help } from "@/components/bulb/help"
import { Remove } from "@/components/bulb/remove"
import { Monitors } from "@/components/screen/monitors"
import { Preview } from "@/components/screen/preview"
import { Modal } from "@/components/ui/modal"

type Screen = "dashboard" | "add" | "edit" | "remove" | "color" | "help" | "capture" | "monitors"

interface ModalsProps {
  readonly screen: Screen
  readonly activeBulb: Bulb | null
  readonly busy: boolean
  readonly preview: CapturePreviewModel
  readonly syncModel: ScreenSyncModel
  readonly onSubmitAdd: (draft: BulbDraft) => void
  readonly onSubmitEdit: (draft: BulbDraft) => void
  readonly onConfirmRemove: () => void
}

const FORM_WIDTH = 58

const Modals = (props: ModalsProps) => {
  const dimensions = useTerminalDimensions()
  const formWidth = createMemo(() => Math.max(34, Math.min(FORM_WIDTH, dimensions().width - 2)))
  const inputWidth = createMemo(() => Math.max(12, formWidth() - 17))

  return (
    <>
      <Show when={props.screen === "add"}>
        <Modal title="Add bulb" hint="esc cancel" width={formWidth()} height={16}>
          <Form
            mode="add"
            initial={null}
            inputWidth={inputWidth()}
            busy={props.busy}
            onSubmit={props.onSubmitAdd}
          />
        </Modal>
      </Show>

      <Show when={props.screen === "edit" && props.activeBulb !== null}>
        <Modal title="Edit bulb" hint="esc cancel" width={formWidth()} height={16}>
          <Form
            mode="edit"
            initial={props.activeBulb}
            inputWidth={inputWidth()}
            busy={props.busy}
            onSubmit={props.onSubmitEdit}
          />
        </Modal>
      </Show>

      <Show when={props.screen === "remove"}>
        <Show when={props.activeBulb}>
          {(bulb) => (
            <Modal title="Remove bulb" hint="y or n" width={50} height={12}>
              <Remove bulb={bulb()} busy={props.busy} />
            </Modal>
          )}
        </Show>
      </Show>

      <Show when={props.screen === "help"}>
        <Modal title="Keys" hint="esc close" width={58} height={20}>
          <scrollbox flexGrow={1} minHeight={0} focused>
            <Help />
          </scrollbox>
        </Modal>
      </Show>

      <Show when={props.screen === "capture"}>
        <Modal title="Capture preview (dev)" hint="esc close" width={58} height={18}>
          <Preview model={props.preview} />
        </Modal>
      </Show>

      <Show when={props.screen === "monitors"}>
        <Modal title="Captured monitor" hint="esc cancel" width={64} height={14}>
          <Monitors
            monitors={props.syncModel.monitors()}
            selected={props.syncModel.monitorIndex()}
            captured={props.syncModel.status().monitor}
            pending={props.syncModel.status().pendingMonitor}
            busy={props.syncModel.monitorBusy()}
          />
        </Modal>
      </Show>
    </>
  )
}

export { Modals, type Screen }
