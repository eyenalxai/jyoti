use std::time::Duration;

use rune::{El, ScrollSpec, TextStyle, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, Divider, IconButton, IconButtonVariant, Kit, TextInput,
};

use crate::app::App;
use crate::app::state::ColorTab;
use crate::ui::widget::{
    BACK, BULB, notice, panel, panel_body, panel_footer, panel_header, rgb_color, segmented,
    slider, subpage_header, swatch,
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
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Regular)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);
    let done = Button::new("Keep colour")
        .variant(ButtonVariant::Contrast)
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
        400.0,
        choose_tab,
    );

    let mut details = col().w_full().gap(10.0);
    if let Some(error) = &app.color.error {
        details = details.child(notice(k, &BULB, error, k.c.danger));
    }
    details = details
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(swatch(k, rgb_color(preview), 64.0))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(2.0)
                        .child(
                            text(preview.hex())
                                .font(TextStyle::new(20.0, 600.0).lh(24.0))
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
                        ),
                ),
        )
        .child(row().w_full().gap(8.0).center_y().child(hex).child(apply))
        .child(Divider::horizontal().build(k))
        .child(tab)
        .child(Divider::horizontal().build(k))
        .child(match app.color.tab {
            ColorTab::Hs => col()
                .w_full()
                .gap(8.0)
                .child(slider_block(
                    k,
                    "Hue",
                    format!("{:.0}°", app.color.hue),
                    app.color.hue / 360.0,
                    "hue",
                    1.0 / 360.0,
                    |app: &mut App, fraction| {
                        app.color.hue = fraction * 360.0;
                        app.update_color_from_hsb();
                    },
                ))
                .child(slider_block(
                    k,
                    "Saturation",
                    format!("{:.0}%", app.color.saturation),
                    app.color.saturation / 100.0,
                    "saturation",
                    1.0 / 100.0,
                    |app: &mut App, fraction| {
                        app.color.saturation = fraction * 100.0;
                        app.update_color_from_hsb();
                    },
                )),
            ColorTab::Rgb => col()
                .w_full()
                .gap(8.0)
                .child(channel_block(app, k, Channel::Red))
                .child(channel_block(app, k, Channel::Green))
                .child(channel_block(app, k, Channel::Blue)),
        });

    let panel = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Picker",
                Some("Preview live. Keep colour stores it; Back restores the previous light."),
                Vec::new(),
            ))
            .child(panel_body(details))
            .child(panel_footer(k, vec![done])),
    );

    let scroll = col()
        .key("colour")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(subpage_header(
            k,
            "Colour",
            "Preview live before keeping it.",
            back,
        ))
        .child(panel)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn slider_block(
    k: &Kit,
    label: &str,
    display: String,
    value: f32,
    key: &str,
    step: f32,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    col()
        .w_full()
        .gap(4.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .child(
                    text(label.to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(text(display).font(k.ty.mono).color(k.c.text_base)),
        )
        .child(slider(k, key, value, label, step, 400.0, on_change))
}

enum Channel {
    Red,
    Green,
    Blue,
}

fn channel_block(app: &App, k: &Kit, channel: Channel) -> El {
    let (value, label, key) = match channel {
        Channel::Red => (app.color.preview.red, "Red", "red"),
        Channel::Green => (app.color.preview.green, "Green", "green"),
        Channel::Blue => (app.color.preview.blue, "Blue", "blue"),
    };
    let index = match channel {
        Channel::Red => 0,
        Channel::Green => 1,
        Channel::Blue => 2,
    };
    slider_block(
        k,
        label,
        format!("{value}"),
        f32::from(value) / 255.0,
        key,
        1.0 / 255.0,
        move |app: &mut App, fraction| {
            let mut color = app.color.preview;
            let next = (fraction * 255.0).round() as u8;
            match index {
                0 => color.red = next,
                1 => color.green = next,
                _ => color.blue = next,
            }
            app.update_color_from_rgb(color);
        },
    )
}
