mod stage;

use rune::{Color, Edge, El, ScrollSpec, Spring, SpringPair, Transition, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Menu, MenuItem,
    Placement, Popover, Switch,
};

use crate::app::App;
use crate::bulb::aggregate::{AggregatePower, AggregateState, LightQuality, light_quality_of};
use crate::bulb::health::HealthStatus;
use crate::bulb::model::{
    BulbState, Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN, brightness_from_fraction,
    fraction_of_brightness, fraction_of_temperature, temperature_from_fraction,
};
use crate::bulb::selection::Selection;
use crate::screen::color::kelvin_to_rgb;
use crate::ui::color;
use crate::ui::theme;
use crate::ui::widget::{
    ALERT, BULB, EDIT, MORE, PAGE_PAD, PLUS, REFRESH, SLIDER_WIDTH, SliderSpec, TRASH, dim_when,
    dot, dropdown_trigger, error_text, gradient_slider, hairline, notice, rgb_color,
    segmented_switch, slider, surface, swatch,
};

const STAGE_HEIGHT: f32 = 202.0;

pub fn content(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    if app.bulbs.is_empty() {
        return stage::empty_state(k);
    }

    let viewport = col()
        .key("control")
        .w_full()
        .padding(PAGE_PAD)
        .gap(9.0)
        .child(picker_row(app, k, layers))
        .child(stage::light_stage(app, k))
        .child(control_deck(app, k, layers))
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn picker_row(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let show_all = app.bulbs.len() >= 2;
    let aggregate = app.aggregate();
    let all = matches!(app.selection, Some(Selection::All));
    let current_tone = if all {
        power_tone(k, aggregate.power)
    } else {
        power_tone(k, single_power(&app.single_state()))
    };
    let current = selection_label(app);
    let trigger = dropdown_trigger(
        k,
        344.0,
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
                .checked(all)
                .lead(dot(power_tone(k, aggregate.power)))
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
        let health = app.health.status(&bulb.id);
        let tone = if health.is_down() {
            k.c.danger.icon
        } else {
            power_tone(k, state.map_or(AggregatePower::Unknown, single_power))
        };
        let power = if let Some(word) = health_word(health) {
            word
        } else if health == HealthStatus::Connecting {
            "Connecting"
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
    let select = picker.build(k, layers).grow(1.0).min_w(0.0);

    let add = icon_button(
        k,
        &PLUS,
        "Add bulb",
        IconButtonVariant::GhostMuted,
        |app: &mut App| app.open_add(),
    );

    let trigger = icon_button(
        k,
        &MORE,
        "Bulb actions",
        IconButtonVariant::GhostMuted,
        |app: &mut App| app.bulb_menu_open = !app.bulb_menu_open,
    );
    let single = matches!(app.selection, Some(Selection::Bulb(_)));
    let menu = Menu::new("bulb-actions")
        .trigger(trigger)
        .open(app.bulb_menu_open)
        .on_close(|app: &mut App| app.bulb_menu_open = false)
        .item(
            MenuItem::new("Refresh")
                .icon(&REFRESH)
                .disabled(!app.sync.controls().manual)
                .on_select_tx(|app: &mut App, tx: &mut Tx| {
                    app.bulb_menu_open = false;
                    app.refresh_all(tx);
                }),
        )
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

fn icon_button(
    k: &Kit,
    icon: &'static rune::Icon,
    help: &str,
    variant: IconButtonVariant,
    on_press: impl Fn(&mut App) + 'static,
) -> El {
    IconButton::new(icon)
        .variant(variant)
        .control_size(ControlSize::ExtraLarge)
        .help(help)
        .on_press(on_press)
        .build(k)
        .size(40.0, 40.0)
        .radius(k.radius.md)
}

fn control_deck(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let all = matches!(app.selection, Some(Selection::All));
    let aggregate = app.aggregate();
    let state = app.single_state();
    let quality = if all {
        aggregate.quality
    } else {
        light_quality_of(state.mode)
    };
    let white = quality == LightQuality::White;
    let colour = quality == LightQuality::Color;
    let mixed = quality == LightQuality::Mixed;
    let unknown = quality == LightQuality::Unknown;
    let sync_owned = !app.sync.controls().manual;
    let power = if all {
        aggregate.power
    } else {
        single_power(&state)
    };
    let power_switch = Switch::new(power == AggregatePower::On)
        .control_size(ControlSize::Large)
        .label("Power")
        .hide_label()
        .on_toggle(|app: &mut App| app.toggle_power())
        .build(k)
        .padding(6.0);

    let mut deck = col()
        .w_full()
        .child(
            row()
                .w_full()
                .px(16.0)
                .py(9.0)
                .center_y()
                .child(
                    text("Power".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .grow(1.0),
                )
                .child(power_switch),
        )
        .child(hairline(k));

    if sync_owned {
        deck = deck.child(col().w_full().padding(12.0).child(notice(
            k,
            &crate::ui::widget::SYNC,
            "Screen sync is driving colour, brightness and warmth. Power stays available.",
            k.c.info,
        )));
        deck = deck.child(hairline(k));
    }

    deck = deck.child(brightness_block(k, &aggregate, &state, all, sync_owned));
    deck =
        deck.child(hairline(k))
            .child(mode_block(k, white, colour, sync_owned || app.color.open));
    if white || colour || app.color.open {
        let (key, edge, block) = if white && !app.color.open {
            (
                "tuning-white",
                Edge::Leading,
                temperature_block(k, &aggregate, &state, all, sync_owned),
            )
        } else {
            (
                "tuning-colour",
                Edge::Trailing,
                colour_block(app, k, &aggregate, &state, all, sync_owned, layers),
            )
        };
        deck = deck.child(
            col()
                .key(key)
                .w_full()
                .transition(Transition::move_edge(edge).springs(SpringPair::new(
                    Spring::new(0.3, 0.06),
                    Spring::new(0.24, 0.03),
                )))
                .child(hairline(k))
                .child(block),
        );
    }
    if mixed {
        deck = deck.child(hairline(k)).child(
            col().w_full().padding(12.0).child(notice(
                k,
                &BULB,
                "The selected bulbs disagree on the mode. Choose White or Colour to apply one mode to all of them.",
                k.c.warning,
            )),
        );
    }
    if unknown && !mixed {
        deck = deck
            .child(hairline(k))
            .child(col().w_full().padding(12.0).child(notice(
                k,
                &BULB,
                "Waiting for the bulb to report its light state.",
                k.c.info,
            )));
    }
    let error = match app.selection.as_ref() {
        Some(Selection::Bulb(id)) => app.health.message(id).map(str::to_owned),
        Some(Selection::All) => app
            .bulbs
            .iter()
            .find_map(|bulb| app.health.message(&bulb.id))
            .map(str::to_owned),
        None => None,
    };
    if let Some(error) = error {
        deck = deck.child(hairline(k)).child(
            row()
                .w_full()
                .padding(12.0)
                .gap(8.0)
                .center_y()
                .child(
                    rune::icon(&ALERT, 15.0)
                        .color(k.c.danger.icon)
                        .a11y_hidden(),
                )
                .child(error_text(k, &error)),
        );
    }

    surface(k, deck)
}

fn brightness_block(
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
        SLIDER_WIDTH,
        |app: &mut App, fraction| app.set_brightness(brightness_from_fraction(fraction)),
    );
    let slider = dim_when(slider, disabled);
    let block = col()
        .w_full()
        .gap(2.0)
        .px(16.0)
        .py(10.0)
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
                .child(
                    text(label)
                        .font(theme::number(13.0, 560.0))
                        .color(k.c.text_muted),
                ),
        )
        .child(row().w_full().justify_center().child(slider));
    dim_when(block, disabled)
}

fn mode_block(k: &Kit, white: bool, colour: bool, disabled: bool) -> El {
    let selected = if white {
        Some(0)
    } else if colour {
        Some(1)
    } else {
        None
    };
    let control = segmented_switch(
        k,
        "mode",
        "Light mode",
        &["White", "Colour"],
        selected,
        |app, index, _tx| {
            if index == 0 {
                app.set_mode_white();
            } else {
                app.set_mode_color();
            }
        },
    );
    let block = row()
        .w_full()
        .gap(14.0)
        .px(16.0)
        .py(10.0)
        .center_y()
        .child(
            text("Mode".to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(row().grow(1.0).min_w(0.0).child(control));
    dim_when(block, disabled)
}

fn temperature_block(
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
            width: SLIDER_WIDTH,
        },
        stops,
        |app: &mut App, fraction| app.set_temperature(temperature_from_fraction(fraction)),
    );
    let slider = dim_when(slider, disabled);
    let block = col()
        .w_full()
        .gap(2.0)
        .px(16.0)
        .py(10.0)
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
                        .child(swatch(k, rgb_color(kelvin_to_rgb(value)), 12.0))
                        .child(
                            text(label)
                                .font(theme::number(13.0, 560.0))
                                .color(k.c.text_muted),
                        ),
                ),
        )
        .child(row().w_full().justify_center().child(slider))
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
        );
    dim_when(block, disabled)
}

