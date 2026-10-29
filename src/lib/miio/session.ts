import type { Scope } from "effect"

import { BunDatagramSocket } from "@effect/platform-bun"
import { Duration, Effect, Result } from "effect"
import { NetAddress } from "effect/net"

import type { BulbEndpoint, MiioReply } from "@/lib/bulbs/schema"
import type { HelloReply, ParsedPacket } from "@/lib/miio/packet"
import type { PropertyParam } from "@/lib/miio/properties"

import { MiioError, toMiioError } from "@/lib/miio/error"
import {
  buildHello,
  buildRequest,
  parseHelloReply,
  parsePacket,
  tokenBytes,
} from "@/lib/miio/packet"
import { commandError } from "@/lib/miio/properties"
import { decodeReply } from "@/lib/miio/reply"

const HELLO_TIMEOUT = Duration.millis(1000)
const STREAM_TIMEOUT = Duration.millis(600)
const REHELLO_AFTER_MS = 120_000
const MAX_MESSAGE_ID = 9999

interface MiioSession {
  readonly hello: () => Effect.Effect<void, MiioError>
  readonly setProperties: (params: readonly PropertyParam[]) => Effect.Effect<void, MiioError>
}

interface SessionState {
  deviceId: Uint8Array | null
  stamp: Uint8Array | null
  lastPacketAt: number | null
  messageId: number
}

interface IncomingReply {
  readonly packet: ParsedPacket
  readonly reply: MiioReply
}

