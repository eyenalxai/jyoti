import { BunDatagramSocket } from "@effect/platform-bun"
import { Context, Effect, Layer, Result, Schema } from "effect"
import { NetAddress } from "effect/net"
import { DatagramSocket } from "effect/socket"

import type { BulbEndpoint, MiioReply } from "@/lib/bulbs/schema"

import { MiioReplySchema } from "@/lib/bulbs/schema"
import { AppConfig } from "@/lib/config"
import { MiioError } from "@/lib/miio/error"
import {
  buildHello,
  buildRequest,
  messageIdFromStamp,
  parseHelloReply,
  parsePacket,
  tokenBytes,
} from "@/lib/miio/packet"

interface MiioTransportShape {
  readonly transact: (
    endpoint: BulbEndpoint,
    method: string,
    params: readonly unknown[],
  ) => Effect.Effect<MiioReply, MiioError>
}

const JsonUnknown = Schema.fromJsonString(Schema.Unknown)

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

class MiioTransport extends Context.Service<MiioTransport, MiioTransportShape>()(
  "jyoti/lib/miio/transport/MiioTransport",
) {
  static readonly layer = Layer.effect(
    MiioTransport,
    Effect.gen(function* layer() {
      const config = yield* AppConfig
      const socket = yield* DatagramSocket.DatagramSocket

      const transact = Effect.fn("MiioTransport.transact")(function* transact(
        endpoint: BulbEndpoint,
        method: string,
        params: readonly unknown[],
      ) {
        const target = NetAddress.inetAddressFromIpString(endpoint.address, config.miioPort)
        if (Result.isFailure(target)) {
          return yield* new MiioError({
            kind: "address",
            address: endpoint.address,
            message: `"${endpoint.address}" is not a valid IP address. Edit the bulb and enter its IP.`,
          })
        }

        yield* Effect.try({
          try: () => {
            tokenBytes(endpoint.token)
          },
          catch: (cause) =>
            new MiioError({
              kind: "token",
              address: endpoint.address,
              message:
                "The stored token is not 32 hexadecimal characters. Edit the bulb and enter its token again.",
              cause,
            }),
        })

        const destination = target.success

        const exchange = Effect.scoped(
          Effect.gen(function* exchange() {
            const reader = yield* socket.reader
            const writer = yield* socket.writer

            yield* writer.write({
              payload: buildHello(),
              address: destination,
            })
            const helloPackets = yield* reader.pull
            const hello = parseHelloReply(helloPackets[0].payload)

            yield* writer.write({
              payload: buildRequest({
                token: endpoint.token,
                deviceId: hello.deviceId,
                stamp: hello.stamp,
                messageId: messageIdFromStamp(hello.stamp),
                method,
                params,
              }),
              address: destination,
            })

            const replyPackets = yield* reader.pull
            const reply = parsePacket(endpoint.token, replyPackets[0].payload)
            const text = new TextDecoder().decode(reply.body)
            return yield* Schema.decodeUnknownEffect(JsonUnknown)(text)
          }),
        )

        const reply = yield* exchange.pipe(
          Effect.mapError((error) => toMiioError(error, endpoint.address)),
          Effect.timeout(config.miioTimeout),
          Effect.catchTag("TimeoutError", () =>
            Effect.fail(
              new MiioError({
                kind: "timeout",
                address: endpoint.address,
                message: `No reply from ${endpoint.address}. Check its power and network.`,
              }),
            ),
          ),
        )

        const decoded = yield* Schema.decodeUnknownEffect(MiioReplySchema)(reply).pipe(
          Effect.mapError(
            (cause) =>
              new MiioError({
                kind: "protocol",
                address: endpoint.address,
                message: `The bulb at ${endpoint.address} sent a reply jyoti could not read.`,
                cause,
              }),
          ),
        )

        if (decoded.error !== undefined) {
          return yield* new MiioError({
            kind: "device",
            address: endpoint.address,
            message: `The bulb at ${endpoint.address} refused the command: ${decoded.error.message} (${decoded.error.code}).`,
          })
        }

        return decoded
      })

      return MiioTransport.of({ transact })
    }),
  ).pipe(Layer.provide(BunDatagramSocket.layer()))
}

export { MiioTransport, type MiioTransportShape }
