import { render } from "@opentui/solid"

import { App } from "@/components/app"

await render(() => <App />, { exitOnCtrlC: false })
