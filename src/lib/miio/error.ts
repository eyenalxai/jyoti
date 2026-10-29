import { Schema } from "effect"
import { DatagramSocket } from "effect/socket"

class MiioError extends Schema.TaggedError<MiioError>()("MiioError", {
  kind: Schema.Literals(["address", "token", "timeout", "network", "protocol", "device", "value"]),
  address: Schema.String,
  message: Schema.String,
  cause: Schema.optionalKey(Schema.Defect()),
}) {}

const toMiioError = (error: unknown, address: string): MiioError => {
  if (Schema.is(MiioError)(error)) {
    return error
  }
  if (DatagramSocket.isDatagramSocketError(error)) {
    return new MiioError({
      kind: "network",
      address,
      message: `Could not talk to the bulb at ${address}. Check it is powered and on this network.`,
      cause: error,
    })
  }
  return new MiioError({
    kind: "protocol",
    address,
    message: `The bulb at ${address} sent a reply jyoti could not read.`,
    cause: error,
  })
}

export { MiioError, toMiioError }
