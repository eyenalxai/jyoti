use rune::{El, ScrollSpec, TextStyle, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Menu, MenuItem, Switch,
};

use crate::app::App;
use crate::bulb::aggregate::{AggregatePower, AggregateState, LightQuality, light_quality_of};
use crate::bulb::model::{
    BRIGHTNESS_MAX, BRIGHTNESS_MIN, BulbState, Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN,
    brightness_from_fraction, fraction_of_brightness, fraction_of_temperature,
    temperature_from_fraction,
};
use crate::bulb::selection::Selection;
use crate::screen::color::kelvin_to_rgb;
use crate::ui::widget::{
    BULB, EDIT, MORE, PLUS, REFRESH, TRASH, badge, dot, dropdown_trigger, error_text, notice,
    panel, panel_body, panel_header, rgb_color, segmented, slider, swatch, value_text,
};

pub fn content(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    if app.bulbs.is_empty() {
        return empty_state(app, k);
    }

    let scroll = col()
        .key("control")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(picker_row(app, k, layers))
        .child(summary(app, k))
        .child(light_panel(app, k))
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn picker_row(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let show_all = app.bulbs.len() >= 2;
    let aggregate = app.aggregate();
    let all = matches!(app.selection, Some(Selection::All));
    let current_tone = if all {
        power_tone(k, aggregate_power_word(aggregate.power))
    } else {
        power_tone(k, state_power_word(&app.single_state()))
    };
    let current = selection_label(app);
    let trigger = dropdown_trigger(
        k,
        "Bulb",
        &current,
        app.bulb_picker_open,
        Some(current_tone),
        |app: &mut App| app.bulb_picker_open = !app.bulb_picker_open,
    );
    let mut picker = Menu::new("bulb-picker")
        .trigger(trigger)
        .open(app.bulb_picker_open)
        .min_width(360.0)
        .on_close(|app: &mut App| app.bulb_picker_open = false);
    if show_all {
        picker = picker.item(
            MenuItem::new("All bulbs")
                .checked(app.selection == Some(Selection::All))
                .lead(dot(power_tone(k, aggregate_power_word(aggregate.power))))
                .description(format!(
                    "{} configured · {} reachable",
                    aggregate.total, aggregate.reachable
                ))
                .on_select(|app: &mut App| {
                    app.bulb_picker_open = false;
                    app.select_all();
                }),
        );
    }
    for bulb in &app.bulbs {
        let state = app.states.get(&bulb.id);
        let tone = if app.errors.contains_key(&bulb.id) || app.sync.unreachable.contains(&bulb.id) {
            k.c.danger.icon
        } else {
            match state.and_then(|state| state.power) {
                Some(true) => k.c.success.icon,
                Some(false) => k.c.text_faint,
                None => k.c.warning.icon,
            }
        };
        let power = if app.errors.contains_key(&bulb.id) {
            "Error"
        } else if app.sync.unreachable.contains(&bulb.id) {
            "Unreachable"
        } else {
            match state.and_then(|state| state.power) {
                Some(true) => "On",
                Some(false) => "Off",
                None => "Connecting",
            }
        };
        let selected = app.selection == Some(Selection::Bulb(bulb.id.clone()));
        let id = bulb.id.clone();
        picker = picker.item(
            MenuItem::new(bulb.name.clone())
                .checked(selected)
                .lead(dot(tone))
                .description(format!("{} · {}", bulb.address, power))
                .on_select(move |app: &mut App| {
                    app.bulb_picker_open = false;
                    app.select_bulb(&id);
                }),
        );
    }
    let select = picker.build(k, layers);

    let add = IconButton::new(&PLUS)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Regular)
        .help("Add bulb")
        .on_press(|app: &mut App| app.open_add())
        .build(k);

    let single = matches!(app.selection, Some(Selection::Bulb(_)));
    let trigger = IconButton::new(&MORE)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Regular)
        .help("Bulb actions")
        .on_press(|app: &mut App| app.bulb_menu_open = !app.bulb_menu_open)
        .build(k);
    let menu =
        Menu::new("bulb-actions")
            .trigger(trigger)
            .open(app.bulb_menu_open)
            .on_close(|app: &mut App| app.bulb_menu_open = false)
            .item(MenuItem::new("Refresh").icon(&REFRESH).on_select_tx(
                |app: &mut App, tx: &mut Tx| {
                    app.bulb_menu_open = false;
                    app.refresh_all(tx);
                },
            ))
            .item(
                MenuItem::new("Edit bulb")
                    .icon(&EDIT)
                    .disabled(!single)
                    .on_select(|app: &mut App| {
                        app.bulb_menu_open = false;
                        app.edit_selected();
                    }),
            )
            .separator()
            .item(
                MenuItem::new("Remove bulb")
                    .icon(&TRASH)
                    .danger(true)
                    .disabled(!single)
                    .on_select(|app: &mut App| {
                        app.bulb_menu_open = false;
                        if let Some(Selection::Bulb(id)) = app.selection.clone() {
                            app.open_remove(&id);
                        }
                    }),
            )
            .build(k, layers);

    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(select)
        .child(add)
        .child(menu)
}

