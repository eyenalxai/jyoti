mod stage;

use rune::{Color, Edge, El, ScrollSpec, Spring, SpringPair, Transition, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Menu, MenuItem,
    Placement, Popover, Switch,
};

use crate::app::App;
use crate::app::selected::SelectedLight;
use crate::bulb::aggregate::{AggregatePower, LightQuality, power_of_state};
use crate::bulb::health::HealthStatus;
use crate::bulb::model::{
    Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN, brightness_from_fraction, fraction_of_brightness,
    fraction_of_temperature, temperature_from_fraction,
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
    let Some(view) = app.selected_light() else {
        return stage::empty_state(k);
    };

    let viewport = col()
        .key("control")
        .w_full()
        .padding(PAGE_PAD)
        .gap(9.0)
        .child(picker_row(app, k, layers, &view))
        .child(stage::light_stage(&view, k))
        .child(control_deck(app, k, layers, &view))
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn picker_row(app: &App, k: &Kit, layers: &mut Vec<El>, view: &SelectedLight) -> El {
    let show_all = view.health.total >= 2;
    let all = view.is_all();
    let current_tone = power_tone(k, view.power);
    let trigger = dropdown_trigger(
        k,
        344.0,
        "Bulb",
        &view.name,
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
                .lead(dot(power_tone(k, view.power)))
                .description(format!(
                    "{} configured · {} reachable",
                    view.health.total, view.health.reachable
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
            power_tone(k, state.map_or(AggregatePower::Unknown, power_of_state))
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
    let single = !view.is_all();
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

fn control_deck(app: &App, k: &Kit, layers: &mut Vec<El>, view: &SelectedLight) -> El {
    let quality = view.quality;
    let white = quality == LightQuality::White;
    let colour = quality == LightQuality::Color;
    let mixed = quality == LightQuality::Mixed;
    let unknown = quality == LightQuality::Unknown;
    let sync_owned = !app.sync.controls().manual;
    let power = view.power;
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

    deck = deck.child(brightness_block(k, view, sync_owned));
    deck =
        deck.child(hairline(k))
            .child(mode_block(k, white, colour, sync_owned || app.color.open));
    if white || colour || app.color.open {
        let (key, edge, block) = if white && !app.color.open {
            (
                "tuning-white",
                Edge::Leading,
                temperature_block(k, view, sync_owned),
            )
        } else {
            (
                "tuning-colour",
                Edge::Trailing,
                colour_block(app, k, view, sync_owned, layers),
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
    if let Some(error) = view.health.message.as_deref() {
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
                .child(error_text(k, error)),
        );
    }

    surface(k, deck)
}

fn brightness_block(k: &Kit, view: &SelectedLight, disabled: bool) -> El {
    let value = view.brightness_value();
    let label = view.brightness_label();
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

fn temperature_block(k: &Kit, view: &SelectedLight, disabled: bool) -> El {
    let value = view.temperature_value();
    let label = view.temperature_label();
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
    view: &SelectedLight,
    disabled: bool,
    layers: &mut Vec<El>,
) -> El {
    let editing = app.color.open;
    let (color, mixed) = if editing {
        (Some(app.color.preview), false)
    } else {
        (view.color_value(), view.color_mixed())
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
