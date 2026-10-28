import { createCipheriv, createDecipheriv, createHash } from "node:crypto"

import { isToken } from "@/lib/bulbs/schema"

const HELLO_HEX = "21310020ffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
const MAGIC_FIRST = 0x21
const MAGIC_SECOND = 0x31
const BLOCK_SIZE = 16
const HEADER_SIZE = 16
const CHECKSUM_SIZE = 16
const PACKET_PREFIX = HEADER_SIZE + CHECKSUM_SIZE

interface HelloReply {
  readonly deviceId: Uint8Array
  readonly stamp: Uint8Array
}

interface ParsedPacket extends HelloReply {
  readonly body: Uint8Array
}

interface PacketInput {
  readonly token: string
  readonly deviceId: Uint8Array
  readonly stamp: Uint8Array
  readonly body: Uint8Array
}

interface RequestInput {
  readonly token: string
  readonly deviceId: Uint8Array
  readonly stamp: Uint8Array
  readonly messageId: number
  readonly method: string
  readonly params: readonly unknown[]
}

const tokenBytes = (token: string): Uint8Array => {
  if (!isToken(token)) {
    throw new Error("The token must be 32 hexadecimal characters.")
  }
  return Uint8Array.from(Buffer.from(token, "hex"))
}

const md5 = (parts: readonly Uint8Array[]): Uint8Array => {
  const hash = createHash("md5")
  for (const part of parts) {
    hash.update(part)
  }
  return Uint8Array.from(hash.digest())
}

const pad = (input: Uint8Array): Uint8Array => {
  const size = BLOCK_SIZE - (input.length % BLOCK_SIZE)
  const output = new Uint8Array(input.length + size)
  output.set(input)
  output.fill(size, input.length)
  return output
}

const unpad = (input: Uint8Array): Uint8Array => {
  if (input.length === 0) {
    throw new Error("The bulb sent an empty payload.")
  }
  const size = input.at(-1)
  if (size === undefined || size < 1 || size > BLOCK_SIZE || size > input.length) {
    throw new Error("The bulb sent malformed padding.")
  }
  for (let index = input.length - size; index < input.length; index += 1) {
    if (input[index] !== size) {
      throw new Error("The bulb sent malformed padding.")
    }
  }
  return input.subarray(0, input.length - size)
}

const encrypt = (token: Uint8Array, plain: Uint8Array): Uint8Array => {
  const key = md5([token])
  const iv = md5([key, token])
  const cipher = createCipheriv("aes-128-cbc", key, iv)
  cipher.setAutoPadding(false)
  const encrypted = cipher.update(Buffer.from(pad(plain)))
  const final = cipher.final()
  return Uint8Array.from(Buffer.concat([encrypted, final]))
}

const decrypt = (token: Uint8Array, encrypted: Uint8Array): Uint8Array => {
  const key = md5([token])
  const iv = md5([key, token])
  const decipher = createDecipheriv("aes-128-cbc", key, iv)
  decipher.setAutoPadding(false)
  const plain = decipher.update(Buffer.from(encrypted))
  const final = decipher.final()
  return unpad(Uint8Array.from(Buffer.concat([plain, final])))
}

const buildHello = (): Uint8Array => Uint8Array.from(Buffer.from(HELLO_HEX, "hex"))

const readUint16 = (packet: Uint8Array, offset: number): number => {
  const high = packet[offset]
  const low = packet[offset + 1]
  if (high === undefined || low === undefined) {
    throw new Error("The bulb sent a short packet.")
  }
  return high * 256 + low
}

const messageIdFromStamp = (stamp: Uint8Array): number => {
  if (stamp.length !== 4) {
    throw new Error("A miIO stamp is four bytes.")
  }
  const view = new DataView(stamp.buffer, stamp.byteOffset, stamp.byteLength)
  return (view.getUint32(0, false) + 1) % 4_294_967_296
}

const buildPacket = ({ token, deviceId, stamp, body }: PacketInput): Uint8Array => {
  if (deviceId.length !== 4) {
    throw new Error("A miIO device id is four bytes.")
  }
  if (stamp.length !== 4) {
    throw new Error("A miIO stamp is four bytes.")
  }

  const secret = tokenBytes(token)
  const encrypted = encrypt(secret, body)
  const total = PACKET_PREFIX + encrypted.length
  const header = new Uint8Array(HEADER_SIZE)
  header[0] = MAGIC_FIRST
  header[1] = MAGIC_SECOND
  header[2] = Math.floor(total / 256) % 256
  header[3] = total % 256
  header.set(deviceId, 8)
  header.set(stamp, 12)

  const checksum = md5([header, secret, encrypted])
  const packet = new Uint8Array(total)
  packet.set(header, 0)
  packet.set(checksum, HEADER_SIZE)
  packet.set(encrypted, PACKET_PREFIX)
  return packet
}

const buildRequest = (input: RequestInput): Uint8Array => {
  const body = new TextEncoder().encode(
    JSON.stringify({
      id: input.messageId,
      method: input.method,
      params: input.params,
    }),
  )
  return buildPacket({
    token: input.token,
    deviceId: input.deviceId,
    stamp: input.stamp,
    body,
  })
}

const parseHelloReply = (packet: Uint8Array): HelloReply => {
  if (packet.length < PACKET_PREFIX) {
    throw new Error("The bulb sent a short hello reply.")
  }
  if (packet[0] !== MAGIC_FIRST || packet[1] !== MAGIC_SECOND) {
    throw new Error("The bulb sent an unknown packet.")
  }
  const length = readUint16(packet, 2)
  if (length < PACKET_PREFIX || length > packet.length) {
    throw new Error("The bulb sent a malformed hello reply.")
  }
  return {
    deviceId: packet.slice(8, 12),
    stamp: packet.slice(12, 16),
  }
}

const bytesEqual = (left: Uint8Array, right: Uint8Array): boolean => {
  if (left.length !== right.length) {
    return false
  }
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false
    }
  }
  return true
}

const parsePacket = (token: string, packet: Uint8Array): ParsedPacket => {
  if (packet.length < PACKET_PREFIX) {
    throw new Error("The bulb sent a short reply.")
  }
  if (packet[0] !== MAGIC_FIRST || packet[1] !== MAGIC_SECOND) {
    throw new Error("The bulb sent an unknown packet.")
  }
  const length = readUint16(packet, 2)
  if (length < PACKET_PREFIX || length > packet.length) {
    throw new Error("The bulb sent a malformed reply.")
  }

  const secret = tokenBytes(token)
  const header = packet.subarray(0, HEADER_SIZE)
  const checksum = packet.subarray(HEADER_SIZE, PACKET_PREFIX)
  const encrypted = packet.subarray(PACKET_PREFIX, length)
  const expected = md5([header, secret, encrypted])

  if (!bytesEqual(checksum, expected)) {
    throw new Error("The bulb reply failed its checksum.")
  }

  return {
    deviceId: header.slice(8, 12),
    stamp: header.slice(12, 16),
    body: decrypt(secret, encrypted),
  }
}

export {
  buildHello,
  buildPacket,
  buildRequest,
  messageIdFromStamp,
  parseHelloReply,
  parsePacket,
  tokenBytes,
}
export type { HelloReply, ParsedPacket }
