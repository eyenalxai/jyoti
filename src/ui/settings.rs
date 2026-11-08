use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Divider, Kit};

use crate::app::App;
use crate::ui::widget::{badge, panel, panel_body, panel_footer, panel_header, setting_row};

pub fn view(app: &App, k: &Kit) -> El {
    let reset = Button::new("Reset to 100%")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.reset_sync_gain())
        .build(k);
    let open_sync = Button::new("Open screen sync")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.open_sync())
        .build(k);

    let preferences = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Preferences",
                Some("Settings the app carries between launches."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(setting_row(
                        k,
                        "Sync brightness gain",
                        "Scales only the brightness derived from the screen, from 50% to 200%.",
                        row()
                            .gap(10.0)
                            .center_y()
                            .child(
                                text(format!("{:.1}×", app.sync.gain))
                                    .font(k.ty.mono)
                                    .color(k.c.text_base),
                            )
                            .child(reset),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Appearance",
                        "jyoti follows the desktop's light or dark appearance.",
                        badge(k, "Follow system", k.c.info.icon),
                    )),
            ))
            .child(panel_footer(k, vec![open_sync])),
    );

    let metadata = app.config.config_path.display().to_string();
    let preferences_path = app.config.preferences_path.display().to_string();
    let storage = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Storage",
                Some("Secrets stay out of files and logs."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(setting_row(
                        k,
                        "Bulb metadata",
                        "Name, address and model.",
                        path_text(k, &metadata),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Preferences",
                        "Sync brightness gain.",
                        path_text(k, &preferences_path),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Tokens",
                        "One OS keyring entry per bulb. Never written to config.",
                        badge(k, "OS keyring", k.c.success.icon),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Keyring service",
                        "Token account prefix.",
                        text(app.config.keyring_service.clone())
                            .font(k.ty.mono)
                            .color(k.c.text_muted),
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
                Some("A small native control panel with no daemon or account."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(setting_row(
                        k,
                        "Version",
                        "Built from this checkout.",
                        text(env!("CARGO_PKG_VERSION"))
                            .font(k.ty.mono)
                            .color(k.c.text_base),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Protocol",
                        "Xiaomi miIO over UDP on the local network.",
                        text("miIO · UDP".to_owned())
                            .font(k.ty.mono)
                            .color(k.c.text_muted),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Screen capture",
                        "Wayland, libwayshot, EGL/DMA-BUF.",
                        text("libwayshot".to_owned())
                            .font(k.ty.mono)
                            .color(k.c.text_muted),
                    )),
            )),
    );

    let scroll = col()
        .key("settings")
        .w_full()
        .padding(18.0)
        .gap(14.0)
        .child(preferences)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .child(col().grow(1.0).child(storage))
                .child(col().grow(1.0).child(about)),
        )
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn path_text(k: &Kit, value: &str) -> El {
    text(value.to_owned())
        .font(k.ty.mono)
        .color(k.c.text_muted)
        .truncate()
        .w(300.0)
}
