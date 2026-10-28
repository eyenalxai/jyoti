import { Schema } from "effect"

class MiioError extends Schema.TaggedError<MiioError>()("MiioError", {
  kind: Schema.Literals(["address", "token", "timeout", "network", "protocol", "device", "value"]),
  address: Schema.String,
  message: Schema.String,
  cause: Schema.optionalKey(Schema.Defect()),
}) {}

export { MiioError }
