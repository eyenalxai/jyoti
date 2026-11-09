use std::time::Duration;

use rune::{Color, El, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, TextInput};

use crate::app::App;
use crate::app::state::ColorSession;
use crate::screen::color::{Hsb, hsb_to_rgb};
use crate::ui::theme;
use crate::ui::widget::{ALERT, SliderSpec, gradient_slider, hairline, notice, rgb_color};

const POPOVER_WIDTH: f32 = 320.0;
const POPOVER_SLIDER_WIDTH: f32 = 272.0;

pub fn popover(session: &ColorSession, k: &Kit) -> El {
    let hex = TextInput::new(&session.hex)
        .label("Hex colour")
        .placeholder("#FF8A00")
        .width(None)
        .on_edit(|app: &mut App| app.color.hex_mut())
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
    let cancel = Button::new("Cancel")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Regular)
        .on_press(|app: &mut App| app.cancel_color())
        .build(k)
        .key("cancel-colour")
        .h(40.0);
    let keep = Button::new("Keep colour")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Regular)
        .on_press(|app: &mut App| app.commit_color())
        .build(k)
        .key("keep-colour")
        .h(40.0);

    let header = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            text("Colour".to_owned())
                .font(k.ty.body_medium)
                .color(k.c.text_base)
                .grow(1.0)
                .min_w(0.0)
                .truncate(),
        )
        .child(cancel)
        .child(keep);

    let mut body = col()
        .w(POPOVER_WIDTH)
        .padding(12.0)
        .gap(8.0)
        .child(header)
        .child(
            text(format!(
                "Applying to {} {}",
                session.targets.len(),
                if session.targets.len() == 1 {
                    "bulb"
                } else {
                    "bulbs"
                }
            ))
            .font(k.ty.small)
            .color(k.c.text_muted),
        )
        .child(hairline(k))
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(row().grow(1.0).min_w(0.0).child(hex))
                .child(apply),
        );
    if let Some(error) = &session.error {
        body = body.child(notice(k, &ALERT, error, k.c.danger));
    }
    let hsb = Hsb {
        hue: session.hue,
        saturation: session.saturation,
        value: session.value,
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
    body = body
        .child(tune_block(
            k,
            SliderSpec {
                label: "Hue",
                key: "hue",
                step: 1.0 / 360.0,
                width: POPOVER_SLIDER_WIDTH,
            },
            format!("{:.0}°", session.hue),
            session.hue / 360.0,
            hue_stops,
            |app: &mut App, fraction| {
                app.set_color_hue(fraction * 360.0);
            },
        ))
        .child(tune_block(
            k,
            SliderSpec {
                label: "Saturation",
                key: "saturation",
                step: 1.0 / 100.0,
                width: POPOVER_SLIDER_WIDTH,
            },
            format!("{:.0}%", session.saturation),
            session.saturation / 100.0,
            saturation_stops,
            |app: &mut App, fraction| {
                app.set_color_saturation(fraction * 100.0);
            },
        ))
        .child(tune_block(
            k,
            SliderSpec {
                label: "Brightness",
                key: "value",
                step: 1.0 / 100.0,
                width: POPOVER_SLIDER_WIDTH,
            },
            format!("{:.0}%", session.value),
            session.value / 100.0,
            brightness_stops,
            |app: &mut App, fraction| {
                app.set_color_value(fraction * 100.0);
            },
        ));

    body.role("none")
        .modal(true, |app: &mut App| app.cancel_color())
}

fn tune_block(
    k: &Kit,
    spec: SliderSpec<'_>,
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
                    text(spec.label.to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(display)
                        .font(theme::number(13.0, 560.0))
                        .color(k.c.text_muted),
                ),
        )
        .child(
            row()
                .w_full()
                .justify_center()
                .child(gradient_slider(k, value, spec, stops, on_change)),
        )
}
