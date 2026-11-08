use rune::{El, ScrollSpec, TextStyle, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Menu, MenuItem, Switch,
};

use crate::app::App;
use crate::bulb::aggregate::{AggregatePower, AggregateState, LightQuality, light_quality_of};
use crate::bulb::model::{
    BulbState, Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN, brightness_from_fraction,
    fraction_of_brightness, fraction_of_temperature, temperature_from_fraction,
};
use crate::bulb::selection::Selection;
use crate::screen::color::kelvin_to_rgb;
use crate::ui::widget::{
    BULB, EDIT, MORE, PLUS, REFRESH, SliderSpec, TRASH, dot, dropdown_trigger, error_text,
    gradient_slider, notice, panel, panel_body, panel_header, rgb_color, segmented, slider, swatch,
    value_text,
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
        348.0,
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
        .control_size(ControlSize::ExtraLarge)
        .help("Add bulb")
        .on_press(|app: &mut App| app.open_add())
        .build(k);

    let single = matches!(app.selection, Some(Selection::Bulb(_)));
    let trigger = IconButton::new(&MORE)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::ExtraLarge)
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
        .gap(6.0)
        .center_y()
        .child(select)
        .child(add)
        .child(menu)
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
        aggregate.power
    } else if state.power == Some(true) {
        AggregatePower::On
    } else if state.power == Some(false) {
        AggregatePower::Off
    } else {
        AggregatePower::Unknown
    };
    let power_switch = Switch::new(power == AggregatePower::On)
        .control_size(ControlSize::Large)
        .label("Power")
        .on_toggle(|app: &mut App| app.toggle_power())
        .build(k);

    let temperature = if all {
        aggregate.temperature.map_or(4_000, |span| span.high)
    } else {
        state.temperature.unwrap_or(4_000)
    };
    let preview = if white {
        rgb_color(kelvin_to_rgb(temperature))
    } else if color {
        if all {
            match aggregate.color {
                Some(crate::bulb::aggregate::AggregateColor::Equal(color)) => rgb_color(color),
                _ => k.c.layer_03,
            }
        } else {
            state.color.map_or(k.c.layer_03, rgb_color)
        }
    } else {
        k.c.layer_03
    };
    let title = if white {
        white_name(temperature)
    } else if color {
        "Colour"
    } else if mixed {
        "Mixed light"
    } else {
        "Waiting for light"
    };
    let scope = if all {
        format!("{} bulbs", aggregate.total)
    } else {
        app.selected_bulb()
            .map_or_else(|| "No bulb".to_owned(), |bulb| bulb.address.clone())
    };
    let mut subtitle = if mixed {
        format!("{scope} · choose a mode")
    } else if unknown {
        format!("{scope} · no state yet")
    } else {
        format!("{scope} · {}", aggregate_power_word(power))
    };
    if all && aggregate.failed > 0 {
        subtitle.push_str(&format!(" · {} failed", aggregate.failed));
    }
    if sync_owned {
        subtitle.push_str(" · screen sync");
    }

    let hero = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(swatch(k, preview, 48.0))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(title.to_owned())
                        .font(TextStyle::new(18.0, 600.0).lh(23.0))
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(
                    text(subtitle)
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate(),
                ),
        );

    let mut controls = col().w_full().gap(14.0).child(hero).child(brightness_block(
        app, k, &aggregate, &state, all, sync_owned,
    ));
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
    controls = controls.child(if sync_owned { mode.inert() } else { mode });
    if white {
        controls = controls.child(temperature_block(
            app, k, &aggregate, &state, all, sync_owned,
        ));
    }
    if color {
        controls = controls.child(colour_tile(k, &aggregate, &state, all, sync_owned));
    }
    if sync_owned {
        controls = controls.inert().layer_opacity(0.55);
    }

    let mut body = col().w_full().gap(10.0).child(controls);
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
            .child(panel_header(k, "Light", None, vec![power_switch]))
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
        .gap(5.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .center_y()
                .child(
                    text("Brightness".to_owned())
                        .font(k.ty.small_medium)
                        .color(k.c.text_muted),
                )
                .child(
                    text(label)
                        .font(TextStyle::new(26.0, 600.0).lh(30.0))
                        .color(k.c.text_base),
                ),
        )
        .child(slider)
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
    let stop = |kelvin| {
        (
            fraction_of_temperature(kelvin),
            rgb_color(kelvin_to_rgb(kelvin)),
        )
    };
    let stops = vec![
        stop(TEMPERATURE_MIN),
        stop(3_400),
        stop(4_000),
        stop(5_000),
        stop(TEMPERATURE_MAX),
    ];
    let slider = gradient_slider(
        k,
        fraction_of_temperature(value),
        SliderSpec {
            key: "temperature",
            label: "Colour temperature",
            step: 100.0 / 3_800.0,
            width: 400.0,
        },
        stops,
        |app: &mut App, fraction| app.set_temperature(temperature_from_fraction(fraction)),
    );
    let slider = if disabled {
        slider.inert().layer_opacity(0.55)
    } else {
        slider
    };
    col()
        .w_full()
        .gap(5.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(8.0)
                .center_y()
                .child(
                    text("Warmth".to_owned())
                        .font(k.ty.small_medium)
                        .color(k.c.text_muted),
                )
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(swatch(k, rgb_color(kelvin_to_rgb(value)), 14.0))
                        .child(value_text(k, label)),
                ),
        )
        .child(slider)
        .child(
            row()
                .w_full()
                .justify_between()
                .child(
                    text("Warm".to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_faint),
                )
                .child(
                    text("Cool".to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_faint),
                ),
        )
}

