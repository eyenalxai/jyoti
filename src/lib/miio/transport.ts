import { BunDatagramSocket } from "@effect/platform-bun"
import { Context, Effect, Layer, Result } from "effect"
import { NetAddress } from "effect/net"
import { DatagramSocket } from "effect/socket"

import type { BulbEndpoint, MiioReply } from "@/lib/bulbs/schema"

import { AppConfig } from "@/lib/config"
import { MiioError, toMiioError } from "@/lib/miio/error"
import {
  buildHello,
  buildRequest,
  messageIdFromStamp,
  parseHelloReply,
  parsePacket,
  tokenBytes,
} from "@/lib/miio/packet"
import { replyError } from "@/lib/miio/properties"
import { decodeReply } from "@/lib/miio/reply"

interface MiioTransportShape {
  readonly transact: (
    endpoint: BulbEndpoint,
    method: string,
    params: readonly unknown[],
  ) => Effect.Effect<MiioReply, MiioError>
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
            return yield* decodeReply(reply.body, endpoint.address)
          }),
        )

        const reply = yield* exchange.pipe(
          Effect.mapError((error) => toMiioError(error, endpoint.address)),
          Effect.timeoutOrElse({
            duration: config.miioTimeout,
            orElse: () =>
              Effect.fail(
                new MiioError({
                  kind: "timeout",
                  address: endpoint.address,
                  message: `No reply from ${endpoint.address}. Check its power and network.`,
                }),
              ),
          }),
        )

        const failure = replyError(reply, endpoint.address)
        if (failure !== null) {
          return yield* failure
        }

        return reply
      })

      return MiioTransport.of({ transact })
    }),
  ).pipe(Layer.provide(BunDatagramSocket.layer()))
}

export { MiioTransport, type MiioTransportShape }
