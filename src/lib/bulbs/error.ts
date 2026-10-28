import { Schema } from "effect"

import type { BulbStoreError } from "@/lib/bulbs/store"
import type { KeyringError } from "@/lib/keyring/service"
import type { MiioError } from "@/lib/miio/error"

const BulbFieldSchema = Schema.Literals(["name", "address", "model", "token", "color"])

class BulbInvalid extends Schema.TaggedError<BulbInvalid>()("BulbInvalid", {
  field: BulbFieldSchema,
  message: Schema.String,
}) {}

class BulbConflict extends Schema.TaggedError<BulbConflict>()("BulbConflict", {
  field: Schema.Literals(["name", "address"]),
  message: Schema.String,
}) {}

class BulbNotFound extends Schema.TaggedError<BulbNotFound>()("BulbNotFound", {
  id: Schema.String,
  message: Schema.String,
}) {}

class BulbTokenMissing extends Schema.TaggedError<BulbTokenMissing>()("BulbTokenMissing", {
  id: Schema.String,
  message: Schema.String,
}) {}

type BulbField = typeof BulbFieldSchema.Type

type BulbsError =
  | BulbStoreError
  | KeyringError
  | MiioError
  | BulbInvalid
  | BulbConflict
  | BulbNotFound
  | BulbTokenMissing

export {
  BulbConflict,
  BulbInvalid,
  BulbNotFound,
  BulbTokenMissing,
  type BulbField,
  type BulbsError,
}
