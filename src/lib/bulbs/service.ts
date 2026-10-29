import type { Scope } from "effect"

import {
  Array as Arr,
  Context,
  Effect,
  Layer,
  Option,
  Result,
  Semaphore,
  String as Str,
} from "effect"

import type { BulbsError } from "@/lib/bulbs/error"
import type {
  Bulb,
  BulbDraft,
  BulbEndpoint,
  BulbId,
  BulbPatch,
  BulbState,
} from "@/lib/bulbs/schema"
import type { MiioSession } from "@/lib/miio/session"

import { BulbConflict, BulbInvalid, BulbTokenMissing } from "@/lib/bulbs/error"
import { findBulb, firstIssue, newId, withModel } from "@/lib/bulbs/helpers"
import { isToken, parseColor, validateDraft } from "@/lib/bulbs/schema"
import { BulbStore } from "@/lib/bulbs/store"
import { AppConfig } from "@/lib/config"
import { Keyring } from "@/lib/keyring/service"
import { Miio } from "@/lib/miio/service"
import { openMiioSession } from "@/lib/miio/session"

interface BulbsShape {
  readonly list: () => Effect.Effect<readonly Bulb[], BulbsError>
  readonly add: (draft: BulbDraft) => Effect.Effect<Bulb, BulbsError>
  readonly update: (id: BulbId, patch: BulbPatch) => Effect.Effect<Bulb, BulbsError>
  readonly remove: (id: BulbId) => Effect.Effect<void, BulbsError>
  readonly readState: (id: BulbId) => Effect.Effect<BulbState, BulbsError>
  readonly openSession: (id: BulbId) => Effect.Effect<MiioSession, BulbsError, Scope.Scope>
  readonly setPower: (id: BulbId, on: boolean) => Effect.Effect<void, BulbsError>
  readonly setBrightness: (id: BulbId, value: number) => Effect.Effect<void, BulbsError>
  readonly setTemperature: (id: BulbId, kelvin: number) => Effect.Effect<void, BulbsError>
  readonly setColor: (id: BulbId, hex: string) => Effect.Effect<void, BulbsError>
}

