use rune::{El, ScrollSpec, TextStyle, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Menu, MenuItem};

use crate::app::App;
use crate::screen::gain::{SYNC_GAIN_MAX, SYNC_GAIN_MIN};
use crate::screen::sync::SyncStatus;
use crate::ui::widget::{
    SYNC, badge, dropdown_trigger, frame_age, notice, panel, panel_body, panel_header, rgb_color,
    slider, swatch, sync_status_tone,
};

pub fn view(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let active = app.sync.status.is_active();
    let running = app.sync.status.is_running();
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

    let capture = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Live capture",
                Some("One monitor drives every configured bulb."),
                vec![badge(
                    k,
                    &app.sync.status.label(),
                    sync_status_tone(k, &app.sync.status),
                )],
            ))
            .child(panel_body(capture_body(app, k)))
            .child(crate::ui::widget::panel_footer(k, vec![start, pause, stop])),
    );

    let monitor = monitor_panel(app, k, layers);
    let gain = gain_panel(app, k);

    let mut content = col()
        .key("sync")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(capture)
        .child(monitor)
        .child(gain);

    if let Some(report) = &app.sync.restore_report {
        let tone = if report.not_restored.is_empty() {
            k.c.success.icon
        } else {
            k.c.danger.icon
        };
        content = content.child(panel(
            k,
            col()
                .w_full()
                .child(panel_header(
                    k,
                    "Restore report",
                    Some("What Stop changed on the bulbs."),
                    vec![badge(
                        k,
                        if report.not_restored.is_empty() {
                            "Complete"
                        } else {
                            "Partial"
                        },
                        tone,
                    )],
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

fn capture_body(app: &App, k: &Kit) -> El {
    let derived = app
        .sync
        .derived
        .map_or_else(|| "No frame yet".to_owned(), |color| color.hex());
    let monitor = app
        .sync
        .selected_monitor
        .clone()
        .unwrap_or_else(|| "No monitor selected".to_owned());
    let mut body = col()
        .w_full()
        .gap(10.0)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(swatch(
                    k,
                    app.sync.derived.map_or(k.c.layer_03, rgb_color),
                    56.0,
                ))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(1.0)
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
                ),
        )
        .child(
            row()
                .w_full()
                .gap(10.0)
                .child(fact(k, "Frame", frame_age(app.sync.last_scene_at)))
                .child(fact(k, "Gain", format!("{:.1}×", app.sync.gain))),
        );
    if app.sync.stale {
        body = body.child(notice(
            k,
            &SYNC,
            "The last frame is older than five seconds. Capture recovers on the next good frame.",
            k.c.warning,
        ));
    } else if app.sync.flashing {
        body = body.child(notice(
            k,
            &SYNC,
            "The flash guard is holding brightness after a large scene reversal.",
            k.c.warning,
        ));
    }
    if let Some(error) = &app.sync.capture_error {
        body = body.child(notice(k, &SYNC, error, k.c.danger)).child(
            Button::new("Retry capture")
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Small)
                .on_press(|app: &mut App| app.restart_screen())
                .build(k),
        );
    }
    body
}

fn monitor_panel(app: &App, k: &Kit, layers: &mut Vec<El>) -> El {
    let mut body = col().w_full().gap(8.0);
    if app.sync.monitors.is_empty() {
        body = body.child(notice(
            k,
            &SYNC,
            "No monitors found. Capture needs a Wayland compositor with wlr-screencopy, such as Hyprland or Sway.",
            k.c.warning,
        ));
    } else {
        let current = app
            .sync
            .selected_monitor
            .clone()
            .unwrap_or_else(|| "Select a monitor".to_owned());
        let trigger = dropdown_trigger(
            k,
            "Monitor",
            &current,
            app.monitor_picker_open,
            None,
            |app: &mut App| app.monitor_picker_open = !app.monitor_picker_open,
        );
        let mut picker = Menu::new("monitor-picker")
            .trigger(trigger)
            .open(app.monitor_picker_open)
            .min_width(360.0)
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
        body = body.child(picker.build(k, layers));
    }
    panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Monitor",
                Some("Choose the output to capture."),
                Vec::new(),
            ))
            .child(panel_body(body)),
    )
}

fn gain_panel(app: &App, k: &Kit) -> El {
    let gain = ((app.sync.gain - SYNC_GAIN_MIN) / (SYNC_GAIN_MAX - SYNC_GAIN_MIN)).clamp(0.0, 1.0);
    let slider = slider(
        k,
        "sync-gain",
        gain,
        "Sync brightness gain",
        0.1 / (SYNC_GAIN_MAX - SYNC_GAIN_MIN),
        400.0,
        |app: &mut App, fraction| {
            app.set_sync_gain(SYNC_GAIN_MIN + fraction * (SYNC_GAIN_MAX - SYNC_GAIN_MIN));
        },
    );
    panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Capture settings",
                Some("Gain scales brightness, never colour, from 50% to 400%."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(4.0)
                    .child(
                        row()
                            .w_full()
                            .justify_between()
                            .gap(8.0)
                            .child(
                                text("Brightness gain".to_owned())
                                    .font(k.ty.ui_medium)
                                    .color(k.c.text_base),
                            )
                            .child(
                                text(format!("{:.1}×", app.sync.gain))
                                    .font(k.ty.mono)
                                    .color(k.c.text_base),
                            ),
                    )
                    .child(slider)
                    .child(
                        text(
                            "Stop bounds confirmed restore sends to about two seconds.".to_owned(),
                        )
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap(),
                    ),
            )),
    )
}

fn fact(k: &Kit, label: &str, value: impl Into<String>) -> El {
    row()
        .gap(5.0)
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