fn summary(app: &App, k: &Kit) -> El {
    let all = matches!(app.selection, Some(Selection::All));
    let aggregate = app.aggregate();
    let state = app.single_state();
    let (power, tone) = if all {
        let power = aggregate_power_word(aggregate.power);
        (power, power_tone(k, power))
    } else {
        let power = state_power_word(&state);
        (power, power_tone(k, power))
    };
    let detail = if all {
        if aggregate.failed == 0 {
            format!(
                "{} configured · {} reachable",
                aggregate.total, aggregate.reachable
            )
        } else {
            format!(
                "{} configured · {} reachable · {} failed",
                aggregate.total, aggregate.reachable, aggregate.failed
            )
        }
    } else {
        app.selected_bulb()
            .map_or_else(|| "Select a bulb".to_owned(), |bulb| bulb.address.clone())
    };
    let mut block = col().w_full().gap(8.0);
    block = block.child(
        row()
            .w_full()
            .gap(8.0)
            .center_y()
            .child(badge(k, power, tone))
            .child(
                text(detail)
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .truncate()
                    .grow(1.0),
            ),
    );
    if let Some(error) = app
        .selection
        .as_ref()
        .and_then(|selection| match selection {
            Selection::Bulb(id) => app.errors.get(id).cloned(),
            Selection::All => app.errors.values().next().cloned(),
        })
    {
        block = block.child(error_text(k, &error));
    }
    block
}

fn light_panel(app: &App, k: &Kit) -> El {
    let all = matches!(app.selection, Some(Selection::All));
    let aggregate = app.aggregate();
    let state = app.single_state();
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
    let power = if all {
        aggregate.power == AggregatePower::On
    } else {
        state.power == Some(true)
    };
    let power_switch = Switch::new(power)
        .control_size(ControlSize::Small)
        .label("Power")
        .on_toggle(|app: &mut App| app.toggle_power())
        .build(k);
    let actions = vec![power_switch];

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
        400.0,
        choose_mode,
    );
    let mode = if sync_owned { mode.inert() } else { mode };

    let mut controls = col().w_full().gap(10.0);
    controls = controls.child(
        col()
            .w_full()
            .gap(5.0)
            .child(
                text("Mode".to_owned())
                    .font(k.ty.ui_medium)
                    .color(k.c.text_base),
            )
            .child(mode),
    );
    controls = controls.child(brightness_block(
        app, k, &aggregate, &state, all, sync_owned,
    ));
    if white {
        controls = controls.child(temperature_block(
            app, k, &aggregate, &state, all, sync_owned,
        ));
    }
    if color {
        controls = controls.child(colour_block(app, k, &aggregate, &state, all, sync_owned));
        controls = controls.child(intensity_block(app, k, sync_owned));
    }
    if sync_owned {
        controls = controls.inert().layer_opacity(0.55);
    }

    let mut body = col().w_full().gap(10.0).child(controls);
    if mixed || unknown {
        body = body.child(notice(
            k,
            &BULB,
            if mixed {
                "The selected bulbs disagree on the mode. Choose White or Colour to apply one mode to all of them."
            } else {
                "Waiting for the bulb to report its light state."
            },
            if mixed { k.c.warning } else { k.c.info },
        ));
    }
    if sync_owned {
        body = body.child(notice(
            k,
            &crate::ui::widget::SYNC,
            "Screen sync is driving colour, brightness and warmth. Only power stays available here.",
            k.c.info,
        ));
    }

    panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Light",
                Some(if all {
                    "Applied to every selected bulb."
                } else {
                    "Applied to this bulb."
                }),
                actions,
            ))
            .child(panel_body(body)),
    )
}