fn colour_block(
    app: &App,
    k: &Kit,
    aggregate: &AggregateState,
    state: &BulbState,
    all: bool,
    disabled: bool,
    layers: &mut Vec<El>,
) -> El {
    let editing = app.color.open;
    let (color, mixed) = if editing {
        (Some(app.color.preview), false)
    } else if all {
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
    let choose = Button::new(if editing {
        "Editing colour"
    } else {
        "Choose colour"
    })
    .variant(if editing {
        ButtonVariant::Ghost
    } else {
        ButtonVariant::Outline
    })
    .control_size(ControlSize::Regular)
    .icon(&EDIT)
    .disabled(disabled)
    .on_press(|app: &mut App| app.open_color())
    .build(k)
    .key("choose-colour")
    .h(40.0);
    let label_style = if color.is_some() {
        theme::code(14.0, 500.0)
    } else {
        k.ty.ui_medium
    };
    let mut popover = Popover::new("colour-popover")
        .trigger(choose)
        .open(editing)
        .padding(0.0)
        .placement(Placement::TopEnd)
        .gutter(8.0)
        .label("Colour editor");
    if editing {
        popover = popover.child(color::popover(app, k));
    }
    let trigger = popover.build(k, layers);

    let trigger_row = row()
        .w_full()
        .gap(10.0)
        .px(16.0)
        .py(10.0)
        .center_y()
        .child(swatch(k, fill, 30.0))
        .child(
            text(label)
                .font(label_style)
                .color(k.c.text_base)
                .grow(1.0)
                .min_w(0.0)
                .truncate(),
        )
        .child(trigger);
    dim_when(trigger_row, disabled)
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

fn single_power(state: &BulbState) -> AggregatePower {
    match state.power {
        Some(true) => AggregatePower::On,
        Some(false) => AggregatePower::Off,
        None => AggregatePower::Unknown,
    }
}

pub(super) fn health_word(status: HealthStatus) -> Option<&'static str> {
    status.is_down().then_some("Unreachable")
}

fn power_tone(k: &Kit, power: AggregatePower) -> Color {
    match power {
        AggregatePower::On => k.c.success.icon,
        AggregatePower::Mixed => k.c.warning.icon,
        AggregatePower::Off | AggregatePower::Unknown => k.c.text_faint,
    }
}
