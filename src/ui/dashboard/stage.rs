use rune::{Color, El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::app::App;
use crate::app::selected::SelectedLight;
use crate::bulb::aggregate::{AggregatePower, LightQuality};
use crate::screen::color::kelvin_to_rgb;
use crate::ui::theme;
use crate::ui::widget::{ALERT, BULB, PAGE_PAD, dot, nameplate, rgb_color, stage_surface};

use super::{STAGE_HEIGHT, health_word};

pub fn light_stage(view: &SelectedLight, k: &Kit) -> El {
    let white = view.quality == LightQuality::White;
    let colour = view.quality == LightQuality::Color;
    let mixed = view.quality == LightQuality::Mixed;
    let power = view.power;
    let dimmed = power == AggregatePower::Off;

    let temperature = view.temperature_value();
    let colour_value = view.color_value();
    let mixed_colour = view.color_mixed();
    let base = if white {
        Some(kelvin_to_rgb(temperature))
    } else if colour {
        colour_value
    } else {
        None
    };
    let mut fill = base.map_or(k.c.layer_02, rgb_color);
    if dimmed {
        fill = fill.mix(k.c.layer_03, 0.72);
    }

    let title = if white {
        white_name(temperature).to_owned()
    } else if colour {
        if mixed_colour {
            "Mixed colour".to_owned()
        } else {
            "Colour".to_owned()
        }
    } else if mixed {
        "Mixed light".to_owned()
    } else {
        "No light yet".to_owned()
    };
    let scope = view.scope_label();
    let detail = if white {
        view.temperature_label()
    } else if colour {
        if mixed_colour {
            "Colours differ".to_owned()
        } else {
            match colour_value {
                Some(value) => value.hex(),
                None => "No colour reported".to_owned(),
            }
        }
    } else if mixed {
        "Choose a mode".to_owned()
    } else {
        "Waiting for the bulb".to_owned()
    };

    let detail_style = if colour && !mixed_colour {
        k.ty.mono.lh(16.0)
    } else {
        theme::number(12.0, 450.0)
    };
    let brightness_text = view.brightness_text();
    let (status_ink, status_word) = match view.health.status {
        Some(status) => match health_word(status) {
            Some(word) => (theme::warning_on_plate(), word),
            None => stage_status(power),
        },
        None => stage_status(power),
    };
    let plate_ink = theme::ink();
    let plate_muted = plate_ink.alpha(0.62);
    let plate = nameplate(
        k,
        col()
            .w_full()
            .gap(8.0)
            .child(
                row()
                    .w_full()
                    .justify_between()
                    .gap(12.0)
                    .center_y()
                    .child(
                        row().gap(6.0).center_y().child(dot(status_ink)).child(
                            text(status_word.to_owned())
                                .font(k.ty.small_medium)
                                .color(plate_ink),
                        ),
                    )
                    .child(
                        row()
                            .grow(1.0)
                            .min_w(0.0)
                            .justify_end()
                            .child(text(scope).font(k.ty.small).color(plate_muted).truncate()),
                    ),
            )
            .child(
                row()
                    .w_full()
                    .items_end()
                    .justify_between()
                    .gap(10.0)
                    .child(
                        col()
                            .grow(1.0)
                            .min_w(0.0)
                            .gap(1.0)
                            .child(
                                text(title)
                                    .font(theme::display(27.0, 580.0))
                                    .color(plate_ink)
                                    .truncate(),
                            )
                            .child(
                                text(detail)
                                    .font(detail_style)
                                    .color(plate_muted)
                                    .truncate(),
                            ),
                    )
                    .child(
                        col()
                            .items_end()
                            .gap(0.0)
                            .child(text("BRIGHTNESS").font(theme::label()).color(plate_muted))
                            .child(
                                row()
                                    .items_end()
                                    .gap(1.0)
                                    .child(
                                        text(brightness_text)
                                            .font(theme::number(38.0, 610.0))
                                            .color(plate_ink),
                                    )
                                    .child(
                                        text("%")
                                            .font(theme::number(16.0, 560.0))
                                            .color(plate_muted),
                                    ),
                            ),
                    ),
            ),
    );

    let glow = if dimmed { 0.0 } else { 0.24 };
    let stage = stage_surface(k, STAGE_HEIGHT, fill, glow, plate);

    if view.is_all() && view.health.failed > 0 {
        stage.child(
            row()
                .absolute(12.0, 10.0)
                .gap(6.0)
                .padding(7.0)
                .radius(12.0)
                .fill(theme::ink().alpha(0.62))
                .center_y()
                .child(rune::icon(&ALERT, 13.0).color(theme::plate()).a11y_hidden())
                .child(
                    text(format!("{} unreachable", view.health.failed))
                        .font(k.ty.small_medium)
                        .color(theme::plate()),
                ),
        )
    } else {
        stage
    }
}

pub fn empty_state(k: &Kit) -> El {
    let add = Button::new("Add a bulb")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
        .on_press(|app: &mut App| app.open_add())
        .build(k)
        .h(40.0);
    let body = col()
        .w_full()
        .h(350.0)
        .justify_between()
        .padding(18.0)
        .radius(k.radius.xl)
        .clip_soft(k.radius.xl)
        .fill(k.c.layer_02)
        .linear_gradient(
            std::f32::consts::PI,
            [
                (0.0, k.c.bg_accent.alpha(0.26)),
                (0.6, k.c.bg_accent.alpha(0.02)),
            ],
        )
        .border(1.0, k.c.border_muted)
        .child(
            row()
                .w_full()
                .justify_center()
                .child(rune::icon(&BULB, 62.0).color(k.c.bg_accent.alpha(0.4)).a11y_hidden()),
        )
        .child(
            col()
                .w_full()
                .gap(10.0)
                .padding(18.0)
                .radius(k.radius.md)
                .fill(k.c.bg_base)
                .child(
                    text("No bulbs yet".to_owned())
                        .font(theme::display(28.0, 580.0))
                        .color(k.c.text_base),
                )
                .child(
                    text("Add the bulb’s IP address and its 32-character token. Both stay on this machine.".to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap(),
                )
                .child(add),
        );
    col()
        .key("empty")
        .w_full()
        .h_full()
        .center()
        .padding(PAGE_PAD)
        .child(col().w_full().max_w(420.0).child(body))
}

/// The stage plate always sits on a cream label, so its status colour is fixed
/// rather than taken from the window appearance.
fn stage_status(power: AggregatePower) -> (Color, &'static str) {
    match power {
        AggregatePower::On => (theme::positive_on_plate(), "On"),
        AggregatePower::Off => (theme::quiet_on_plate(), "Off"),
        AggregatePower::Mixed => (theme::warning_on_plate(), "Mixed"),
        AggregatePower::Unknown => (theme::quiet_on_plate(), "Connecting"),
    }
}

fn white_name(kelvin: u16) -> &'static str {
    match kelvin {
        0..=3_300 => "Warm white",
        3_301..=4_500 => "Soft white",
        4_501..=5_500 => "Neutral white",
        _ => "Cool white",
    }
}