fn colour_tile(
    k: &Kit,
    aggregate: &AggregateState,
    state: &BulbState,
    all: bool,
    disabled: bool,
) -> El {
    let (color, mixed) = if all {
        match aggregate.color {
            Some(crate::bulb::aggregate::AggregateColor::Equal(color)) => (Some(color), false),
            Some(crate::bulb::aggregate::AggregateColor::Mixed) => (None, true),
            None => (None, false),
        }
    } else {
        (state.color, false)
    };
    let label = color.map_or_else(
        || {
            if mixed {
                "Mixed".to_owned()
            } else {
                "No colour".to_owned()
            }
        },
        Rgb::hex,
    );
    let fill = color.map_or(k.c.layer_03, rgb_color);
    let mut tile = row()
        .w_full()
        .h(58.0)
        .padding(8.0)
        .gap(8.0)
        .center_y()
        .radius(k.radius.md)
        .fill(fill)
        .border(1.0, k.c.border_base)
        .role("button")
        .label("Colour")
        .a11y_value(label.clone());
    if disabled {
        tile = tile.inert();
    } else {
        tile = tile
            .focusable(true)
            .focus_ring(2.0, 1.0, k.c.border_focus)
            .hover(k.c.hover)
            .press(k.c.pressed)
            .on_press(|app: &mut App| app.open_color());
    }
    let pill = |content: El| {
        row()
            .h(30.0)
            .px(10.0)
            .gap(6.0)
            .center_y()
            .radius(15.0)
            .fill(k.c.bg_base.alpha(0.92))
            .border(1.0, k.c.border_base)
            .child(content)
    };
    tile = tile
        .child(pill(
            text(label)
                .font(k.ty.mono)
                .color(k.c.text_base)
                .a11y_hidden(),
        ))
        .child(row().grow(1.0));
    if !disabled {
        tile = tile.child(pill(
            row()
                .gap(5.0)
                .center_y()
                .child(
                    text("Choose".to_owned())
                        .font(k.ty.small_medium)
                        .color(k.c.text_base)
                        .a11y_hidden(),
                )
                .child(rune::icon(&EDIT, 15.0).color(k.c.text_muted).a11y_hidden()),
        ));
    }
    tile
}

fn white_name(kelvin: u16) -> &'static str {
    match kelvin {
        0..=3_300 => "Warm white",
        3_301..=4_500 => "Soft white",
        4_501..=5_500 => "Neutral white",
        _ => "Cool white",
    }
}

fn empty_state(_app: &App, k: &Kit) -> El {
    let add = Button::new("Add a bulb")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
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
