use std::time::Instant;

use rune::{Color, El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Menu, MenuItem};

use crate::app::App;
use crate::app::sync::run::RunPhase;
use crate::bulb::model::Rgb;
use crate::screen::gain::{SYNC_GAIN_MAX, SYNC_GAIN_MIN};
use crate::screen::intensity::intensify;
use crate::ui::theme;
use crate::ui::widget::{
    ALERT, PAGE_PAD, SLIDER_WIDTH, SYNC, SliderSpec, dot, dropdown_trigger, frame_age,
    gradient_slider, hairline, nameplate, notice, rgb_color, slider, stage_surface, surface,
    swatch, value_text,
};

const STAGE_HEIGHT: f32 = 190.0;

pub fn view(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let controls = app.sync.controls();
    let active = controls.stop;
    let paused = controls.resume;

    let start = Button::new("Start sync")
        .variant(if app.sync.monitors.is_empty() {
            ButtonVariant::Outline
        } else {
            ButtonVariant::Contrast
        })
        .control_size(ControlSize::Large)
        .disabled(!controls.start || app.sync.monitors.is_empty())
        .on_press(|app: &mut App| app.start_sync())
        .build(k)
        .h(40.0);
    let pause = Button::new(if paused { "Resume" } else { "Pause" })
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .disabled(!(controls.pause || controls.resume))
        .on_press(|app: &mut App| {
            if app.sync.controls().resume {
                app.resume_sync();
            } else {
                app.pause_sync();
            }
        })
        .build(k)
        .h(40.0);
    let stop = Button::new("Stop and restore")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Large)
        .disabled(!controls.stop)
        .on_press(|app: &mut App| app.stop_sync())
        .build(k)
        .h(40.0);

    let actions = if active {
        row()
            .w_full()
            .gap(8.0)
            .child(pause.grow(1.0))
            .child(stop.grow(1.0))
    } else {
        row().w_full().child(start.grow(1.0))
    };

    let content = col()
        .key("sync")
        .w_full()
        .padding(PAGE_PAD)
        .gap(10.0)
        .child(capture_stage(app, k))
        .child(actions)
        .child(capture_panel(app, k, layers))
        .child(output_panel(app, k));

    let viewport = content.scroll(ScrollSpec {
        stick_bottom: false,
    });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);
    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn capture_stage(app: &App, k: &Kit) -> El {
    let derived = app.sync.derived;
    let fill = derived.map_or(k.c.layer_02, rgb_color);
    let title = derived.map_or_else(|| "No frame yet".to_owned(), |color| color.hex());
    let title_style = if derived.is_some() {
        theme::code(26.0, 500.0)
    } else {
        theme::display(28.0, 580.0)
    };
    let monitor = app
        .sync
        .selected_monitor
        .clone()
        .unwrap_or_else(|| "No monitor selected".to_owned());
    let detail = if derived.is_some() {
        monitor.clone()
    } else {
        "Choose a monitor, then start sync.".to_owned()
    };
    let status = &app.sync.status;
    let status_word = match status {
        RunPhase::Failed { .. } => "Error".to_owned(),
        _ => status.label(),
    };
    let status_ink = sync_status_ink(status);
    let plate_ink = theme::ink();
    let muted = plate_ink.alpha(0.62);
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
                        row()
                            .grow(1.0)
                            .min_w(0.0)
                            .gap(6.0)
                            .center_y()
                            .child(dot(status_ink))
                            .child(
                                text(status_word)
                                    .font(k.ty.small_medium)
                                    .color(plate_ink)
                                    .truncate(),
                            ),
                    )
                    .child(
                        row()
                            .gap(5.0)
                            .center_y()
                            .child(text("Frame".to_owned()).font(k.ty.small).color(muted))
                            .child(
                                row().w(62.0).justify_end().child(
                                    text(frame_age(app.sync.last_scene_at))
                                        .font(theme::number(12.0, 450.0))
                                        .color(muted),
                                ),
                            ),
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
                            .child(text(title).font(title_style).color(plate_ink).truncate())
                            .child(text(detail).font(k.ty.small).color(muted).truncate()),
                    )
                    .child(
                        col()
                            .items_end()
                            .gap(0.0)
                            .child(text("BRIGHTNESS").font(theme::label()).color(muted))
                            .child(if derived.is_some() {
                                row()
                                    .items_end()
                                    .gap(1.0)
                                    .child(
                                        text(format!("{}", app.sync.brightness))
                                            .font(theme::number(38.0, 610.0))
                                            .color(plate_ink),
                                    )
                                    .child(text("%").font(theme::number(16.0, 560.0)).color(muted))
                            } else {
                                row().child(
                                    text("–".to_owned())
                                        .font(theme::number(28.0, 560.0))
                                        .color(muted),
                                )
                            }),
                    ),
            ),
    );

    stage_surface(k, STAGE_HEIGHT, fill, 0.2, plate)
}

fn capture_panel(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let mut body = col().w_full().child(monitor_row(app, k, layers));
    let now = Instant::now();
    if app.sync.stale_at(now) {
        body = body.child(hairline(k)).child(padded_notice(
            k,
            &SYNC,
            "The last frame is older than five seconds. Capture recovers on the next good frame.",
            k.c.warning,
        ));
    } else if app.sync.flashing_at(now) {
        body = body.child(hairline(k)).child(padded_notice(
            k,
            &SYNC,
            "The flash guard is holding brightness after a large scene reversal.",
            k.c.warning,
        ));
    }
    let error = app
        .sync
        .capture_error
        .clone()
        .or_else(|| match &app.sync.status {
            RunPhase::Failed { message, .. } => Some(message.clone()),
            _ => None,
        });
    if let Some(error) = &error {
        body = body
            .child(hairline(k))
            .child(padded_notice(k, &ALERT, error, k.c.danger))
            .child(
                row().w_full().px(14.0).pb(12.0).justify_end().child(
                    Button::new("Retry capture")
                        .variant(ButtonVariant::Outline)
                        .control_size(ControlSize::Regular)
                        .on_press(|app: &mut App| app.restart_screen())
                        .build(k)
                        .h(40.0),
                ),
            );
    }
    surface(k, body)
}

