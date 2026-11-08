use std::time::Duration;

use rune::{Color, El, ScrollSpec, TextStyle, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, TextInput};

use crate::app::App;
use crate::screen::color::{Hsb, hsb_to_rgb};
use crate::ui::widget::{
    BACK, BULB, SliderSpec, gradient_slider, notice, panel, panel_body, panel_footer, panel_header,
    rgb_color, subpage_header, swatch,
};

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

    let mut preview_body = col().w_full().gap(12.0).child(
        row()
            .w_full()
            .gap(14.0)
            .center_y()
            .child(swatch(k, rgb_color(preview), 76.0))
            .child(
                col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(2.0)
                    .child(
                        text(preview.hex())
                            .font(TextStyle::new(22.0, 600.0).lh(26.0))
                            .color(k.c.text_base),
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
                        .color(k.c.text_muted),
                    ),
            ),
    );
    if let Some(error) = &app.color.error {
        preview_body = preview_body.child(notice(k, &BULB, error, k.c.danger));
    }
    let preview_panel = panel(
        k,
        panel_body(preview_body.child(row().w_full().gap(8.0).center_y().child(hex).child(apply))),
    );

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

    let tune = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Tune",
                Some("Hue, saturation and brightness."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(14.0)
                    .child(tune_block(
                        k,
                        TuneSpec {
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
                        TuneSpec {
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
                        TuneSpec {
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
                    )),
            ))
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
            "Preview live. Back restores the previous light.",
            back,
        ))
        .child(preview_panel)
        .child(tune)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

struct TuneSpec<'a> {
    label: &'a str,
    key: &'a str,
    step: f32,
}

fn tune_block(
    k: &Kit,
    spec: TuneSpec<'_>,
    display: String,
    value: f32,
    stops: Vec<(f32, Color)>,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    col()
        .w_full()
        .gap(5.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .child(
                    text(spec.label.to_owned())
                        .font(k.ty.small_medium)
                        .color(k.c.text_muted),
                )
                .child(text(display).font(k.ty.mono).color(k.c.text_base)),
        )
        .child(gradient_slider(
            k,
            value,
            SliderSpec {
                key: spec.key,
                label: spec.label,
                step: spec.step,
                width: 400.0,
            },
            stops,
            on_change,
        ))
}
