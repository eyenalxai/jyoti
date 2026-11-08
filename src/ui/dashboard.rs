use rune::{El, ScrollSpec, col, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Switch};

use crate::app::App;
use crate::bulb::aggregate::{AggregateColor, AggregatePower, LightQuality, light_quality_of};
use crate::bulb::model::{
    BRIGHTNESS_MAX, BRIGHTNESS_MIN, BulbState, Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN,
    brightness_from_fraction, fraction_of_brightness, fraction_of_temperature,
    temperature_from_fraction,
};
use crate::bulb::selection::Selection;
use crate::screen::color::kelvin_to_rgb;
use crate::screen::sync::SyncStatus;
use crate::ui::widget::{
    BULB, EDIT, SYNC, TRASH, badge, dot, error_text, rgb_color, section, segmented, slider, swatch,
    sync_status_tone, value_text,
};

pub fn content(app: &App, k: &Kit) -> El {
    if app.bulbs.is_empty() {
        return empty_state(app, k);
    }

    let mut body = col().key("dashboard").w_full().padding(24.0).gap(16.0);
    body = body.child(hero(app, k));
    if !matches!(app.sync.status, SyncStatus::Idle) {
        body = body.child(sync_strip(app, k));
    }
    body = body.child(controls(app, k));
    body = body.child(status_card(app, k));
    let body = body
        .max_w(1100.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    row().w_full().h_full().justify_center().child(body)
}

fn hero(app: &App, k: &Kit) -> El {
    let name = app.selection_label();
    let (subtitle, power, can_edit) = match &app.selection {
        Some(Selection::All) => {
            let aggregate = app.aggregate();
            (
                format!(
                    "{} bulbs · {} reachable",
                    aggregate.total, aggregate.reachable
                ),
                aggregate.power == AggregatePower::On,
                false,
            )
        }
        Some(Selection::Bulb(id)) => {
            let bulb = app.bulbs.iter().find(|bulb| &bulb.id == id);
            let state = app.states.get(id);
            let subtitle = match bulb {
                Some(bulb) => format!(
                    "{} · {}",
                    bulb.address,
                    state_word(state).unwrap_or_else(|| "connecting".to_owned())
                ),
                None => "No bulb selected".to_owned(),
            };
            (
                subtitle,
                state.and_then(|state| state.power) == Some(true),
                true,
            )
        }
        None => ("No bulb selected".to_owned(), false, false),
    };

    let edit = IconButton::new(&EDIT)
        .variant(IconButtonVariant::GhostMuted)
        .help("Edit bulb")
        .disabled(!can_edit)
        .on_press(|app: &mut App| app.edit_selected())
        .build(k)
        .key("hero-edit");
    let remove = {
        let id = match &app.selection {
            Some(Selection::Bulb(id)) => Some(id.clone()),
            _ => None,
        };
        IconButton::new(&TRASH)
            .variant(IconButtonVariant::GhostMuted)
            .help("Remove bulb")
            .disabled(id.is_none())
            .on_press(move |app: &mut App| {
                if let Some(id) = &id {
                    app.open_remove(id);
                }
            })
            .build(k)
            .key("hero-remove")
    };

    row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            col()
                .grow(1.0)
                .gap(3.0)
                .child(text(name).font(k.ty.body_medium).color(k.c.text_base))
                .child(text(subtitle).font(k.ty.small).color(k.c.text_muted)),
        )
        .child(
            Switch::new(power)
                .control_size(ControlSize::Regular)
                .label("Power")
                .on_toggle(|app: &mut App| app.toggle_power())
                .build(k),
        )
        .child(edit)
        .child(remove)
}

