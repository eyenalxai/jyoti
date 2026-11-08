use rune::{Align, El, ScrollSpec, TextStyle, col, row, text};
use rune_kit::{
    Button, ButtonVariant, Cell, ControlSize, Divider, IconButton, IconButtonVariant, Kit, Switch,
    Table, TableColumn, TableStyle,
};

use crate::app::App;
use crate::bulb::aggregate::{
    AggregateColor, AggregatePower, AggregateState, LightQuality, light_quality_of,
};
use crate::bulb::model::{
    BRIGHTNESS_MAX, BRIGHTNESS_MIN, BulbState, ColorMode, Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN,
    brightness_from_fraction, fraction_of_brightness, fraction_of_temperature,
    temperature_from_fraction,
};
use crate::bulb::selection::Selection;
use crate::screen::color::kelvin_to_rgb;
use crate::ui::widget::{
    BULB, EDIT, SYNC, TRASH, badge, dot, error_text, fact_row, frame_age, notice, panel,
    panel_body, panel_footer, panel_header, rgb_color, segmented, setting_row, slider, stat,
    swatch, sync_status_tone, value_text,
};

pub fn content(app: &App, k: &Kit) -> El {
    if app.bulbs.is_empty() {
        return empty_state(app, k);
    }

    let aggregate = app.aggregate();
    let scroll = col()
        .key("dashboard")
        .w_full()
        .padding(18.0)
        .gap(14.0)
        .child(dashboard_stats(app, k, &aggregate))
        .child(bulbs_panel(app, k))
        .child(
            row()
                .w_full()
                .gap(14.0)
                .items_start()
                .child(
                    col()
                        .grow(2.0)
                        .min_w(0.0)
                        .child(light_panel(app, k, &aggregate)),
                )
                .child(
                    col()
                        .grow(1.0)
                        .min_w(300.0)
                        .gap(14.0)
                        .child(device_panel(app, k))
                        .child(sync_panel(app, k)),
                ),
        )
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn dashboard_stats(app: &App, k: &Kit, aggregate: &AggregateState) -> El {
    let all = matches!(app.selection, Some(Selection::All));
    let state = app.single_state();
    let power = if all {
        aggregate_power_word(aggregate.power)
    } else {
        state_power_word(&state)
    };
    let brightness = if all {
        span_word(aggregate.brightness.map(|span| (span.low, span.high)), "%")
    } else {
        state
            .brightness
            .map_or_else(|| "Unknown".to_owned(), |value| format!("{value}%"))
    };
    let sync = app.sync.status.label();
    let sync_meta = if app.sync.last_scene_at.is_some() {
        frame_age(app.sync.last_scene_at)
    } else {
        "No frame yet".to_owned()
    };
    let power_tone = match power {
        "On" => k.c.success.icon,
        "Off" => k.c.text_faint,
        "Mixed" => k.c.warning.icon,
        _ => k.c.text_faint,
    };

    row()
        .w_full()
        .gap(12.0)
        .items_stretch()
        .child(stat(
            k,
            "Bulbs",
            format!("{}", aggregate.total),
            format!(
                "{} reachable · {} failed",
                aggregate.reachable, aggregate.failed
            ),
            k.c.info.icon,
        ))
        .child(stat(k, "Power", power, "Selected bulbs", power_tone))
        .child(stat(
            k,
            "Brightness",
            brightness,
            if all {
                "Aggregate span"
            } else {
                "Current level"
            },
            k.c.warning.icon,
        ))
        .child(stat(
            k,
            "Screen sync",
            sync,
            sync_meta,
            sync_status_tone(k, &app.sync.status),
        ))
}

fn light_panel(app: &App, k: &Kit, aggregate: &AggregateState) -> El {
    let state = app.single_state();
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
        228.0,
        choose_mode,
    );
    let mode = if sync_owned { mode.inert() } else { mode };

    let mut controls = col().w_full().gap(0.0);
    controls = controls.child(setting_row(
        k,
        "Mode",
        "White uses warmth. Colour uses packed RGB.",
        mode,
    ));

    controls = controls.child(Divider::horizontal().build(k));
    controls = controls.child(brightness_row(app, k, aggregate, &state, all, sync_owned));

    if white {
        controls = controls.child(Divider::horizontal().build(k));
        controls = controls.child(temperature_row(app, k, aggregate, &state, all, sync_owned));
    }
    if color {
        controls = controls.child(Divider::horizontal().build(k));
        controls = controls.child(colour_row(app, k, aggregate, &state, all, sync_owned));
    }
    if mixed || unknown {
        controls = controls.child(Divider::horizontal().build(k)).child(notice(
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

    let mut body = col().w_full().child(panel_body(controls));
    if sync_owned {
        body = body.child(col().w_full().px(14.0).pb(14.0).child(notice(
            k,
            &SYNC,
            "Screen sync owns colour, brightness and temperature while it runs.",
            k.c.warning,
        )));
    }
    if sync_owned {
        body = body.child(panel_footer(
            k,
            vec![
                Button::new("Stop and restore")
                    .variant(ButtonVariant::Danger)
                    .control_size(ControlSize::Small)
                    .on_press(|app: &mut App| app.stop_sync())
                    .build(k),
            ],
        ));
    }

    let mut header_actions = Vec::new();
    if all && aggregate.power == AggregatePower::Mixed {
        header_actions.push(badge(k, "Mixed power", k.c.warning.icon));
    }
    header_actions.push(power_switch);
    let header = panel_header(
        k,
        "Light control",
        Some("Adjust the selected bulb or every bulb at once."),
        header_actions,
    );
    panel(k, col().w_full().child(header).child(body))
}

fn brightness_row(
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
        320.0,
        |app: &mut App, fraction| app.set_brightness(brightness_from_fraction(fraction)),
    );
    col()
        .w_full()
        .gap(4.0)
        .py(10.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(12.0)
                .center_y()
                .child(
                    text("Brightness".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(value_text(k, label)),
        )
        .child(
            text(format!(
                "{BRIGHTNESS_MIN}–{BRIGHTNESS_MAX}% across the selected bulbs."
            ))
            .font(k.ty.small)
            .color(k.c.text_muted),
        )
        .child(if disabled {
            slider.inert().layer_opacity(0.55)
        } else {
            slider
        })
}

fn temperature_row(
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
        320.0,
        |app: &mut App, fraction| app.set_temperature(temperature_from_fraction(fraction)),
    );
    col()
        .w_full()
        .gap(4.0)
        .py(10.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(12.0)
                .center_y()
                .child(
                    text("Warmth".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(swatch(k, rgb_color(kelvin_to_rgb(value)), 18.0))
                        .child(value_text(k, label)),
                ),
        )
        .child(
            text(format!(
                "{TEMPERATURE_MIN}–{TEMPERATURE_MAX} K for the White side."
            ))
            .font(k.ty.small)
            .color(k.c.text_muted),
        )
        .child(if disabled {
            slider.inert().layer_opacity(0.55)
        } else {
            slider
        })
}

fn colour_row(
    _app: &App,
    k: &Kit,
    aggregate: &AggregateState,
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
    let mut control = row()
        .gap(10.0)
        .center_y()
        .child(swatch(k, color.map_or(k.c.layer_03, rgb_color), 26.0))
        .child(text(label).font(k.ty.mono).color(k.c.text_base));
    if disabled {
        control = control.child(badge(k, "Sync owns this", k.c.warning.icon));
    } else {
        control = control.child(
            Button::new("Choose colour")
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Small)
                .on_press(|app: &mut App| app.open_color())
                .build(k),
        );
    }
    setting_row(
        k,
        "Colour",
        "Packed sRGB. The picker previews live, then keeps or restores it.",
        control,
    )
}

fn device_panel(app: &App, k: &Kit) -> El {
    let all = matches!(app.selection, Some(Selection::All));
    let aggregate = app.aggregate();
    let state = app.single_state();
    let selected = app.selected_bulb();
    let title = if all {
        "Selection"
    } else {
        selected.map_or("Selection", |bulb| bulb.name.as_str())
    };
    let subtitle = if all {
        "All configured bulbs"
    } else {
        "One bulb"
    };
    let power = if all {
        aggregate_power_word(aggregate.power)
    } else {
        state_power_word(&state)
    };
    let tone = match power {
        "On" => k.c.success.icon,
        "Off" => k.c.text_faint,
        "Mixed" => k.c.warning.icon,
        _ => k.c.text_faint,
    };

    let mut facts = col().w_full().gap(0.0);
    if all {
        facts = facts
            .child(fact_row(k, "Bulbs", format!("{}", aggregate.total)))
            .child(fact_row(k, "Reachable", format!("{}", aggregate.reachable)))
            .child(fact_row(k, "Failed", format!("{}", aggregate.failed)))
            .child(fact_row(k, "Power", power))
            .child(fact_row(
                k,
                "Brightness",
                span_word(aggregate.brightness.map(|span| (span.low, span.high)), "%"),
            ))
            .child(fact_row(
                k,
                "Warmth",
                span_word(
                    aggregate.temperature.map(|span| (span.low, span.high)),
                    " K",
                ),
            ))
            .child(fact_row(
                k,
                "Colour",
                match aggregate.color {
                    Some(AggregateColor::Equal(color)) => color.hex(),
                    Some(AggregateColor::Mixed) => "Mixed".to_owned(),
                    None => "Unknown".to_owned(),
                },
            ));
    } else if let Some(bulb) = selected {
        facts = facts
            .child(fact_row(k, "Address", bulb.address.clone()))
            .child(fact_row(
                k,
                "Model",
                bulb.model.clone().unwrap_or_else(|| "Unknown".to_owned()),
            ))
            .child(fact_row(k, "Power", power))
            .child(fact_row(
                k,
                "Mode",
                quality_word(light_quality_of(state.mode)),
            ))
            .child(fact_row(
                k,
                "Brightness",
                state
                    .brightness
                    .map_or_else(|| "Unknown".to_owned(), |value| format!("{value}%")),
            ))
            .child(fact_row(
                k,
                "Warmth",
                state
                    .temperature
                    .map_or_else(|| "Unknown".to_owned(), |value| format!("{value} K")),
            ))
            .child(fact_row(
                k,
                "Colour",
                state.color.map_or_else(|| "Unknown".to_owned(), Rgb::hex),
            ));
    }

    let error = app
        .selection
        .as_ref()
        .and_then(|selection| match selection {
            Selection::Bulb(id) => app.errors.get(id).cloned(),
            Selection::All => app.errors.values().next().cloned(),
        });

    let mut body = panel_body(facts);
    if let Some(error) = error {
        body = body.child(error_text(k, &error));
    }
    let edit = IconButton::new(&EDIT)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Small)
        .help("Edit selected bulb")
        .disabled(all)
        .on_press(|app: &mut App| app.edit_selected())
        .build(k);
    let remove = IconButton::new(&TRASH)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Small)
        .help("Remove selected bulb")
        .disabled(all)
        .on_press(|app: &mut App| {
            if let Some(Selection::Bulb(id)) = app.selection.clone() {
                app.open_remove(&id);
            }
        })
        .build(k);

    let header = panel_header(k, title, Some(subtitle), vec![dot(tone), edit, remove]);
    panel(k, col().w_full().child(header).child(body))
}

fn sync_panel(app: &App, k: &Kit) -> El {
    let tone = sync_status_tone(k, &app.sync.status);
    let open = Button::new("Open")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Small)
        .on_press(|app: &mut App| app.open_sync())
        .build(k);
    let monitor = app
        .sync
        .selected_monitor
        .clone()
        .unwrap_or_else(|| "No monitor".to_owned());
    let derived = app
        .sync
        .derived
        .map_or_else(|| "No frame yet".to_owned(), Rgb::hex);
    let mut facts = col()
        .w_full()
        .gap(0.0)
        .child(fact_row(k, "Status", app.sync.status.label()))
        .child(fact_row(k, "Monitor", monitor))
        .child(fact_row(k, "Derived", derived))
        .child(fact_row(
            k,
            "Brightness",
            format!("{}% · {:.1}×", app.sync.brightness, app.sync.gain),
        ))
        .child(fact_row(k, "Frame age", frame_age(app.sync.last_scene_at)));
    if let Some(error) = &app.sync.capture_error {
        facts = facts.child(notice(k, &SYNC, error, k.c.danger));
    }
    let header = panel_header(
        k,
        "Screen sync",
        Some("Drive every bulb from one monitor."),
        vec![dot(tone), open],
    );
    panel(k, col().w_full().child(header).child(panel_body(facts)))
}

