use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Divider, Kit};

use crate::app::App;
use crate::ui::widget::{BULB, SYNC, panel, panel_body, panel_header, setting_row};

pub fn view(_app: &App, k: &Kit) -> El {
    let add = Button::new("Add a bulb")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.open_add())
        .build(k);
    let open_sync = Button::new("Open screen sync")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.open_sync())
        .build(k);
    let open_settings = Button::new("Open settings")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.open_settings())
        .build(k);

    let getting_started = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Getting started",
                Some("Three steps from an empty window to a controlled bulb."),
                vec![add],
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(step(k, "1", "Add the bulb", "Enter its name, LAN address and 32-character token. The token goes to the OS keyring."))
                    .child(Divider::horizontal().build(k))
                    .child(step(k, "2", "Wait for the state", "The worker greets the bulb, reads power, mode, brightness, warmth and colour."))
                    .child(Divider::horizontal().build(k))
                    .child(step(k, "3", "Control it", "Use the Overview controls or select All bulbs to apply one value to every bulb.")),
            )),
    );

    let sync = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Screen sync",
                Some("One monitor drives every configured bulb."),
                vec![open_sync],
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(setting_row(
                        k,
                        "Capture",
                        "Reads the chosen Wayland output through libwayshot on the GPU path.",
                        text("EGL / DMA-BUF".to_owned()).font(k.ty.mono).color(k.c.text_muted),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Derive",
                        "Finds the dominant colour and mean brightness from a small reduced grid.",
                        text("64 × 36".to_owned()).font(k.ty.mono).color(k.c.text_muted),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Restore",
                        "Stop returns each bulb to the colour or warmth and brightness it had before the run.",
                        text("~2 s".to_owned()).font(k.ty.mono).color(k.c.text_muted),
                    )),
            ))
            .child(
                col()
                    .w_full()
                    .px(14.0)
                    .pb(14.0)
                    .child(crate::ui::widget::notice(
                        k,
                        &SYNC,
                        "Screen sync needs a Wayland compositor that implements wlr-screencopy. GNOME and KDE do not yet.",
                        k.c.info,
                    )),
            ),
    );

    let troubleshooting = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Troubleshooting",
                Some("The common cases and where to go next."),
                vec![open_settings],
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(setting_row(
                        k,
                        "A bulb does not answer",
                        "Confirm the address on the same LAN, local control enabled on the bulb, and the token from the Xiaomi Home app.",
                        Button::new("Refresh bulbs")
                            .variant(ButtonVariant::Ghost)
                            .control_size(ControlSize::Small)
                            .on_press_tx(|app: &mut App, tx| app.refresh_all(tx))
                            .build(k),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "The token is rejected",
                        "Edit the bulb and paste the 32-character token again. Leaving it blank on edit keeps the stored token.",
                        Button::new("Edit selected")
                            .variant(ButtonVariant::Ghost)
                            .control_size(ControlSize::Small)
                            .on_press(|app: &mut App| app.edit_selected())
                            .build(k),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "No monitors found",
                        "The capture path needs a wlr-screencopy compositor such as Hyprland or Sway. The bulb controls keep working without it.",
                        Button::new("Open screen sync")
                            .variant(ButtonVariant::Ghost)
                            .control_size(ControlSize::Small)
                            .on_press(|app: &mut App| app.open_sync())
                            .build(k),
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "A restore was incomplete",
                        "The report on Screen sync names any bulb that could not be restored. Check its connection and try Stop again.",
                        Button::new("Read the report")
                            .variant(ButtonVariant::Ghost)
                            .control_size(ControlSize::Small)
                            .on_press(|app: &mut App| app.open_sync())
                            .build(k),
                    )),
            )),
    );

    let about = panel(
        k,
        row()
            .w_full()
            .padding(14.0)
            .gap(12.0)
            .center_y()
            .child(rune::icon(&BULB, 18.0).color(k.c.text_faint))
            .child(
                text(
                    "jyoti talks to Xiaomi bulbs directly over miIO. No daemon, cloud or account.",
                )
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
            ),
    );

    let scroll = col()
        .key("help")
        .w_full()
        .padding(18.0)
        .gap(14.0)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .child(col().grow(1.0).child(getting_started))
                .child(col().grow(1.0).child(sync)),
        )
        .child(troubleshooting)
        .child(about)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn step(k: &Kit, number: &str, title: &str, description: &str) -> El {
    row()
        .w_full()
        .gap(10.0)
        .py(10.0)
        .items_start()
        .child(
            rune::el()
                .size(22.0, 22.0)
                .center()
                .radius(11.0)
                .fill(k.c.layer_03)
                .child(
                    text(number.to_owned())
                        .font(k.ty.small_medium)
                        .color(k.c.text_base),
                ),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(title.to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(description.to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap(),
                ),
        )
}