fn controls(app: &App, k: &Kit) -> El {
    let state = app.single_state();
    let aggregate = app.aggregate();
    let all = matches!(app.selection, Some(Selection::All));
    let quality = if all {
        aggregate.quality
    } else {
        light_quality_of(state.mode)
    };
    let white = quality == LightQuality::White;
    let color = quality == LightQuality::Color;
    let mixed = quality == LightQuality::Mixed;
    let unknown = quality == LightQuality::Unknown;
    let sync_owned = app.sync.status.is_busy();

    let mut body = col().w_full().gap(16.0);
    if sync_owned {
        body = body.child(
            row()
                .w_full()
                .padding(10.0)
                .gap(10.0)
                .center_y()
                .radius(k.radius.md)
                .fill(k.c.warning.bg)
                .child(icon(&SYNC, 16.0).color(k.c.warning.icon))
                .child(
                    text(
                        "Screen sync owns colour, brightness and temperature while it runs."
                            .to_owned(),
                    )
                    .font(k.ty.small)
                    .color(k.c.warning.fg)
                    .wrap(),
                ),
        );
    }

    let selected_mode = if white {
        Some(0)
    } else if color {
        Some(1)
    } else {
        None
    };
    let mode = segmented(
        k,
        &["White", "Colour"],
        selected_mode,
        "Light mode",
        300.0,
        choose_mode,
    );
    let mut light = col().gap(16.0).child(mode);
    if mixed {
        light = light.child(
            text("The selected bulbs disagree on the mode. Choose White or Colour to apply one to every bulb.".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        );
    } else if unknown {
        light = light.child(
            text("Waiting for the bulb's light state.".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    } else {
        light = light.child(brightness_row(app, k, &aggregate, &state, all, sync_owned));
        if white {
            light = light.child(temperature_row(app, k, &aggregate, &state, all, sync_owned));
        }
        if color {
            light = light.child(colour_row(app, k, &aggregate, &state, all, sync_owned));
        }
    }
    body = body.child(section(k, "Light", light));

    body
}

fn choose_mode(app: &mut App, index: usize) {
    if index == 0 {
        app.set_mode_white();
    } else {
        app.set_mode_color();
    }
}

fn brightness_row(
    _app: &App,
    k: &Kit,
    aggregate: &crate::bulb::aggregate::AggregateState,
    state: &BulbState,
    all: bool,
    disabled: bool,
) -> El {
    let value = if all {
        aggregate.brightness.map_or(50, |span| span.high)
    } else {
        state.brightness.unwrap_or(50)
    };
    let label = if all {
        match aggregate.brightness {
            Some(span) if span.low != span.high => format!("{}–{}%", span.low, span.high),
            _ => format!("{value}%"),
        }
    } else {
        format!("{value}%")
    };
    row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            text("Brightness".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w(86.0),
        )
        .child({
            let control = slider(
                k,
                "brightness",
                fraction_of_brightness(value),
                "Brightness",
                1.0 / 99.0,
                |app: &mut App, fraction| app.set_brightness(brightness_from_fraction(fraction)),
            );
            if disabled { control.inert() } else { control }
        })
        .child(text(label).font(k.ty.mono).color(k.c.text_base).w(62.0))
        .child(
            text(format!("{}–{}%", BRIGHTNESS_MIN, BRIGHTNESS_MAX))
                .font(k.ty.small)
                .color(k.c.text_faint),
        )
}

fn temperature_row(
    _app: &App,
    k: &Kit,
    aggregate: &crate::bulb::aggregate::AggregateState,
    state: &BulbState,
    all: bool,
    disabled: bool,
) -> El {
    let value = if all {
        aggregate.temperature.map_or(4_000, |span| span.high)
    } else {
        state.temperature.unwrap_or(4_000)
    };
    let label = if all {
        match aggregate.temperature {
            Some(span) if span.low != span.high => format!("{}–{} K", span.low, span.high),
            _ => format!("{value} K"),
        }
    } else {
        format!("{value} K")
    };
    row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            text("Warmth".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w(86.0),
        )
        .child({
            let control = slider(
                k,
                "temperature",
                fraction_of_temperature(value),
                "Colour temperature",
                100.0 / 3_800.0,
                |app: &mut App, fraction| app.set_temperature(temperature_from_fraction(fraction)),
            );
            if disabled { control.inert() } else { control }
        })
        .child(text(label).font(k.ty.mono).color(k.c.text_base).w(86.0))
        .child(
            text(format!("{TEMPERATURE_MIN}–{TEMPERATURE_MAX} K"))
                .font(k.ty.small)
                .color(k.c.text_faint),
        )
        .child(swatch(k, rgb_color(kelvin_to_rgb(value)), 22.0))
}

fn colour_row(
    _app: &App,
    k: &Kit,
    aggregate: &crate::bulb::aggregate::AggregateState,
    state: &BulbState,
    all: bool,
    disabled: bool,
) -> El {
    let color = if all {
        match aggregate.color {
            Some(AggregateColor::Equal(color)) => Some(color),
            _ => None,
        }
    } else {
        state.color
    };
    let label = color.map_or_else(|| "Mixed".to_owned(), Rgb::hex);
    let mut control =
        row()
            .gap(10.0)
            .center_y()
            .child(swatch(k, color.map_or(k.c.layer_03, rgb_color), 28.0));
    control = control.child(text(label).font(k.ty.mono).color(k.c.text_base));
    if disabled {
        control.child(badge(k, "Sync owns this", k.c.warning.icon))
    } else {
        control.child(
            Button::new("Choose colour")
                .variant(ButtonVariant::Outline)
                .on_press(|app: &mut App| app.open_color())
                .build(k),
        )
    }
}

fn status_card(app: &App, k: &Kit) -> El {
    let aggregate = app.aggregate();
    let all = matches!(app.selection, Some(Selection::All));
    let state = app.single_state();
    let mut body = col().gap(8.0);

    if all {
        body = body
            .child(status_row(k, "Bulbs", format!("{}", aggregate.total)))
            .child(status_row(
                k,
                "Reachable",
                format!("{}", aggregate.reachable),
            ))
            .child(status_row(k, "Failed", format!("{}", aggregate.failed)))
            .child(status_row(k, "Mode", quality_word(aggregate.quality)))
            .child(status_row(
                k,
                "Brightness",
                span_word(aggregate.brightness.map(|span| (span.low, span.high)), "%"),
            ))
            .child(status_row(
                k,
                "Temperature",
                span_word(
                    aggregate.temperature.map(|span| (span.low, span.high)),
                    " K",
                ),
            ))
            .child(status_row(
                k,
                "Colour",
                match aggregate.color {
                    Some(AggregateColor::Equal(color)) => color.hex(),
                    Some(AggregateColor::Mixed) => "Mixed".to_owned(),
                    None => "Unknown".to_owned(),
                },
            ));
    } else {
        if let Some(bulb) = app.selected_bulb() {
            body = body
                .child(status_row(k, "Address", bulb.address.clone()))
                .child(status_row(
                    k,
                    "Model",
                    bulb.model.clone().unwrap_or_else(|| "Unknown".to_owned()),
                ))
                .child(status_row(
                    k,
                    "Power",
                    state.power.map_or("Unknown", power_word).to_owned(),
                ))
                .child(status_row(
                    k,
                    "Mode",
                    quality_word(light_quality_of(state.mode)),
                ))
                .child(status_row(
                    k,
                    "Brightness",
                    state
                        .brightness
                        .map_or_else(|| "Unknown".to_owned(), |v| format!("{v}%")),
                ))
                .child(status_row(
                    k,
                    "Temperature",
                    state
                        .temperature
                        .map_or_else(|| "Unknown".to_owned(), |v| format!("{v} K")),
                ))
                .child(status_row(
                    k,
                    "Colour",
                    state.color.map_or_else(|| "Unknown".to_owned(), Rgb::hex),
                ));
        }
    }

    if let Some(error) = app
        .selection
        .as_ref()
        .and_then(|selection| match selection {
            Selection::Bulb(id) => app.errors.get(id).cloned(),
            Selection::All => app.errors.values().next().cloned(),
        })
    {
        body = body.child(error_text(k, &error));
    }

    section(k, "Status", body)
}

fn status_row(k: &Kit, label: &str, value: String) -> El {
    row()
        .w_full()
        .justify_between()
        .gap(16.0)
        .child(
            text(label.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(value_text(k, value))
}

fn sync_strip(app: &App, k: &Kit) -> El {
    let tone = sync_status_tone(k, &app.sync.status);
    let mut row = row()
        .w_full()
        .padding(12.0)
        .gap(10.0)
        .center_y()
        .radius(k.radius.md)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_weak)
        .child(dot(tone))
        .child(
            text("Screen sync".to_owned())
                .font(k.ty.small_medium)
                .color(k.c.text_base),
        )
        .child(
            text(app.sync.status.label())
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    if let Some(color) = app.sync.derived {
        row = row.child(swatch(k, rgb_color(color), 18.0)).child(
            text(format!("{}%", app.sync.brightness))
                .font(k.ty.mono)
                .color(k.c.text_base),
        );
    }
    if app.sync.stale {
        row = row.child(badge(k, "Stale", k.c.warning.icon));
    }
    row.child(
        Button::new("Open")
            .variant(ButtonVariant::Ghost)
            .on_press(|app: &mut App| app.open_sync())
            .build(k),
    )
}

fn empty_state(_app: &App, k: &Kit) -> El {
    let add = Button::new("Add your first bulb")
        .variant(ButtonVariant::Contrast)
        .on_press(|app: &mut App| app.open_add())
        .build(k);
    col()
        .key("empty")
        .w_full()
        .h_full()
        .center()
        .gap(14.0)
        .fill(k.c.page)
        .child(icon(&BULB, 44.0).color(k.c.text_faint))
        .child(
            text("No bulbs yet".to_owned())
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            text(
                "Add a Xiaomi bulb and its token. Jyoti keeps the token in the OS keyring."
                    .to_owned(),
            )
            .font(k.ty.small)
            .color(k.c.text_muted)
            .wrap()
            .max_w(360.0),
        )
        .child(add)
}

fn state_word(state: Option<&BulbState>) -> Option<String> {
    let state = state?;
    Some(match state.power {
        Some(true) => "on".to_owned(),
        Some(false) => "off".to_owned(),
        None => "state unknown".to_owned(),
    })
}

fn power_word(power: bool) -> &'static str {
    if power { "On" } else { "Off" }
}

fn quality_word(quality: LightQuality) -> String {
    match quality {
        LightQuality::White => "White".to_owned(),
        LightQuality::Color => "Colour".to_owned(),
        LightQuality::Mixed => "Mixed".to_owned(),
        LightQuality::Unknown => "Unknown".to_owned(),
    }
}

fn span_word<T: Copy + PartialEq + std::fmt::Display>(
    span: Option<(T, T)>,
    suffix: &str,
) -> String {
    match span {
        Some((low, high)) if low == high => format!("{low}{suffix}"),
        Some((low, high)) => format!("{low}–{high}{suffix}"),
        None => "Unknown".to_owned(),
    }
}