fn bulbs_panel(app: &App, k: &Kit) -> El {
    let header = panel_header(
        k,
        "Configured bulbs",
        Some("Select a row to control it. Double-click to edit."),
        Vec::new(),
    );
    let height = (28.0 + 34.0 * app.bulbs.len() as f32).clamp(110.0, 340.0);
    let table = bulb_table(app, k);
    panel(
        k,
        col()
            .w_full()
            .child(header)
            .child(col().w_full().h(height).child(table.grow(1.0).min_h(0.0))),
    )
}

fn bulb_table(app: &App, k: &Kit) -> El {
    let kit = k.clone();
    let style = TableStyle {
        row_height: 34.0,
        header_height: 28.0,
        cell_pad: 8.0,
        font: k.ty.text_small,
        mono: k.ty.mono,
        header_font: k.ty.text_small_medium,
        text: k.c.text_base,
        muted: k.c.text_muted,
        header_text: k.c.text_muted,
        rule: k.c.border_weak,
        selected: k.c.layer_04,
        stripe: k.c.layer_02,
    };
    Table::new(
        |app: &App| &app.table,
        |app: &mut App| &mut app.table,
        |app: &App| app.bulbs.len(),
        |app: &App, index| {
            app.bulbs
                .get(index)
                .map_or("", |bulb| bulb.id.as_str())
                .into()
        },
    )
    .column(TableColumn::new("name", "Name").flex(2.0).min(96.0))
    .column(TableColumn::new("address", "Address").width(110.0))
    .column(TableColumn::new("status", "State").width(92.0))
    .column(TableColumn::new("mode", "Mode").width(68.0))
    .column(
        TableColumn::new("brightness", "Brightness")
            .width(86.0)
            .align(Align::End),
    )
    .column(
        TableColumn::new("temperature", "Warmth")
            .width(86.0)
            .align(Align::End),
    )
    .column(TableColumn::new("colour", "Colour").width(90.0))
    .cell(move |app: &App, index: usize, column: &str| {
        let Some(bulb) = app.bulbs.get(index) else {
            return Cell::text("");
        };
        let state = app.states.get(&bulb.id);
        match column {
            "name" => Cell::text(bulb.name.clone()),
            "address" => Cell::text(bulb.address.clone()).mono().muted(),
            "status" => {
                let (label, tone) = if app.errors.contains_key(&bulb.id) {
                    ("Error", kit.c.danger.icon)
                } else if app.sync.unreachable.contains(&bulb.id) {
                    ("Unreachable", kit.c.danger.icon)
                } else {
                    match state.and_then(|state| state.power) {
                        Some(true) => ("On", kit.c.success.icon),
                        Some(false) => ("Off", kit.c.text_faint),
                        None => ("Connecting", kit.c.warning.icon),
                    }
                };
                Cell::el(badge(&kit, label, tone))
            }
            "mode" => {
                let mode = state.map_or(ColorMode::Unknown, |state| state.mode);
                Cell::text(quality_word(light_quality_of(mode))).muted()
            }
            "brightness" => Cell::text(
                state
                    .and_then(|state| state.brightness)
                    .map_or_else(|| "—".to_owned(), |value| format!("{value}%")),
            )
            .mono()
            .muted(),
            "temperature" => Cell::text(
                state
                    .and_then(|state| state.temperature)
                    .map_or_else(|| "—".to_owned(), |value| format!("{value} K")),
            )
            .mono()
            .muted(),
            "colour" => state.and_then(|state| state.color).map_or_else(
                || Cell::text("—").muted(),
                |color| {
                    Cell::el(
                        row()
                            .gap(6.0)
                            .center_y()
                            .child(swatch(&kit, rgb_color(color), 14.0))
                            .child(text(color.hex()).font(kit.ty.mono).color(kit.c.text_muted)),
                    )
                },
            ),
            _ => Cell::text(""),
        }
    })
    .style(style)
    .name("Configured bulbs")
    .on_select(|app: &mut App, id: &str, _tx| app.select_bulb(id))
    .on_open(|app: &mut App, id: &str, _tx| app.open_edit(id))
    .build(app, k)
}