fn monitor_row(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    if app.sync.monitors.is_empty() {
        return col().w_full().padding(14.0).child(notice(
            k,
            &SYNC,
            "No monitors found. Capture needs a Wayland compositor with wlr-screencopy, such as Hyprland or Sway.",
            k.c.warning,
        ));
    }
    let current = app
        .sync
        .selected_monitor
        .clone()
        .unwrap_or_else(|| "Select a monitor".to_owned());
    let resolution = app
        .sync
        .monitors
        .iter()
        .find(|monitor| monitor.name == current)
        .map_or_else(String::new, |monitor| {
            format!("{} × {}", monitor.width, monitor.height)
        });
    let trigger = dropdown_trigger(
        k,
        240.0,
        "Monitor",
        &current,
        app.monitor_picker_open,
        None,
        |app: &mut App| app.monitor_picker_open = !app.monitor_picker_open,
    );
    let mut picker = Menu::new("monitor-picker")
        .trigger(trigger)
        .open(app.monitor_picker_open)
        .min_width(300.0)
        .on_close(|app: &mut App| app.monitor_picker_open = false);
    for monitor in &app.sync.monitors {
        let selected = app.sync.selected_monitor.as_deref() == Some(monitor.name.as_str());
        let name = monitor.name.clone();
        let detail = format!(
            "{} × {}{}",
            monitor.width,
            monitor.height,
            if monitor.description.is_empty() {
                String::new()
            } else {
                format!(" · {}", monitor.description)
            }
        );
        picker = picker.item(
            MenuItem::new(monitor.name.clone())
                .checked(selected)
                .description(detail)
                .on_select(move |app: &mut App| {
                    app.monitor_picker_open = false;
                    app.select_monitor(name.clone());
                }),
        );
    }
    row()
        .w_full()
        .gap(14.0)
        .padding(14.0)
        .center_y()
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(1.0)
                .child(
                    text("Monitor".to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(resolution)
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate(),
                ),
        )
        .child(picker.build(k, layers))
}

fn output_panel(app: &App, k: &Kit) -> El {
    let source = app.sync.last_scene.map_or(Rgb::WHITE, |scene| scene.color);
    let vivid = intensify(source, 100.0);
    let output = app.sync.derived.map_or(k.c.layer_03, rgb_color);
    let intensity = gradient_slider(
        k,
        app.sync.colour_intensity / 100.0,
        SliderSpec {
            key: "sync-intensity",
            label: "Colour intensity",
            step: 1.0 / 100.0,
            width: SLIDER_WIDTH,
        },
        vec![(0.0, rgb_color(source)), (1.0, rgb_color(vivid))],
        |app: &mut App, fraction| app.set_sync_intensity(fraction * 100.0),
    );
    let gain = ((app.sync.gain - SYNC_GAIN_MIN) / (SYNC_GAIN_MAX - SYNC_GAIN_MIN)).clamp(0.0, 1.0);
    let gain = slider(
        k,
        "sync-gain",
        gain,
        "Sync brightness gain",
        0.1 / (SYNC_GAIN_MAX - SYNC_GAIN_MIN),
        SLIDER_WIDTH,
        |app: &mut App, fraction| {
            app.set_sync_gain(SYNC_GAIN_MIN + fraction * (SYNC_GAIN_MAX - SYNC_GAIN_MIN));
        },
    );

    surface(
        k,
        col()
            .w_full()
            .child(
                col()
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
                                text("Colour intensity".to_owned())
                                    .font(k.ty.ui_medium)
                                    .color(k.c.text_base),
                            )
                            .child(
                                row()
                                    .gap(6.0)
                                    .center_y()
                                    .child(swatch(k, output, 12.0))
                                    .child(value_text(
                                        k,
                                        format!("{:.0}%", app.sync.colour_intensity),
                                    )),
                            ),
                    )
                    .child(row().w_full().justify_center().child(intensity))
                    .child(
                        text("Natural to fully saturated; hue and value stay put.".to_owned())
                            .font(k.ty.small)
                            .color(k.c.text_muted),
                    ),
            )
            .child(hairline(k))
            .child(
                col()
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
                                text("Brightness gain".to_owned())
                                    .font(k.ty.ui_medium)
                                    .color(k.c.text_base),
                            )
                            .child(value_text(k, format!("{:.1}×", app.sync.gain))),
                    )
                    .child(row().w_full().justify_center().child(gain))
                    .child(
                        text("50–400%; output caps at 100%.".to_owned())
                            .font(k.ty.small)
                            .color(k.c.text_muted),
                    ),
            ),
    )
}

fn padded_notice(
    k: &Kit,
    icon: &'static rune::Icon,
    message: &str,
    state: rune_kit::StateColors,
) -> El {
    col()
        .w_full()
        .padding(14.0)
        .child(notice(k, icon, message, state))
}

fn sync_status_ink(status: &RunPhase) -> Color {
    match status {
        RunPhase::Running(_) | RunPhase::Starting(_) => theme::positive_on_plate(),
        RunPhase::Paused(_) | RunPhase::Stopping(_) => theme::warning_on_plate(),
        RunPhase::Failed { .. } => theme::danger_on_plate(),
        RunPhase::Idle => theme::quiet_on_plate(),
    }
}