const openMiioSession = (
  endpoint: BulbEndpoint,
  port: number,
): Effect.Effect<MiioSession, MiioError, Scope.Scope> =>
  Effect.gen(function* openSession() {
    const target = NetAddress.inetAddressFromIpString(endpoint.address, port)
    if (Result.isFailure(target)) {
      return yield* new MiioError({
        kind: "address",
        address: endpoint.address,
        message: `"${endpoint.address}" is not a valid IP address. Edit the bulb and enter its IP.`,
      })
    }

    yield* Effect.try({
      try: () => tokenBytes(endpoint.token),
      catch: (cause) =>
        new MiioError({
          kind: "token",
          address: endpoint.address,
          message:
            "The stored token is not 32 hexadecimal characters. Edit the bulb and enter its token again.",
          cause,
        }),
    })

    const socket = yield* BunDatagramSocket.make()
    const reader = yield* socket.reader.pipe(
      Effect.mapError((cause) => toMiioError(cause, endpoint.address)),
    )
    const writer = yield* socket.writer
    const destination = target.success
    const state: SessionState = {
      deviceId: null,
      stamp: null,
      lastPacketAt: null,
      messageId: 0,
    }

    const nextMessageId = (): number => {
      state.messageId = (state.messageId % MAX_MESSAGE_ID) + 1
      return state.messageId
    }

    const send = (payload: Uint8Array): Effect.Effect<void, MiioError> =>
      writer
        .write({ payload, address: destination })
        .pipe(Effect.mapError((cause) => toMiioError(cause, endpoint.address)))

    const pull = reader.pull.pipe(Effect.mapError((cause) => toMiioError(cause, endpoint.address)))

    const timeout = (kind: "hello" | "stream"): MiioError =>
      new MiioError({
        kind: "timeout",
        address: endpoint.address,
        message:
          kind === "hello"
            ? `No reply from ${endpoint.address} to the miIO hello. Check its power and network.`
            : `No reply from ${endpoint.address} within 600 ms.`,
      })

    const within = <A>(
      effect: Effect.Effect<A, MiioError>,
      duration: Duration.Duration,
      kind: "hello" | "stream",
    ): Effect.Effect<A, MiioError> =>
      effect.pipe(
        Effect.timeoutOrElse({
          duration,
          orElse: () => Effect.fail(timeout(kind)),
        }),
      )

    const nextHelloReply = (): Effect.Effect<HelloReply, MiioError> =>
      Effect.gen(function* readHello() {
        const packets = yield* pull
        for (const packet of packets) {
          return yield* Effect.try({
            try: () => parseHelloReply(packet.payload),
            catch: (cause) =>
              new MiioError({
                kind: "protocol",
                address: endpoint.address,
                message: `The bulb at ${endpoint.address} sent a hello reply jyoti could not read.`,
                cause,
              }),
          })
        }
        return yield* nextHelloReply()
      })

    const hello = Effect.fn("MiioSession.hello")(function* hello() {
      yield* send(buildHello())
      const reply = yield* within(nextHelloReply(), HELLO_TIMEOUT, "hello")
      state.deviceId = reply.deviceId
      state.stamp = reply.stamp
      state.lastPacketAt = performance.now()
    })

    const nextReply = (messageId: number): Effect.Effect<IncomingReply, MiioError> =>
      Effect.gen(function* readReply() {
        const packets = yield* pull
        for (const packet of packets) {
          const parsed = yield* Effect.try({
            try: () => parsePacket(endpoint.token, packet.payload),
            catch: (cause) =>
              new MiioError({
                kind: "protocol",
                address: endpoint.address,
                message: `The bulb at ${endpoint.address} sent a reply jyoti could not read.`,
                cause,
              }),
          })
          state.lastPacketAt = performance.now()
          const reply = yield* decodeReply(parsed.body, endpoint.address)
          if (reply.id === undefined || reply.id === messageId) {
            return { packet: parsed, reply }
          }
        }
        return yield* nextReply(messageId)
      })

    const exchange = (
      messageId: number,
      method: string,
      params: readonly unknown[],
    ): Effect.Effect<MiioReply, MiioError> => {
      const deviceId = state.deviceId
      const stamp = state.stamp
      if (deviceId === null || stamp === null) {
        return Effect.fail(
          new MiioError({
            kind: "protocol",
            address: endpoint.address,
            message: `The session to ${endpoint.address} has no device ID. Reconnect and try again.`,
          }),
        )
      }

      return Effect.gen(function* exchangeRequest() {
        yield* send(
          buildRequest({
            token: endpoint.token,
            deviceId,
            stamp,
            messageId,
            method,
            params,
          }),
        )
        const incoming = yield* nextReply(messageId)
        state.deviceId = incoming.packet.deviceId
        state.stamp = incoming.packet.stamp
        state.lastPacketAt = performance.now()
        return incoming.reply
      }).pipe(
        Effect.timeoutOrElse({
          duration: STREAM_TIMEOUT,
          orElse: () => Effect.fail(timeout("stream")),
        }),
      )
    }

    const ensureSession = Effect.gen(function* ensure() {
      if (state.deviceId === null || state.stamp === null) {
        yield* hello()
        return
      }
      if (
        state.lastPacketAt === null ||
        performance.now() - state.lastPacketAt > REHELLO_AFTER_MS
      ) {
        yield* hello()
      }
    })

    const request = (
      method: string,
      params: readonly unknown[],
    ): Effect.Effect<MiioReply, MiioError> =>
      Effect.gen(function* requestReply() {
        yield* ensureSession
        return yield* exchange(nextMessageId(), method, params).pipe(
          Effect.catchIf(
            (error: MiioError) => error.kind === "protocol",
            () =>
              Effect.gen(function* retryAfterProtocolError() {
                yield* hello()
                return yield* exchange(nextMessageId(), method, params)
              }),
          ),
        )
      })

    const setProperties = Effect.fn("MiioSession.setProperties")(function* setProperties(
      params: readonly PropertyParam[],
    ) {
      const reply = yield* request("set_properties", params)
      const failure = commandError(reply, endpoint.address)
      if (failure !== null) {
        return yield* failure
      }
      return yield* Effect.void
    })

    return { hello, setProperties }
  })

export { openMiioSession, type MiioSession }
