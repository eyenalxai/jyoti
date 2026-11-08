use rune::{El, ScrollSpec, col, text};
use rune_kit::{Divider, Kit};

use crate::app::App;
use crate::ui::widget::{badge, fact_row, panel, panel_body, panel_header, setting_block};

pub fn view(app: &App, k: &Kit) -> El {
    let appearance = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Appearance",
                Some("jyoti follows the desktop."),
                Vec::new(),
            ))
            .child(panel_body(setting_block(
                k,
                "Theme",
                "Light or dark comes from the system setting.",
                badge(k, "Follow system", k.c.info.icon),
            ))),
    );

    let metadata = app.config.config_path.display().to_string();
    let preferences = app.config.preferences_path.display().to_string();
    let storage = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Storage",
                Some("Secrets never enter a file or a log."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(setting_block(
                        k,
                        "Bulb metadata",
                        "Name, address and model.",
                        path(k, &metadata),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_block(
                        k,
                        "Preferences file",
                        "Screen-sync brightness gain.",
                        path(k, &preferences),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_block(
                        k,
                        "Tokens",
                        "One OS keyring entry per bulb.",
                        badge(k, "OS keyring", k.c.success.icon),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_block(
                        k,
                        "Keyring service",
                        "The account prefix for every token.",
                        path(k, &app.config.keyring_service),
                    )),
            )),
    );

    let about = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "About",
                Some("A native panel with no daemon or account."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(fact_row(k, "Version", env!("CARGO_PKG_VERSION")))
                    .child(fact_row(k, "Protocol", "miIO over UDP"))
                    .child(fact_row(k, "Capture", "libwayshot · EGL/DMA-BUF")),
            )),
    );

    let scroll = col()
        .key("settings")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(appearance)
        .child(storage)
        .child(about)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn path(k: &Kit, value: &str) -> El {
    text(value.to_owned())
        .font(k.ty.mono)
        .color(k.c.text_muted)
        .wrap()
        .w_full()
}
