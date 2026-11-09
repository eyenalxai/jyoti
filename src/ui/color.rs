use std::time::Duration;

use rune::{Color, El, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, TextInput};

use crate::app::App;
use crate::screen::color::{Hsb, hsb_to_rgb};
use crate::ui::theme;
use crate::ui::widget::{ALERT, SliderSpec, gradient_slider, hairline, notice, rgb_color, swatch};

const POPOVER_WIDTH: f32 = 296.0;
const POPOVER_SLIDER_WIDTH: f32 = 272.0;

pub fn popover(app: &App, k: &Kit) -> El {
    let preview = app.color.preview;
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
        .child(swatch(k, rgb_color(preview), 30.0))
        .child(
            text("Colour".to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0),
        )
        .child(cancel)
        .child(keep);

    let mut body = col()
        .w(POPOVER_WIDTH)
        .gap(8.0)
        .child(header)
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
    if let Some(error) = &app.color.error {
        body = body.child(notice(k, &ALERT, error, k.c.danger));
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
    body = body
        .child(tune_block(
            k,
            SliderSpec {
                label: "Hue",
                key: "hue",
                step: 1.0 / 360.0,
                width: POPOVER_SLIDER_WIDTH,
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
            SliderSpec {
                label: "Saturation",
                key: "saturation",
                step: 1.0 / 100.0,
                width: POPOVER_SLIDER_WIDTH,
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
            SliderSpec {
                label: "Brightness",
                key: "value",
                step: 1.0 / 100.0,
                width: POPOVER_SLIDER_WIDTH,
            },
            format!("{:.0}%", app.color.value),
            app.color.value / 100.0,
            brightness_stops,
            |app: &mut App, fraction| {
                app.color.value = fraction * 100.0;
                app.update_color_from_hsb();
            },
        ));

    body
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
