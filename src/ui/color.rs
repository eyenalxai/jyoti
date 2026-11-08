use std::time::Duration;

use rune::{Color, El, ScrollSpec, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, TextInput};

use crate::app::App;
use crate::screen::color::{Hsb, hsb_to_rgb};
use crate::ui::theme;
use crate::ui::widget::{
    ALERT, BACK, SliderSpec, gradient_slider, hairline, notice, rgb_color, subpage_header, surface,
};

const SLIDER_WIDTH: f32 = 400.0;

pub fn view(app: &App, k: &Kit) -> El {
    let preview = app.color.preview;
    let ink = theme::ink_on(rgb_color(preview));
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::ExtraLarge)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k)
        .size(40.0, 40.0)
        .radius(k.radius.md);
    let done = Button::new("Keep colour")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
        .on_press(|app: &mut App| app.commit_color())
        .build(k)
        .h(40.0);
    let hex = TextInput::new(&app.color.hex)
        .label("Hex colour")
        .placeholder("#FF8A00")
        .width(None)
        .on_edit(|app: &mut App| &mut app.color.hex)
        .on_submit(|app: &mut App, tx: &mut Tx| {
            app.update_color_from_hex();
            tx.wake_after(Duration::from_millis(160));
        })
        .build(k);
    let apply = Button::new("Apply")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .on_press(|app: &mut App| app.update_color_from_hex())
        .build(k)
        .h(40.0);

    let chip = col()
        .w_full()
        .h(190.0)
        .justify_end()
        .gap(2.0)
        .padding(16.0)
        .radius(k.radius.md)
        .clip_soft(k.radius.md)
        .fill(rgb_color(preview))
        .linear_gradient(
            std::f32::consts::PI,
            [
                (0.0, Color::WHITE.alpha(0.2)),
                (0.55, Color::WHITE.alpha(0.0)),
                (1.0, theme::ink().alpha(0.1)),
            ],
        )
        .border(1.0, theme::ink().alpha(0.08))
        .child(
            text(preview.hex())
                .font(theme::code(30.0, 500.0))
                .color(ink),
        )
        .child(
            text(format!(
                "Live preview · applying to {} {}",
                app.color.targets.len(),
                if app.color.targets.len() == 1 {
                    "bulb"
                } else {
                    "bulbs"
                }
            ))
            .font(k.ty.small)
            .color(ink.alpha(0.72)),
        );

    let mut tune = col().w_full().gap(9.0).child(
        row()
            .w_full()
            .gap(8.0)
            .center_y()
            .child(row().grow(1.0).min_w(0.0).child(hex))
            .child(apply),
    );
    if let Some(error) = &app.color.error {
        tune = tune.child(notice(k, &ALERT, error, k.c.danger));
    }
    let hsb = Hsb {
        hue: app.color.hue,
        saturation: app.color.saturation,
        value: app.color.value,
    };
    let hue_stops = (0..=6)
        .map(|step| {
            (
                step as f32 / 6.0,
                rgb_color(hsb_to_rgb(Hsb {
                    hue: step as f32 * 60.0,
                    saturation: 100.0,
                    value: 100.0,
                })),
            )
        })
        .collect();
    let saturation_stops = vec![
        (
            0.0,
            rgb_color(hsb_to_rgb(Hsb {
                saturation: 0.0,
                ..hsb
            })),
        ),
        (
            1.0,
            rgb_color(hsb_to_rgb(Hsb {
                saturation: 100.0,
                ..hsb
            })),
        ),
    ];
    let brightness_stops = vec![
        (0.0, rgb_color(hsb_to_rgb(Hsb { value: 0.0, ..hsb }))),
        (
            1.0,
            rgb_color(hsb_to_rgb(Hsb {
                value: 100.0,
                ..hsb
            })),
        ),
    ];
    let tune = tune
        .child(hairline(k))
        .child(tune_block(
            k,
            Tune {
                label: "Hue",
                key: "hue",
                step: 1.0 / 360.0,
            },
            format!("{:.0}°", app.color.hue),
            app.color.hue / 360.0,
            hue_stops,
            |app: &mut App, fraction| {
                app.color.hue = fraction * 360.0;
                app.update_color_from_hsb();
            },
        ))
        .child(tune_block(
            k,
            Tune {
                label: "Saturation",
                key: "saturation",
                step: 1.0 / 100.0,
            },
            format!("{:.0}%", app.color.saturation),
            app.color.saturation / 100.0,
            saturation_stops,
            |app: &mut App, fraction| {
                app.color.saturation = fraction * 100.0;
                app.update_color_from_hsb();
            },
        ))
        .child(tune_block(
            k,
            Tune {
                label: "Brightness",
                key: "value",
                step: 1.0 / 100.0,
            },
            format!("{:.0}%", app.color.value),
            app.color.value / 100.0,
            brightness_stops,
            |app: &mut App, fraction| {
                app.color.value = fraction * 100.0;
                app.update_color_from_hsb();
            },
        ));

    let body = surface(
        k,
        col()
            .w_full()
            .padding(14.0)
            .gap(12.0)
            .child(chip)
            .child(tune),
    );

    let viewport = col()
        .key("colour")
        .w_full()
        .padding(14.0)
        .gap(12.0)
        .child(subpage_header(
            k,
            "Colour",
            "Live preview. Back restores the previous light.",
            back,
        ))
        .child(body)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);
    let footer = col()
        .w_full()
        .child(hairline(k))
        .child(row().w_full().px(14.0).py(10.0).justify_end().child(done));

    col()
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(scroll)
        .child(footer)
}

struct Tune<'a> {
    label: &'a str,
    key: &'a str,
    step: f32,
}

fn tune_block(
    k: &Kit,
    tune: Tune<'_>,
    display: String,
    value: f32,
    stops: Vec<(f32, Color)>,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    col()
        .w_full()
        .gap(1.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .center_y()
                .child(
                    text(tune.label.to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(display)
                        .font(theme::number(13.0, 560.0))
                        .color(k.c.text_muted),
                ),
        )
        .child(row().w_full().justify_center().child(gradient_slider(
            k,
            value,
            SliderSpec {
                key: tune.key,
                label: tune.label,
                step: tune.step,
                width: SLIDER_WIDTH,
            },
            stops,
            on_change,
        )))
}
