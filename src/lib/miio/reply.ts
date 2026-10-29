import { Effect, Schema } from "effect"

import type { MiioReply } from "@/lib/bulbs/schema"

import { MiioReplySchema } from "@/lib/bulbs/schema"
import { MiioError } from "@/lib/miio/error"

const JsonUnknown = Schema.fromJsonString(Schema.Unknown)

const decodeFailure = (address: string) => (cause: unknown) =>
  new MiioError({
    kind: "protocol",
    address,
    message: `The bulb at ${address} sent a reply jyoti could not read.`,
    cause,
  })

const decodeReply = (body: Uint8Array, address: string): Effect.Effect<MiioReply, MiioError> =>
  Effect.gen(function* decodeReplyBody() {
    const text = new TextDecoder().decode(body)
    const json = yield* Schema.decodeUnknownEffect(JsonUnknown)(text)
    return yield* Schema.decodeUnknownEffect(MiioReplySchema)(json)
  }).pipe(Effect.mapError(decodeFailure(address)))

export { decodeReply }