fn brightness_block(
    _app: &App,
    k: &Kit,
    aggregate: &AggregateState,
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
    let slider = slider(
        k,
        "brightness",
        fraction_of_brightness(value),
        "Brightness",
        1.0 / 99.0,
        400.0,
        |app: &mut App, fraction| app.set_brightness(brightness_from_fraction(fraction)),
    );
    let slider = if disabled {
        slider.inert().layer_opacity(0.55)
    } else {
        slider
    };
    col()
        .w_full()
        .gap(4.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .center_y()
                .child(
                    text("Brightness".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(value_text(k, label)),
        )
        .child(slider)
        .child(
            text(format!(
                "{BRIGHTNESS_MIN}–{BRIGHTNESS_MAX}% across the selected bulbs."
            ))
            .font(k.ty.small)
            .color(k.c.text_muted)
            .wrap(),
        )
}

fn temperature_block(
    _app: &App,
    k: &Kit,
    aggregate: &AggregateState,
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
    let slider = slider(
        k,
        "temperature",
        fraction_of_temperature(value),
        "Colour temperature",
        100.0 / 3_800.0,
        400.0,
        |app: &mut App, fraction| app.set_temperature(temperature_from_fraction(fraction)),
    );
    let slider = if disabled {
        slider.inert().layer_opacity(0.55)
    } else {
        slider
    };
    col()
        .w_full()
        .gap(4.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .center_y()
                .child(
                    text("Warmth".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(swatch(k, rgb_color(kelvin_to_rgb(value)), 16.0))
                        .child(value_text(k, label)),
                ),
        )
        .child(slider)
        .child(
            text(format!(
                "{TEMPERATURE_MIN}–{TEMPERATURE_MAX} K for the White side."
            ))
            .font(k.ty.small)
            .color(k.c.text_muted),
        )
}

fn colour_block(
    _app: &App,
    k: &Kit,
    aggregate: &AggregateState,
    state: &BulbState,
    all: bool,
    disabled: bool,
) -> El {
    let color = if all {
        match aggregate.color {
            Some(crate::bulb::aggregate::AggregateColor::Equal(color)) => Some(color),
            _ => None,
        }
    } else {
        state.color
    };
    let label = color.map_or_else(|| "Mixed".to_owned(), Rgb::hex);
    let mut row = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(swatch(k, color.map_or(k.c.layer_03, rgb_color), 26.0))
        .child(
            text(label)
                .font(k.ty.mono)
                .color(k.c.text_base)
                .truncate()
                .grow(1.0),
        );
    if !disabled {
        row = row.child(
            Button::new("Pick")
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Small)
                .on_press(|app: &mut App| app.open_color())
                .build(k),
        );
    } else {
        row = row.child(badge(k, "Sync owns", k.c.warning.icon));
    }
    col()
        .w_full()
        .gap(5.0)
        .child(
            text("Colour".to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(row)
}

fn intensity_block(app: &App, k: &Kit, disabled: bool) -> El {
    let value = app.colour_intensity().unwrap_or(0.0).clamp(0.0, 100.0);
    let slider = slider(
        k,
        "colour-intensity",
        value / 100.0,
        "Colour intensity",
        1.0 / 100.0,
        400.0,
        |app: &mut App, fraction| app.set_colour_intensity(fraction * 100.0),
    );
    let slider = if disabled {
        slider.inert().layer_opacity(0.55)
    } else {
        slider
    };
    col()
        .w_full()
        .gap(4.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .child(
                    text("Colour intensity".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(value_text(k, format!("{value:.0}%"))),
        )
        .child(slider)
        .child(
            text("0% is grey; 100% is the full hue. Brightness still controls depth.".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        )
}

fn empty_state(_app: &App, k: &Kit) -> El {
    let add = Button::new("Add a bulb")
        .variant(ButtonVariant::Contrast)
        .on_press(|app: &mut App| app.open_add())
        .build(k);
    let body = col()
        .w_full()
        .padding(18.0)
        .gap(9.0)
        .child(rune::icon(&BULB, 28.0).color(k.c.text_faint))
        .child(
            text("No bulbs yet".to_owned())
                .font(TextStyle::new(18.0, 600.0).lh(22.0))
                .color(k.c.text_base),
        )
        .child(
            text("Add a Xiaomi bulb with its IP address and token. Everything stays on this machine."
                .to_owned())
            .font(k.ty.small)
            .color(k.c.text_muted)
            .wrap(),
        )
        .child(add);
    col()
        .key("empty")
        .w_full()
        .h_full()
        .center()
        .padding(16.0)
        .child(col().w_full().max_w(360.0).child(panel(k, body)))
}

fn choose_mode(app: &mut App, index: usize) {
    if index == 0 {
        app.set_mode_white();
    } else {
        app.set_mode_color();
    }
}

fn selection_label(app: &App) -> String {
    match &app.selection {
        Some(Selection::All) => "All bulbs".to_owned(),
        Some(Selection::Bulb(id)) => app
            .bulbs
            .iter()
            .find(|bulb| &bulb.id == id)
            .map_or_else(|| "Select a bulb".to_owned(), |bulb| bulb.name.clone()),
        None => "Select a bulb".to_owned(),
    }
}

fn aggregate_power_word(power: AggregatePower) -> &'static str {
    match power {
        AggregatePower::On => "On",
        AggregatePower::Off => "Off",
        AggregatePower::Mixed => "Mixed",
        AggregatePower::Unknown => "Unknown",
    }
}

fn state_power_word(state: &BulbState) -> &'static str {
    match state.power {
        Some(true) => "On",
        Some(false) => "Off",
        None => "Unknown",
    }
}

fn power_tone(k: &Kit, power: &str) -> rune::Color {
    match power {
        "On" => k.c.success.icon,
        "Mixed" => k.c.warning.icon,
        "Off" => k.c.text_faint,
        _ => k.c.text_faint,
    }
}