class Bulbs extends Context.Service<Bulbs, BulbsShape>()("jyoti/lib/bulbs/service/Bulbs") {
  static readonly layer = Layer.effect(
    Bulbs,
    Effect.gen(function* layer() {
      const store = yield* BulbStore
      const keyring = yield* Keyring
      const miio = yield* Miio
      const config = yield* AppConfig
      const mutation = yield* Semaphore.make(1)

      const endpoint = Effect.fn("Bulbs.endpoint")(function* endpoint(bulb: Bulb) {
        const stored = yield* keyring.get(bulb.id)
        if (Option.isNone(stored)) {
          return yield* new BulbTokenMissing({
            id: bulb.id,
            message: `No token is stored for ${bulb.name}. Edit the bulb and enter its token.`,
          })
        }
        if (!isToken(stored.value)) {
          return yield* new BulbTokenMissing({
            id: bulb.id,
            message: `The stored token for ${bulb.name} is not valid. Edit the bulb and enter its token again.`,
          })
        }
        return { address: bulb.address, token: stored.value } satisfies BulbEndpoint
      })

      const list: BulbsShape["list"] = Effect.fn("Bulbs.list")(function* list() {
        return yield* store.load()
      })

      const add: BulbsShape["add"] = Effect.fn("Bulbs.add")(function* addBulb(draft: BulbDraft) {
        return yield* mutation.withPermit(
          Effect.gen(function* addPermit() {
            const issues = validateDraft(draft, { tokenRequired: true })
            if (issues.length > 0) {
              return yield* firstIssue(issues)
            }

            const name = Str.trim(draft.name)
            const address = Str.trim(draft.address)
            const model = Str.trim(draft.model ?? "")
            const token = Str.trim(draft.token)
            const bulbs = yield* store.load()

            if (Arr.some(bulbs, (bulb) => Str.toLowerCase(bulb.name) === Str.toLowerCase(name))) {
              return yield* new BulbConflict({
                field: "name",
                message: `A bulb named ${name} already exists.`,
              })
            }
            if (Arr.some(bulbs, (bulb) => bulb.address === address)) {
              return yield* new BulbConflict({
                field: "address",
                message: `A bulb at ${address} already exists.`,
              })
            }

            const bulb = withModel({ id: yield* newId, name, address }, model)
            yield* keyring.set(bulb.id, token)

            const saved = yield* store.save([...bulbs, bulb]).pipe(Effect.result)
            if (Result.isFailure(saved)) {
              yield* keyring.remove(bulb.id).pipe(Effect.ignore)
              return yield* saved.failure
            }

            return bulb
          }),
        )
      })

      const update: BulbsShape["update"] = Effect.fn("Bulbs.update")(function* updateBulb(
        id: BulbId,
        patch: BulbPatch,
      ) {
        return yield* mutation.withPermit(
          Effect.gen(function* updatePermit() {
            const bulbs = yield* store.load()
            const existing = yield* findBulb(bulbs, id)
            const issues = validateDraft(patch, { tokenRequired: false })
            if (issues.length > 0) {
              return yield* firstIssue(issues)
            }

            const name = Str.trim(patch.name)
            const address = Str.trim(patch.address)
            const model = Str.trim(patch.model ?? "")
            const token = Str.trim(patch.token ?? "")

            if (
              Arr.some(
                bulbs,
                (bulb) => bulb.id !== id && Str.toLowerCase(bulb.name) === Str.toLowerCase(name),
              )
            ) {
              return yield* new BulbConflict({
                field: "name",
                message: `A bulb named ${name} already exists.`,
              })
            }
            if (Arr.some(bulbs, (bulb) => bulb.id !== id && bulb.address === address)) {
              return yield* new BulbConflict({
                field: "address",
                message: `A bulb at ${address} already exists.`,
              })
            }

            if (token.length > 0 && !isToken(token)) {
              return yield* new BulbInvalid({
                field: "token",
                message: "A token is 32 hexadecimal characters.",
              })
            }

            const updated = withModel({ id: existing.id, name, address }, model)
            const nextBulbs = Arr.map(bulbs, (bulb) => (bulb.id === id ? updated : bulb))

            if (token.length === 0) {
              yield* store.save(nextBulbs)
              return updated
            }

            const previous = yield* keyring.get(id)
            yield* keyring.set(id, token)
            const saved = yield* store.save(nextBulbs).pipe(Effect.result)
            if (Result.isFailure(saved)) {
              yield* (
                Option.isSome(previous) ? keyring.set(id, previous.value) : keyring.remove(id)
              ).pipe(Effect.ignore)
              return yield* saved.failure
            }

            return updated
          }),
        )
      })

      const remove: BulbsShape["remove"] = Effect.fn("Bulbs.remove")(function* removeBulb(
        id: BulbId,
      ) {
        return yield* mutation.withPermit(
          Effect.gen(function* removePermit() {
            const bulbs = yield* store.load()
            yield* findBulb(bulbs, id)
            const previous = yield* keyring.get(id)
            const nextBulbs = Arr.filter(bulbs, (bulb) => bulb.id !== id)

            yield* keyring.remove(id)
            const saved = yield* store.save(nextBulbs).pipe(Effect.result)
            if (Result.isFailure(saved)) {
              if (Option.isSome(previous)) {
                yield* keyring.set(id, previous.value).pipe(Effect.ignore)
              }
              return yield* saved.failure
            }
            return yield* Effect.void
          }),
        )
      })

      const readState: BulbsShape["readState"] = Effect.fn("Bulbs.readState")(function* readState(
        id: BulbId,
      ) {
        const bulbs = yield* store.load()
        const bulb = yield* findBulb(bulbs, id)
        const target = yield* endpoint(bulb)
        return yield* miio.getState(target)
      })

      const openSession: BulbsShape["openSession"] = Effect.fn("Bulbs.openSession")(
        function* openSession(id: BulbId) {
          const bulbs = yield* store.load()
          const bulb = yield* findBulb(bulbs, id)
          const target = yield* endpoint(bulb)
          const session = yield* openMiioSession(target, config.miioPort)
          yield* session.hello()
          return session
        },
      )

      const setPower: BulbsShape["setPower"] = Effect.fn("Bulbs.setPower")(function* setPower(
        id: BulbId,
        on: boolean,
      ) {
        const bulbs = yield* store.load()
        const bulb = yield* findBulb(bulbs, id)
        const target = yield* endpoint(bulb)
        yield* miio.setPower(target, on)
      })

      const setBrightness: BulbsShape["setBrightness"] = Effect.fn("Bulbs.setBrightness")(
        function* setBrightness(id: BulbId, value: number) {
          const bulbs = yield* store.load()
          const bulb = yield* findBulb(bulbs, id)
          const target = yield* endpoint(bulb)
          yield* miio.setBrightness(target, value)
        },
      )

      const setTemperature: BulbsShape["setTemperature"] = Effect.fn("Bulbs.setTemperature")(
        function* setTemperature(id: BulbId, kelvin: number) {
          const bulbs = yield* store.load()
          const bulb = yield* findBulb(bulbs, id)
          const target = yield* endpoint(bulb)
          yield* miio.setTemperature(target, kelvin)
        },
      )

      const setColor: BulbsShape["setColor"] = Effect.fn("Bulbs.setColor")(function* setColor(
        id: BulbId,
        hex: string,
      ) {
        const rgb = parseColor(hex)
        if (rgb === null) {
          return yield* new BulbInvalid({
            field: "color",
            message: "Enter a color as #RRGGBB.",
          })
        }
        const bulbs = yield* store.load()
        const bulb = yield* findBulb(bulbs, id)
        const target = yield* endpoint(bulb)
        yield* miio.setColor(target, rgb)
        return yield* Effect.void
      })

      return Bulbs.of({
        list,
        add,
        update,
        remove,
        readState,
        openSession,
        setPower,
        setBrightness,
        setTemperature,
        setColor,
      })
    }),
  )
}

export { Bulbs, type BulbsShape }
