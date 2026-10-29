import { Array as Arr } from "effect"

import type { MiioReply } from "@/lib/bulbs/schema"

import { SIID_LIGHT } from "@/lib/bulbs/schema"
import { MiioError } from "@/lib/miio/error"

interface PropertyParam {
  readonly did: string
  readonly siid: number
  readonly piid: number
  readonly value?: unknown
}

const properties = (piids: readonly number[]): readonly PropertyParam[] =>
  Arr.map(piids, (piid) => ({
    did: `${SIID_LIGHT}-${piid}`,
    siid: SIID_LIGHT,
    piid,
  }))

const propertiesWithValue = (piid: number, value: unknown): readonly PropertyParam[] =>
  Arr.map(properties([piid]), (property) => ({ ...property, value }))

const replyError = (reply: MiioReply, address: string): MiioError | null =>
  reply.error === undefined
    ? null
    : new MiioError({
        kind: "device",
        address,
        message: `The bulb at ${address} refused the command: ${reply.error.message} (${reply.error.code}).`,
      })

const propertyError = (reply: MiioReply, address: string): MiioError | null => {
  for (const item of reply.result ?? []) {
    if (item.code !== undefined && item.code !== 0) {
      return new MiioError({
        kind: "device",
        address,
        message: `Property ${item.siid}-${item.piid} failed with code ${item.code}.`,
      })
    }
  }
  return null
}

const commandError = (reply: MiioReply, address: string): MiioError | null =>
  replyError(reply, address) ?? propertyError(reply, address)

export { commandError, properties, propertiesWithValue, replyError }
export type { PropertyParam }
