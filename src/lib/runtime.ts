import type { Effect } from "effect"

import { Layer, ManagedRuntime } from "effect"

import { Bulbs } from "@/lib/bulbs/service"
import { BulbStore } from "@/lib/bulbs/store"
import { AppConfig } from "@/lib/config"
import { Keyring } from "@/lib/keyring/service"
import { Miio } from "@/lib/miio/service"
import { MiioTransport } from "@/lib/miio/transport"

interface UiFailure {
  readonly ok: false
  readonly message: string
}

interface UiSuccess<A> {
  readonly ok: true
  readonly value: A
}

type UiResult<A> = UiFailure | UiSuccess<A>

const configLayer = AppConfig.layer
const keyringLayer = Keyring.layer.pipe(Layer.provide(configLayer))
const storeLayer = BulbStore.layer.pipe(Layer.provide(configLayer))
const transportLayer = MiioTransport.layer.pipe(Layer.provide(configLayer))
const miioLayer = Miio.layer.pipe(Layer.provide(transportLayer))
const appLayer = Bulbs.layer.pipe(
  Layer.provide(Layer.mergeAll(keyringLayer, storeLayer, miioLayer)),
)

const runtime = ManagedRuntime.make(appLayer)

const messageOf = (error: unknown): string =>
  error instanceof Error ? error.message : "Unexpected error. Press r to refresh, or restart jyoti."

const run = <A, E>(effect: Effect.Effect<A, E, Bulbs>): Promise<UiResult<A>> =>
  runtime.runPromise(effect).then(
    (value): UiResult<A> => ({ ok: true, value }),
    (error: unknown): UiResult<A> => ({ ok: false, message: messageOf(error) }),
  )

const dispose = (): Promise<void> => runtime.dispose()

export { dispose, messageOf, run, type UiResult }
