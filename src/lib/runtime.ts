import type { Config, Effect } from "effect"

import { Layer, ManagedRuntime } from "effect"

import { Bulbs } from "@/lib/bulbs/service"
import { BulbStore } from "@/lib/bulbs/store"
import { AppConfig } from "@/lib/config"
import { Keyring } from "@/lib/keyring/service"
import { Miio } from "@/lib/miio/service"
import { MiioTransport } from "@/lib/miio/transport"
import { ScreenCapture } from "@/lib/screen/capture"
import { ScreenSyncEngine } from "@/lib/screen/sync"

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
const bulbsLayer = Bulbs.layer.pipe(
  Layer.provide(Layer.mergeAll(keyringLayer, storeLayer, miioLayer, configLayer)),
)
const screenLayer = ScreenCapture.layer.pipe(Layer.provide(configLayer))
const servicesLayer = Layer.mergeAll(bulbsLayer, screenLayer)
const screenSyncLayer = ScreenSyncEngine.layer.pipe(Layer.provideMerge(servicesLayer))

const runtime = ManagedRuntime.make(screenSyncLayer)
// One runtime, three typed views: each alias hides the other services so a
// Caller only has to name the service it uses.
const bulbRuntime: ManagedRuntime.ManagedRuntime<Bulbs, Config.ConfigError> = runtime
const screenRuntime: ManagedRuntime.ManagedRuntime<ScreenCapture, Config.ConfigError> = runtime
const syncRuntime: ManagedRuntime.ManagedRuntime<ScreenSyncEngine, Config.ConfigError> = runtime

const messageOf = (error: unknown): string =>
  error instanceof Error ? error.message : "Unexpected error. Press r to refresh, or restart jyoti."

const run = <A, E>(effect: Effect.Effect<A, E, Bulbs>): Promise<UiResult<A>> =>
  bulbRuntime.runPromise(effect).then(
    (value): UiResult<A> => ({ ok: true, value }),
    (error: unknown): UiResult<A> => ({ ok: false, message: messageOf(error) }),
  )

const runScreen = <A, E>(effect: Effect.Effect<A, E, ScreenSyncEngine>): Promise<UiResult<A>> =>
  syncRuntime.runPromise(effect).then(
    (value): UiResult<A> => ({ ok: true, value }),
    (error: unknown): UiResult<A> => ({ ok: false, message: messageOf(error) }),
  )

const forkScreen = <A, E>(effect: Effect.Effect<A, E, ScreenCapture>): void => {
  screenRuntime.runFork(effect)
}

const dispose = (): Promise<void> => runtime.dispose()

export { dispose, forkScreen, messageOf, run, runScreen, type UiResult }
