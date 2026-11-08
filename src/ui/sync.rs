use rune::{Color, El, ScrollSpec, TextStyle, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Divider, Kit};

use crate::app::App;
use crate::screen::gain::{SYNC_GAIN_MAX, SYNC_GAIN_MIN};
use crate::screen::monitor::MonitorInfo;
use crate::screen::sync::SyncStatus;
use crate::ui::widget::{
    SYNC, badge, dot, frame_age, notice, panel, panel_body, panel_footer, panel_header, rgb_color,
    setting_row, slider, stat, swatch, sync_status_tone,
};

pub fn view(app: &App, k: &Kit) -> El {
    let running = app.sync.status.is_running();
    let active = app.sync.status.is_active();
    let paused = app.sync.status == SyncStatus::Paused;
    let stopping = app.sync.status == SyncStatus::Stopping;

    let start = Button::new("Start sync")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Small)
        .disabled(active || stopping || app.sync.monitors.is_empty())
        .on_press(|app: &mut App| app.start_sync())
        .build(k);
    let pause = Button::new(if paused { "Resume" } else { "Pause" })
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Small)
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
        .control_size(ControlSize::Small)
        .disabled(!active)
        .on_press(|app: &mut App| app.stop_sync())
        .build(k);

    let stats = row()
        .w_full()
        .gap(12.0)
        .items_stretch()
        .child(stat(
            k,
            "Status",
            app.sync.status.label(),
            if running {
                "Capturing"
            } else {
                "Not capturing"
            },
            sync_status_tone(k, &app.sync.status),
        ))
        .child(stat(
            k,
            "Frame age",
            frame_age(app.sync.last_scene_at),
            "Captured at 1.5 Hz",
            k.c.info.icon,
        ))
        .child(stat(
            k,
            "Derived colour",
            app.sync
                .derived
                .map_or_else(|| "None".to_owned(), |color| color.hex()),
            format!("{}% brightness", app.sync.brightness),
            k.c.info.icon,
        ))
        .child(stat(
            k,
            "Brightness gain",
            format!("{:.1}×", app.sync.gain),
            "50–200% envelope",
            k.c.warning.icon,
        ));

    let derived = app
        .sync
        .derived
        .map_or_else(|| "No frame yet".to_owned(), |color| color.hex());
    let monitor = app
        .sync
        .selected_monitor
        .clone()
        .unwrap_or_else(|| "No monitor selected".to_owned());
    let mut capture_body = col()
        .w_full()
        .gap(12.0)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .center_y()
                .child(swatch(
                    k,
                    app.sync.derived.map_or(k.c.layer_03, rgb_color),
                    64.0,
                ))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(2.0)
                        .child(
                            text(derived)
                                .font(TextStyle::new(18.0, 600.0).lh(22.0))
                                .color(k.c.text_base),
                        )
                        .child(
                            text(format!("{} · {}% brightness", monitor, app.sync.brightness))
                                .font(k.ty.small)
                                .color(k.c.text_muted)
                                .truncate(),
                        ),
                )
                .child(badge(
                    k,
                    &app.sync.status.label(),
                    sync_status_tone(k, &app.sync.status),
                )),
        )
        .child(Divider::horizontal().build(k))
        .child(
            row()
                .w_full()
                .gap(10.0)
                .child(fact(k, "Frame", frame_age(app.sync.last_scene_at)))
                .child(fact(k, "Gain", format!("{:.1}×", app.sync.gain)))
                .child(fact(k, "Monitor", monitor)),
        );
    if app.sync.stale {
        capture_body = capture_body.child(notice(
            k,
            &SYNC,
            "The last frame is older than five seconds. Capture will recover on the next successful frame.",
            k.c.warning,
        ));
    } else if app.sync.flashing {
        capture_body = capture_body.child(notice(
            k,
            &SYNC,
            "The flash guard is holding brightness after a large scene reversal. Colour keeps updating.",
            k.c.warning,
        ));
    }
    if let Some(error) = &app.sync.capture_error {
        capture_body = capture_body.child(
            col()
                .w_full()
                .gap(8.0)
                .child(notice(k, &SYNC, error, k.c.danger))
                .child(
                    Button::new("Retry capture")
                        .variant(ButtonVariant::Outline)
                        .control_size(ControlSize::Small)
                        .on_press(|app: &mut App| app.restart_screen())
                        .build(k),
                ),
        );
    }

    let capture = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Live capture",
                Some("One monitor drives every configured bulb."),
                Vec::new(),
            ))
            .child(panel_body(capture_body))
            .child(panel_footer(k, vec![start, pause, stop])),
    );

    let monitors = monitor_panel(app, k);
    let settings = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Capture settings",
                Some("The gain scales brightness, never colour."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(10.0)
                    .child(
                        text("Brightness gain".to_owned())
                            .font(k.ty.ui_medium)
                            .color(k.c.text_base),
                    )
                    .child(
                        text("The derived brightness is multiplied by this value. Colour is unchanged.".to_owned())
                            .font(k.ty.small)
                            .color(k.c.text_muted)
                            .wrap(),
                    )
                    .child(
                        row()
                            .w_full()
                            .justify_end()
                            .child(
                                text(format!("{:.1}×", app.sync.gain))
                                    .font(k.ty.mono)
                                    .color(k.c.text_base),
                            ),
                    )
                    .child(slider(
                        k,
                        "sync-gain",
                        ((app.sync.gain - SYNC_GAIN_MIN) / (SYNC_GAIN_MAX - SYNC_GAIN_MIN))
                            .clamp(0.0, 1.0),
                        "Sync brightness gain",
                        0.1 / 1.5,
                        280.0,
                        |app: &mut App, fraction| {
                            app.set_sync_gain(
                                SYNC_GAIN_MIN + fraction * (SYNC_GAIN_MAX - SYNC_GAIN_MIN),
                            );
                        },
                    ))
                    .child(Divider::horizontal().build(k))
                    .child(setting_row(
                        k,
                        "Restore deadline",
                        "Stop bounds confirmed restore sends to about two seconds.",
                        text("~2 s".to_owned())
                            .font(k.ty.mono)
                            .color(k.c.text_muted),
                    )),
            )),
    );

    let right = col()
        .grow(1.0)
        .min_w(300.0)
        .gap(14.0)
        .child(monitors)
        .child(settings);

    let mut content = col()
        .key("sync")
        .w_full()
        .padding(18.0)
        .gap(14.0)
        .child(stats)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .items_start()
                .child(col().grow(2.0).min_w(0.0).child(capture))
                .child(right),
        );

    if let Some(report) = &app.sync.restore_report {
        let tone = if report.not_restored.is_empty() {
            k.c.success.icon
        } else {
            k.c.danger.icon
        };
        let label = if report.not_restored.is_empty() {
            "Complete"
        } else {
            "Partial"
        };
        content = content.child(panel(
            k,
            col()
                .w_full()
                .child(panel_header(
                    k,
                    "Restore report",
                    Some("What Stop changed on the bulbs."),
                    vec![badge(k, label, tone)],
                ))
                .child(panel_body(
                    text(report.message()).font(k.ty.small).color(tone).wrap(),
                )),
        ));
    }

    let scroll = content
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn monitor_panel(app: &App, k: &Kit) -> El {
    let mut rows = col().w_full().gap(6.0);
    if app.sync.monitors.is_empty() {
        rows = rows.child(notice(
            k,
            &SYNC,
            "No monitors found. Screen capture needs a Wayland compositor that implements wlr-screencopy, such as Hyprland or Sway.",
            k.c.warning,
        ));
    } else {
        for monitor in &app.sync.monitors {
            rows = rows.child(monitor_row(app, k, monitor));
        }
    }
    panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Monitors",
                Some("Choose the output to capture."),
                Vec::new(),
            ))
            .child(panel_body(rows)),
    )
}

fn monitor_row(app: &App, k: &Kit, monitor: &MonitorInfo) -> El {
    let selected = app.sync.selected_monitor.as_deref() == Some(monitor.name.as_str());
    let name = monitor.name.clone();
    row()
        .w_full()
        .padding(10.0)
        .gap(9.0)
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
        .child(radio(k, selected))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(monitor.name.clone())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .truncate(),
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
                    .color(k.c.text_muted)
                    .truncate(),
                ),
        )
}

fn radio(k: &Kit, selected: bool) -> El {
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

fn fact(k: &Kit, label: &str, value: impl Into<String>) -> El {
    row()
        .gap(6.0)
        .center_y()
        .child(
            text(format!("{label}: "))
                .font(k.ty.small)
                .color(k.c.text_faint),
        )
        .child(
            text(value.into())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .truncate(),
        )
}
