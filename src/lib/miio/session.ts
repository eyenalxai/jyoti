import { BunDatagramSocket } from "@effect/platform-bun"
import { Duration, Effect, Result } from "effect"
import { NetAddress } from "effect/net"

import type { BulbState, MiioReply } from "@/lib/bulbs/schema"
import type { HelloReply, ParsedPacket } from "@/lib/miio/packet"
import type { PropertyParam } from "@/lib/miio/properties"

import { confirmed } from "@/lib/miio/confirm"
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
import { stateFromReply, stateProperties } from "@/lib/miio/state"

const HELLO_TIMEOUT_MS = 1000
const STREAM_TIMEOUT_MS = 600
const HELLO_TIMEOUT = Duration.millis(HELLO_TIMEOUT_MS)
const STREAM_TIMEOUT = Duration.millis(STREAM_TIMEOUT_MS)
const REHELLO_AFTER_MS = 120_000
const MAX_MESSAGE_ID = 9999
const STAMP_MODULUS = 4_294_967_296

interface MiioSessionTarget {
  readonly address: string
  readonly token: string
  readonly port: number
}

interface MiioSession {
  readonly setProperties: (params: readonly PropertyParam[]) => Effect.Effect<void, MiioError>
  readonly confirmProperties: (params: readonly PropertyParam[]) => Effect.Effect<void, MiioError>
  readonly getState: () => Effect.Effect<BulbState, MiioError>
  readonly hello: () => Effect.Effect<void, MiioError>
}

interface SessionState {
  deviceId: Uint8Array | null
  stamp: Uint8Array | null
  stampAt: number | null
  lastPacketAt: number | null
  messageId: number
}

interface IncomingReply {
  readonly packet: ParsedPacket
  readonly reply: MiioReply
}

const stampValue = (stamp: Uint8Array): number =>
  new DataView(stamp.buffer, stamp.byteOffset, stamp.byteLength).getUint32(0, false)

const openMiioSession = Effect.fn("MiioSession.open")(function* openSession(
  target: MiioSessionTarget,
) {
  const resolved = NetAddress.inetAddressFromIpString(target.address, target.port)
  if (Result.isFailure(resolved)) {
    return yield* new MiioError({
      kind: "address",
      address: target.address,
      message: `"${target.address}" is not a valid IP address. Edit the bulb and enter its IP.`,
    })
  }

  yield* Effect.try({
    try: () => tokenBytes(target.token),
    catch: (cause) =>
      new MiioError({
        kind: "token",
        address: target.address,
        message:
          "The stored token is not 32 hexadecimal characters. Edit the bulb and enter its token again.",
        cause,
      }),
  })

  const socket = yield* BunDatagramSocket.make()
  const reader = yield* socket.reader.pipe(
    Effect.mapError((cause) => toMiioError(cause, target.address)),
  )
  const writer = yield* socket.writer
  const destination = resolved.success
  const state: SessionState = {
    deviceId: null,
    stamp: null,
    stampAt: null,
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
      .pipe(Effect.mapError((cause) => toMiioError(cause, target.address)))

  const pull = reader.pull.pipe(Effect.mapError((cause) => toMiioError(cause, target.address)))

  const timeout = (kind: "hello" | "stream"): MiioError =>
    new MiioError({
      kind: "timeout",
      address: target.address,
      message:
        kind === "hello"
          ? `No reply from ${target.address} to the miIO hello. Check its power and network.`
          : `No reply from ${target.address} within ${STREAM_TIMEOUT_MS} ms.`,
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

  const advancedStamp = (): Uint8Array | null => {
    const stamp = state.stamp
    const stampAt = state.stampAt
    if (stamp === null || stampAt === null) {
      return null
    }
    const elapsed = Math.floor((performance.now() - stampAt) / 1000)
    const bytes = new Uint8Array(4)
    new DataView(bytes.buffer).setUint32(0, (stampValue(stamp) + elapsed) % STAMP_MODULUS, false)
    return bytes
  }

  const nextHelloReply = (): Effect.Effect<HelloReply, MiioError> =>
    Effect.gen(function* readHello() {
      const packets = yield* pull
      for (const packet of packets) {
        return yield* Effect.try({
          try: () => parseHelloReply(packet.payload),
          catch: (cause) =>
            new MiioError({
              kind: "protocol",
              address: target.address,
              message: `The bulb at ${target.address} sent a hello reply jyoti could not read.`,
              cause,
            }),
        })
      }
      return yield* nextHelloReply()
    })

  const hello = Effect.fn("MiioSession.hello")(function* hello() {
    yield* send(buildHello())
    const reply = yield* within(nextHelloReply(), HELLO_TIMEOUT, "hello")
    const receivedAt = performance.now()
    state.deviceId = reply.deviceId
    state.stamp = reply.stamp
    state.stampAt = receivedAt
    state.lastPacketAt = receivedAt
  })

  const nextReply = (messageId: number): Effect.Effect<IncomingReply, MiioError> =>
    Effect.gen(function* readReply() {
      const packets = yield* pull
      for (const packet of packets) {
        const parsed = yield* Effect.try({
          try: () => parsePacket(target.token, packet.payload),
          catch: (cause) =>
            new MiioError({
              kind: "protocol",
              address: target.address,
              message: `The bulb at ${target.address} sent a reply jyoti could not read.`,
              cause,
            }),
        })
        const reply = yield* decodeReply(parsed.body, target.address)
        state.lastPacketAt = performance.now()
        if (reply.id === messageId) {
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
    const stamp = advancedStamp()
    if (deviceId === null || stamp === null) {
      return Effect.fail(
        new MiioError({
          kind: "protocol",
          address: target.address,
          message: `The session to ${target.address} has no device ID. Reconnect and try again.`,
        }),
      )
    }

    return Effect.gen(function* exchangeRequest() {
      yield* send(
        buildRequest({
          token: target.token,
          deviceId,
          stamp,
          messageId,
          method,
          params,
        }),
      )
      const incoming = yield* nextReply(messageId)
      const receivedAt = performance.now()
      state.deviceId = incoming.packet.deviceId
      state.stamp = incoming.packet.stamp
      state.stampAt = receivedAt
      state.lastPacketAt = receivedAt
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
    if (state.lastPacketAt === null || performance.now() - state.lastPacketAt > REHELLO_AFTER_MS) {
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
    const failure = commandError(reply, target.address)
    if (failure !== null) {
      return yield* failure
    }
    return yield* Effect.void
  })

  const confirmProperties = Effect.fn("MiioSession.confirmProperties")(function* confirmProperties(
    params: readonly PropertyParam[],
  ) {
    yield* confirmed(
      () => setProperties(params),
      () => timeout("stream"),
    )
  })

  const getState = Effect.fn("MiioSession.getState")(function* getState() {
    const reply = yield* request("get_properties", stateProperties())
    return stateFromReply(reply)
  })

  yield* hello()
  return { setProperties, confirmProperties, getState, hello }
})

export { openMiioSession, type MiioSession }
