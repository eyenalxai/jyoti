import { Schema } from "effect"

class ScreenCaptureError extends Schema.TaggedError<ScreenCaptureError>()("ScreenCaptureError", {
  kind: Schema.Literals(["library", "wayland", "monitor", "capture"]),
  message: Schema.String,
  cause: Schema.optionalKey(Schema.Defect()),
}) {}

class ScreenSyncError extends Schema.TaggedError<ScreenSyncError>()("ScreenSyncError", {
  kind: Schema.Literals([
    "already-running",
    "cancelled",
    "no-bulbs",
    "no-monitor",
    "not-ready",
    "not-running",
    "capture",
    "library",
    "wayland",
    "monitor",
    "bulb",
  ]),
  message: Schema.String,
  cause: Schema.optionalKey(Schema.Defect()),
}) {}

export { ScreenCaptureError, ScreenSyncError }
