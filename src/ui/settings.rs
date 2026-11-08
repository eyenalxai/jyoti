use rune::{El, ScrollSpec, col, row, text};
use rune_kit::Kit;

use crate::app::App;
use crate::ui::widget::{LOCK, chip, fact_row, group, hairline, setting_row};

pub fn view(app: &App, k: &Kit) -> El {
    let appearance = group(
        k,
        "Appearance",
        setting_row(
            k,
            "Theme",
            "Light or dark follows the desktop setting.",
            chip(k, "Follows system", k.c.text_accent),
        ),
    );

    let metadata = app.config.config_path.display().to_string();
    let preferences = app.config.preferences_path.display().to_string();
    let storage = group(
        k,
        "Storage",
        col()
            .w_full()
            .child(path_row(
                k,
                "Bulb metadata",
                "Name, address and model.",
                &metadata,
            ))
            .child(hairline(k))
            .child(path_row(
                k,
                "Preferences",
                "Screen-sync brightness gain and colour intensity.",
                &preferences,
            ))
            .child(hairline(k))
            .child(setting_row(
                k,
                "Tokens",
                "One OS keyring entry per bulb.",
                row()
                    .gap(6.0)
                    .center_y()
                    .child(
                        rune::icon(&LOCK, 14.0)
                            .color(k.c.success.icon)
                            .a11y_hidden(),
                    )
                    .child(
                        text("OS keyring".to_owned())
                            .font(k.ty.small_medium)
                            .color(k.c.success.fg),
                    ),
            ))
            .child(hairline(k))
            .child(path_row(
                k,
                "Keyring service",
                "The account prefix for every token.",
                &app.config.keyring_service,
            )),
    );

    let about = group(
        k,
        "About",
        col()
            .w_full()
            .padding(12.0)
            .child(fact_row(k, "Version", env!("CARGO_PKG_VERSION")))
            .child(fact_row(k, "Protocol", "miIO over UDP"))
            .child(fact_row(k, "Capture", "libwayshot · EGL/DMA-BUF")),
    );

    let viewport = col()
        .key("settings")
        .w_full()
        .padding(12.0)
        .gap(11.0)
        .child(appearance)
        .child(storage)
        .child(about)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn path_row(k: &Kit, title: &str, description: &str, path: &str) -> El {
    col()
        .w_full()
        .gap(2.0)
        .px(14.0)
        .py(10.0)
        .child(
            text(title.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(
            text(description.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            text(path.to_owned())
                .font(k.ty.mono)
                .color(k.c.text_faint)
                .wrap(),
        )
}
