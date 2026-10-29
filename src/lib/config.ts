import { Config, Context, Duration, Effect, Layer, Option } from "effect"
import { homedir } from "node:os"
import path from "node:path"

interface AppConfigShape {
  readonly configPath: string
  readonly preferencesPath: string
  readonly keyringService: string
  readonly miioPort: number
  readonly miioTimeout: Duration.Duration
}

class AppConfig extends Context.Service<AppConfig, AppConfigShape>()("jyoti/lib/config/AppConfig") {
  static readonly layer = Layer.effect(
    AppConfig,
    Effect.gen(function* layer() {
      const explicitPath = yield* Config.option(Config.NonEmptyString("JYOTI_CONFIG_PATH"))
      const xdgConfigHome = yield* Config.option(Config.NonEmptyString("XDG_CONFIG_HOME"))
      const configPath = Option.match(explicitPath, {
        onNone: () =>
          path.join(
            Option.getOrElse(xdgConfigHome, () => path.join(homedir(), ".config")),
            "jyoti",
            "bulbs.json",
          ),
        onSome: (value) => value,
      })
      const keyringService = yield* Config.NonEmptyString("JYOTI_KEYRING_SERVICE").pipe(
        Config.withDefault("jyoti"),
      )
      const miioPort = yield* Config.Int("JYOTI_MIIO_PORT").pipe(Config.withDefault(54_321))
      const miioTimeout = yield* Config.Duration("JYOTI_MIIO_TIMEOUT").pipe(
        Config.withDefault(Duration.seconds(4)),
      )

      return AppConfig.of({
        configPath,
        preferencesPath: path.join(path.dirname(configPath), "prefs.json"),
        keyringService,
        miioPort,
        miioTimeout,
      })
    }),
  )
}

export { AppConfig }