fn empty_state(_app: &App, k: &Kit) -> El {
    let add = Button::new("Add your first bulb")
        .variant(ButtonVariant::Contrast)
        .on_press(|app: &mut App| app.open_add())
        .build(k);
    let help = Button::new("How it works")
        .variant(ButtonVariant::Outline)
        .on_press(|app: &mut App| app.open_help())
        .build(k);
    let body = row()
        .w_full()
        .padding(20.0)
        .gap(28.0)
        .items_start()
        .child(
            col()
                .grow(1.0)
                .gap(10.0)
                .child(rune::icon(&BULB, 32.0).color(k.c.text_faint))
                .child(
                    text("No bulbs yet".to_owned())
                        .font(TextStyle::new(20.0, 600.0).lh(24.0))
                        .color(k.c.text_base),
                )
                .child(
                    text("Add a Xiaomi bulb to control power, brightness, warmth and colour from one window."
                        .to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap(),
                )
                .child(row().gap(8.0).child(add).child(help)),
        )
        .child(
            col()
                .w(280.0)
                .gap(8.0)
                .child(fact_row(k, "Storage", "XDG config"))
                .child(fact_row(k, "Tokens", "OS keyring"))
                .child(fact_row(k, "Protocol", "miIO over UDP"))
                .child(fact_row(k, "Capture", "Wayland · libwayshot")),
        );

    col()
        .key("empty")
        .w_full()
        .h_full()
        .center()
        .padding(24.0)
        .child(col().w_full().max_w(820.0).child(panel(k, body)))
}

fn choose_mode(app: &mut App, index: usize) {
    if index == 0 {
        app.set_mode_white();
    } else {
        app.set_mode_color();
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
