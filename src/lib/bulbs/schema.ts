import { Result, Schema, String as Str } from "effect"
import { NetAddress } from "effect/net"

const BulbIdSchema = Schema.String.pipe(
  Schema.check(
    Schema.isPattern(/^[0-9a-f]{8}-[0-9a-f]{4}-[1-7][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu),
  ),
  Schema.brand("BulbId"),
)
type BulbId = typeof BulbIdSchema.Type

const TokenSchema = Schema.String.pipe(
  Schema.check(Schema.isPattern(/^[0-9a-f]{32}$/iu)),
  Schema.brand("Token"),
)
type Token = typeof TokenSchema.Type

const ColorHexSchema = Schema.String.pipe(Schema.check(Schema.isPattern(/^#[0-9a-f]{6}$/iu)))
type ColorHex = typeof ColorHexSchema.Type

const BulbSchema = Schema.Struct({
  id: BulbIdSchema,
  name: Schema.NonEmptyString,
  address: Schema.NonEmptyString,
  model: Schema.optionalKey(Schema.String),
})
type Bulb = typeof BulbSchema.Type

const BulbFileSchema = Schema.Struct({
  version: Schema.Literal(1),
  bulbs: Schema.Array(BulbSchema),
})
type BulbFile = typeof BulbFileSchema.Type

const ColorModeSchema = Schema.Literals(["color", "temperature", "unknown"])
type ColorMode = typeof ColorModeSchema.Type

const BulbStateSchema = Schema.Struct({
  power: Schema.NullOr(Schema.Boolean),
  brightness: Schema.NullOr(Schema.Int),
  temperature: Schema.NullOr(Schema.Int),
  color: Schema.NullOr(ColorHexSchema),
  mode: ColorModeSchema,
})
type BulbState = typeof BulbStateSchema.Type

const MiioResultItemSchema = Schema.Struct({
  did: Schema.optionalKey(Schema.String),
  siid: Schema.Int,
  piid: Schema.Int,
  value: Schema.optionalKey(Schema.Unknown),
  code: Schema.optionalKey(Schema.Int),
})
type MiioResultItem = typeof MiioResultItemSchema.Type

const MiioReplySchema = Schema.Struct({
  id: Schema.optionalKey(Schema.Int),
  result: Schema.optionalKey(Schema.Array(MiioResultItemSchema)),
  error: Schema.optionalKey(
    Schema.Struct({
      code: Schema.Int,
      message: Schema.String,
    }),
  ),
})
type MiioReply = typeof MiioReplySchema.Type

interface BulbDraft {
  readonly name: string
  readonly address: string
  readonly model?: string
  readonly token: string
}

interface BulbPatch {
  readonly name: string
  readonly address: string
  readonly model?: string
  readonly token?: string
}

interface BulbEndpoint {
  readonly address: string
  readonly token: string
}

type BulbField = "name" | "address" | "model" | "token" | "color"

interface FieldIssue {
  readonly field: BulbField
  readonly message: string
}

interface Rgb {
  readonly red: number
  readonly green: number
  readonly blue: number
}

const BRIGHTNESS_MIN = 1
const BRIGHTNESS_MAX = 100
const BRIGHTNESS_STEP = 5
const TEMPERATURE_MIN = 2700
const TEMPERATURE_MAX = 6500
const TEMPERATURE_STEP = 100

const SIID_LIGHT = 2
const PIID_POWER = 1
const PIID_BRIGHTNESS = 2
const PIID_TEMPERATURE = 3
const PIID_COLOR = 4
const PIID_COLOR_MODE = 6

const isToken = (value: string): boolean => /^[0-9a-f]{32}$/iu.test(value)

const parseColor = (value: string): Rgb | null => {
  const match = /^#?(?<hex>[0-9a-f]{6})$/iu.exec(Str.trim(value))
  const hex = match?.groups?.hex
  if (hex === undefined) {
    return null
  }
  return {
    red: Number.parseInt(Str.slice(0, 2)(hex), 16),
    green: Number.parseInt(Str.slice(2, 4)(hex), 16),
    blue: Number.parseInt(Str.slice(4, 6)(hex), 16),
  }
}

const packColor = (rgb: Rgb): number => rgb.red * 65_536 + rgb.green * 256 + rgb.blue

const packHex = (hex: ColorHex): number => Number.parseInt(Str.slice(1, 7)(hex), 16)

const formatColor = (value: number): ColorHex => {
  const hex = Str.padStart(6, "0")(Str.toUpperCase(value.toString(16)))
  return `#${hex}`
}

interface DraftValidation {
  readonly tokenRequired: boolean
}

const validateDraft = (
  input: {
    readonly name: string
    readonly address: string
    readonly model?: string | undefined
    readonly token?: string | undefined
  },
  options: DraftValidation,
): readonly FieldIssue[] => {
  const issues: FieldIssue[] = []
  const name = Str.trim(input.name)
  const address = Str.trim(input.address)
  const model = Str.trim(input.model ?? "")
  const token = Str.trim(input.token ?? "")

  if (name.length === 0) {
    issues.push({ field: "name", message: "Give the bulb a name." })
  } else if (name.length > 60) {
    issues.push({ field: "name", message: "Keep the name under 60 characters." })
  }

  if (address.length === 0) {
    issues.push({ field: "address", message: "Enter the bulb's IP address." })
  } else if (Result.isFailure(NetAddress.inetAddressFromIpString(address, 54_321))) {
    issues.push({
      field: "address",
      message: "Use an IPv4 or IPv6 address, for example 192.168.1.42.",
    })
  }

  if (model.length > 60) {
    issues.push({ field: "model", message: "Keep the model under 60 characters." })
  }

  if (token.length === 0) {
    if (options.tokenRequired) {
      issues.push({
        field: "token",
        message: "Paste the 32-character token from the Xiaomi app or setup script.",
      })
    }
  } else if (!isToken(token)) {
    issues.push({ field: "token", message: "A token is 32 hexadecimal characters." })
  }

  return issues
}

export {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  BRIGHTNESS_STEP,
  BulbFileSchema,
  BulbIdSchema,
  BulbSchema,
  BulbStateSchema,
  ColorHexSchema,
  MiioReplySchema,
  PIID_BRIGHTNESS,
  PIID_COLOR,
  PIID_COLOR_MODE,
  PIID_POWER,
  PIID_TEMPERATURE,
  SIID_LIGHT,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
  TEMPERATURE_STEP,
  TokenSchema,
  formatColor,
  isToken,
  packColor,
  packHex,
  parseColor,
  validateDraft,
}
export type {
  Bulb,
  BulbDraft,
  BulbFile,
  BulbId,
  BulbState,
  ColorHex,
  MiioReply,
  Token,
  BulbEndpoint,
  BulbField,
  BulbPatch,
  ColorMode,
  FieldIssue,
  MiioResultItem,
  Rgb,
}
