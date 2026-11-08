use std::time::Duration;

use rune::{El, ScrollSpec, TextStyle, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Divider, Kit, TextInput};

use crate::app::App;
use crate::app::state::ColorTab;
use crate::ui::widget::{
    BULB, badge, notice, panel, panel_body, panel_footer, panel_header, rgb_color, segmented,
    setting_row, slider, swatch,
};

fn choose_tab(app: &mut App, index: usize) {
    app.color.tab = if index == 0 {
        ColorTab::Hs
    } else {
        ColorTab::Rgb
    };
}

pub fn view(app: &App, k: &Kit) -> El {
    let preview = app.color.preview;
    let cancel = Button::new("Cancel")
        .variant(ButtonVariant::Ghost)
        .on_press(|app: &mut App| app.cancel_color())
        .build(k);
    let done = Button::new("Keep colour")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Regular)
        .on_press(|app: &mut App| app.commit_color())
        .build(k);

    let hex = TextInput::new(&app.color.hex)
        .label("Hex colour")
        .placeholder("#FF8A00")
        .width(Some(180.0))
        .on_edit(|app: &mut App| &mut app.color.hex)
        .on_submit(|app: &mut App, tx: &mut Tx| {
            app.update_color_from_hex();
            tx.wake_after(Duration::from_millis(160));
        })
        .build(k);
    let apply = Button::new("Apply")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.update_color_from_hex())
        .build(k);

    let selected_tab = if app.color.tab == ColorTab::Hs {
        Some(0)
    } else {
        Some(1)
    };
    let tab = segmented(
        k,
        &["Hue / saturation", "RGB"],
        selected_tab,
        "Colour picker mode",
        320.0,
        choose_tab,
    );

    let preview_column = col()
        .w(300.0)
        .gap(10.0)
        .child(swatch(k, rgb_color(preview), 140.0))
        .child(
            text(preview.hex())
                .font(TextStyle::new(24.0, 600.0).lh(28.0))
                .color(k.c.text_base),
        )
        .child(
            text(format!(
                "Applying to {} {}",
                app.color.targets.len(),
                if app.color.targets.len() == 1 {
                    "bulb"
                } else {
                    "bulbs"
                }
            ))
            .font(k.ty.small)
            .color(k.c.text_muted),
        )
        .child(row().gap(8.0).center_y().child(hex).child(apply));
    let preview_column = if let Some(error) = &app.color.error {
        preview_column.child(notice(k, &BULB, error, k.c.danger))
    } else {
        preview_column
    };

    let controls = match app.color.tab {
        ColorTab::Hs => col()
            .w_full()
            .gap(0.0)
            .child(setting_row(
                k,
                "Hue",
                "0° to 360° around the colour wheel.",
                slider_control(
                    k,
                    "hue",
                    app.color.hue / 360.0,
                    "Hue",
                    1.0 / 360.0,
                    format!("{:.0}°", app.color.hue),
                    |app: &mut App, fraction| {
                        app.color.hue = fraction * 360.0;
                        app.update_color_from_hsb();
                    },
                ),
            ))
            .child(Divider::horizontal().build(k))
            .child(setting_row(
                k,
                "Saturation",
                "0% is grey; 100% is the pure hue.",
                slider_control(
                    k,
                    "saturation",
                    app.color.saturation / 100.0,
                    "Saturation",
                    1.0 / 100.0,
                    format!("{:.0}%", app.color.saturation),
                    |app: &mut App, fraction| {
                        app.color.saturation = fraction * 100.0;
                        app.update_color_from_hsb();
                    },
                ),
            )),
        ColorTab::Rgb => col()
            .w_full()
            .gap(0.0)
            .child(setting_row(
                k,
                "Red",
                "The red channel, 0 to 255.",
                channel_control(app, k, "red", Channel::Red),
            ))
            .child(Divider::horizontal().build(k))
            .child(setting_row(
                k,
                "Green",
                "The green channel, 0 to 255.",
                channel_control(app, k, "green", Channel::Green),
            ))
            .child(Divider::horizontal().build(k))
            .child(setting_row(
                k,
                "Blue",
                "The blue channel, 0 to 255.",
                channel_control(app, k, "blue", Channel::Blue),
            )),
    };

    let right = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(0.0)
        .child(setting_row(
            k,
            "Picker",
            "Edit hue and saturation, or the RGB channels.",
            tab,
        ))
        .child(Divider::horizontal().build(k))
        .child(controls);

    let body = col()
        .w_full()
        .child(panel_body(
            row()
                .w_full()
                .gap(16.0)
                .items_start()
                .child(preview_column)
                .child(right),
        ))
        .child(
            col()
                .w_full()
                .px(14.0)
                .pb(14.0)
                .gap(8.0)
                .child(
                    text("Changes preview on the bulbs as you adjust. Keep colour stores it; Cancel restores the previous colour and mode."
                        .to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap(),
                ),
        )
        .child(panel_footer(k, vec![cancel, done]));

    let panel = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Colour picker",
                Some("Preview live, then keep or restore the previous light."),
                vec![badge(k, "Live preview", k.c.info.icon)],
            ))
            .child(body),
    );

    let scroll = col()
        .key("colour")
        .w_full()
        .padding(18.0)
        .gap(14.0)
        .child(col().w_full().max_w(1040.0).child(panel))
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn slider_control(
    k: &Kit,
    key: &str,
    value: f32,
    label: &str,
    step: f32,
    display: String,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    col()
        .w(320.0)
        .gap(4.0)
        .child(
            row()
                .w_full()
                .justify_end()
                .child(text(display).font(k.ty.mono).color(k.c.text_base)),
        )
        .child(slider(k, key, value, label, step, 320.0, on_change))
}

enum Channel {
    Red,
    Green,
    Blue,
}

fn channel_control(app: &App, k: &Kit, key: &str, channel: Channel) -> El {
    let (value, label) = match channel {
        Channel::Red => (app.color.preview.red, "Red"),
        Channel::Green => (app.color.preview.green, "Green"),
        Channel::Blue => (app.color.preview.blue, "Blue"),
    };
    let channel = match channel {
        Channel::Red => 0,
        Channel::Green => 1,
        Channel::Blue => 2,
    };
    slider_control(
        k,
        key,
        f32::from(value) / 255.0,
        label,
        1.0 / 255.0,
        format!("{value}"),
        move |app: &mut App, fraction| {
            let mut color = app.color.preview;
            let next = (fraction * 255.0).round() as u8;
            match channel {
                0 => color.red = next,
                1 => color.green = next,
                _ => color.blue = next,
            }
            app.update_color_from_rgb(color);
        },
    )
}
