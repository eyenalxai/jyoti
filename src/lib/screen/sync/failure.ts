import type { BulbsError } from "@/lib/bulbs/error"
import type { ScreenCaptureError } from "@/lib/screen/error"

import { ScreenSyncError } from "@/lib/screen/error"

const captureFailure = (error: ScreenCaptureError): ScreenSyncError =>
  new ScreenSyncError({ kind: error.kind, message: error.message, cause: error })

const bulbFailure = (error: BulbsError): ScreenSyncError =>
  new ScreenSyncError({ kind: "bulb", message: error.message, cause: error })

export { bulbFailure, captureFailure }
