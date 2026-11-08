use std::time::Duration;

use rune::{Color, Cx, El, ScrollSpec, col, row, text};
use rune_kit::shell::Page;
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit};

use crate::app::App;
use crate::screen::gain::{SYNC_GAIN_MAX, SYNC_GAIN_MIN};
use crate::screen::monitor::MonitorInfo;
use crate::screen::sync::SyncStatus;
use crate::ui::widget::{BACK, badge, dot, rgb_color, section, slider, swatch, sync_status_tone};

pub fn view(app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);

    let running = app.sync.status.is_running();
    let active = app.sync.status.is_active();
    let paused = app.sync.status == SyncStatus::Paused;
    let stopping = app.sync.status == SyncStatus::Stopping;

    let start = Button::new("Start sync")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Regular)
        .disabled(active || stopping || app.sync.monitors.is_empty())
        .on_press(|app: &mut App| app.start_sync())
        .build(k);
    let pause = Button::new(if paused { "Resume" } else { "Pause" })
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .disabled(!running || stopping)
        .on_press(|app: &mut App| {
            if app.sync.status == SyncStatus::Paused {
                app.resume_sync();
            } else {
                app.pause_sync();
            }
        })
        .build(k);
    let stop = Button::new("Stop and restore")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Regular)
        .disabled(!active && !stopping)
        .on_press(|app: &mut App| app.stop_sync())
        .build(k);

    let status = section(
        k,
        "Now",
        col()
            .gap(10.0)
            .child(
                row()
                    .justify_between()
                    .w_full()
                    .center_y()
                    .child(
                        row()
                            .gap(8.0)
                            .center_y()
                            .child(dot(sync_status_tone(k, &app.sync.status)))
                            .child(
                                text(app.sync.status.label())
                                    .font(k.ty.ui_medium)
                                    .color(k.c.text_base),
                            ),
                    )
                    .child(
                        text(format!("{:.1}× gain", app.sync.gain))
                            .font(k.ty.mono)
                            .color(k.c.text_muted),
                    ),
            )
            .child(
                text(match app.sync.selected_monitor.as_deref() {
                    Some(monitor) if app.sync.status.is_running() => {
                        format!("Capturing {monitor}")
                    }
                    Some(monitor) => format!("Monitor {monitor}"),
                    None => "No monitor selected".to_owned(),
                })
                .font(k.ty.small)
                .color(k.c.text_muted),
            )
            .child(
                row()
                    .gap(10.0)
                    .center_y()
                    .child(swatch(
                        k,
                        app.sync.derived.map_or(k.c.layer_03, rgb_color),
                        30.0,
                    ))
                    .child(
                        text(
                            app.sync
                                .derived
                                .map_or_else(|| "No frame yet".to_owned(), |color| color.hex()),
                        )
                        .font(k.ty.mono)
                        .color(k.c.text_base),
                    )
                    .child(
                        text(format!("{}% brightness", app.sync.brightness))
                            .font(k.ty.small)
                            .color(k.c.text_muted),
                    ),
            )
            .child(
                row()
                    .gap(6.0)
                    .child(
                        text(format!("Frame age {}", frame_age(app)))
                            .font(k.ty.small)
                            .color(k.c.text_faint),
                    )
                    .child(if app.sync.stale {
                        badge(k, "Stale", k.c.warning.icon)
                    } else if app.sync.flashing {
                        badge(k, "Flashing scene", k.c.warning.icon)
                    } else if app.sync.monitors.len() == 1 {
                        badge(k, "Self-capture", k.c.warning.icon)
                    } else {
                        rune::el().size(0.0, 0.0)
                    }),
            )
            .child(row().gap(8.0).child(start).child(pause).child(stop)),
    );

    let mut monitors = col().gap(6.0);
    if app.sync.monitors.is_empty() {
        monitors = monitors.child(
            text("No monitors found. Screen capture needs a Wayland compositor that implements wlr-screencopy, such as Hyprland or Sway."
                .to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        );
    } else {
        for monitor in &app.sync.monitors {
            monitors = monitors.child(monitor_row(app, k, monitor));
        }
    }
    let monitor_section = section(k, "Monitor", monitors);

    let capture = app.sync.capture_error.as_ref().map(|error| {
        section(
            k,
            "Capture",
            col()
                .gap(8.0)
                .child(
                    text(error.clone())
                        .font(k.ty.small)
                        .color(k.c.danger.icon)
                        .wrap(),
                )
                .child(
                    Button::new("Retry capture")
                        .variant(ButtonVariant::Outline)
                        .on_press(|app: &mut App| app.restart_screen())
                        .build(k),
                ),
        )
    });

    let gain_fraction =
        ((app.sync.gain - SYNC_GAIN_MIN) / (SYNC_GAIN_MAX - SYNC_GAIN_MIN)).clamp(0.0, 1.0);
    let gain = section(
        k,
        "Brightness",
        col()
            .gap(10.0)
            .child(slider(
                k,
                "sync-gain",
                gain_fraction,
                "Sync brightness gain",
                0.1 / 1.5,
                |app: &mut App, fraction| {
                    app.set_sync_gain(SYNC_GAIN_MIN + fraction * (SYNC_GAIN_MAX - SYNC_GAIN_MIN));
                },
            ))
            .child(
                text("The gain scales the derived brightness from 50% to 200%. The derived colour is sent unchanged."
                    .to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
            ),
    );

    let mut body = col()
        .key("sync")
        .w_full()
        .max_w(760.0)
        .padding(32.0)
        .gap(20.0)
        .child(status)
        .child(monitor_section);
    if let Some(capture) = capture {
        body = body.child(capture);
    }
    body = body.child(gain);

    if let Some(report) = &app.sync.restore_report {
        body = body.child(section(
            k,
            "Restore",
            text(report.message())
                .font(k.ty.small)
                .color(if report.not_restored.is_empty() {
                    k.c.success.icon
                } else {
                    k.c.danger.icon
                })
                .wrap(),
        ));
    }

    let body = body
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    let body = row().w_full().h_full().justify_center().child(body);
    Page::new(body)
        .title("Screen sync")
        .leading(back)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .build(k)
}

fn monitor_row(app: &App, k: &Kit, monitor: &MonitorInfo) -> El {
    let selected = app.sync.selected_monitor.as_deref() == Some(monitor.name.as_str());
    let name = monitor.name.clone();
    row()
        .w_full()
        .padding(10.0)
        .gap(10.0)
        .center_y()
        .radius(k.radius.md)
        .fill(if selected {
            k.c.layer_04
        } else {
            Color::TRANSPARENT
        })
        .border(
            1.0,
            if selected {
                k.c.border_base
            } else {
                k.c.border_weak
            },
        )
        .on_press(move |app: &mut App| app.select_monitor(name.clone()))
        .child(el_radio(k, selected))
        .child(
            col()
                .grow(1.0)
                .gap(2.0)
                .child(
                    text(monitor.name.clone())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(format!(
                        "{} × {}{}",
                        monitor.width,
                        monitor.height,
                        if monitor.description.is_empty() {
                            String::new()
                        } else {
                            format!(" · {}", monitor.description)
                        }
                    ))
                    .font(k.ty.small)
                    .color(k.c.text_muted),
                ),
        )
}

fn el_radio(k: &Kit, selected: bool) -> El {
    rune::el()
        .size(16.0, 16.0)
        .radius(8.0)
        .border(
            1.5,
            if selected {
                k.c.bg_accent
            } else {
                k.c.border_base
            },
        )
        .center()
        .child(if selected {
            dot(k.c.bg_accent)
        } else {
            rune::el().size(0.0, 0.0)
        })
}

fn frame_age(app: &App) -> String {
    let Some(at) = app.sync.last_scene_at else {
        return "—".to_owned();
    };
    let elapsed = at.elapsed();
    if elapsed < Duration::from_secs(1) {
        format!("{} ms", elapsed.as_millis())
    } else {
        format!("{:.1} s", elapsed.as_secs_f32())
    }
}
