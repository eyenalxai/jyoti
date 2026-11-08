use std::rc::Rc;

use rune::{Align, Color, Drag, El, Icon, Phase, Spring, Tx, col, el, row, text};
use rune_kit::{Kit, SegmentedControl};

use crate::app::App;
use crate::screen::sync::SyncStatus;

pub static BULB: Icon = Icon {
    name: "jyoti-bulb",
    view: 16.0,
    body: r#"<path d="M8 2.2a4.1 4.1 0 0 0-2.6 7.3c.5.4.8 1 .9 1.6l.1.4h3.2l.1-.4c.1-.6.4-1.2.9-1.6A4.1 4.1 0 0 0 8 2.2Z" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M6.4 13.2h3.2M7 14.6h2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#,
};
pub static EDIT: Icon = Icon {
    name: "jyoti-edit",
    view: 16.0,
    body: r#"<path d="M3 13h2.4l7.2-7.2-2.4-2.4L3 10.6V13Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/><path d="M9.4 4.2l2.4 2.4" stroke="currentColor" stroke-width="1.2"/>"#,
};
pub static TRASH: Icon = Icon {
    name: "jyoti-trash",
    view: 16.0,
    body: r#"<path d="M3.5 4.5h9M6 4.5V3h4v1.5M4.5 4.5l.5 8.5h6l.5-8.5M6.5 7v4M9.5 7v4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>"#,
};
pub static REFRESH: Icon = Icon {
    name: "jyoti-refresh",
    view: 16.0,
    body: r#"<path d="M12.8 6.2A5 5 0 1 0 13 9.7" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/><path d="M12.8 3.4v3h-3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};
pub static PLUS: Icon = Icon {
    name: "jyoti-plus",
    view: 16.0,
    body: r#"<path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#,
};
pub static GEAR: Icon = Icon {
    name: "jyoti-gear",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="2.2" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M8 2.2v1.5M8 12.3v1.5M2.2 8h1.5M12.3 8h1.5M3.9 3.9l1 1M11.1 11.1l1 1M12.1 3.9l-1 1M4.9 11.1l-1 1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#,
};
pub static SYNC: Icon = Icon {
    name: "jyoti-sync",
    view: 16.0,
    body: r#"<path d="M3 6.5A3 3 0 0 1 6 4h5M10.5 2.5 13 4l-2.5 1.5M13 9.5A3 3 0 0 1 10 12H5M5.5 13.5 3 12l2.5-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};
pub static HELP: Icon = Icon {
    name: "jyoti-help",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="5.6" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M6.5 6.2a1.6 1.6 0 1 1 2.4 1.4c-.6.4-1 .8-1 1.5M8 11.4v.1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>"#,
};
pub static BACK: Icon = Icon {
    name: "jyoti-back",
    view: 16.0,
    body: r#"<path d="M9.5 3.5 5 8l4.5 4.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub fn sync_status_tone(k: &Kit, status: &SyncStatus) -> Color {
    match status {
        SyncStatus::Running | SyncStatus::Starting => k.c.success.icon,
        SyncStatus::Paused | SyncStatus::Stopping => k.c.warning.icon,
        SyncStatus::Error(_) => k.c.danger.icon,
        SyncStatus::Restored => k.c.info.icon,
        SyncStatus::Idle => k.c.text_faint,
    }
}

pub fn shortcut_text(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        keys.to_owned()
    } else {
        keys.replace('⌘', "Ctrl+")
    }
}

pub fn rgb_color(rgb: crate::bulb::model::Rgb) -> Color {
    Color {
        r: f32::from(rgb.red) / 255.0,
        g: f32::from(rgb.green) / 255.0,
        b: f32::from(rgb.blue) / 255.0,
        a: 1.0,
    }
}

pub fn dot(color: Color) -> El {
    el().size(7.0, 7.0).radius(3.5).fill(color)
}

pub fn card(k: &Kit, body: El) -> El {
    col()
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .radius(k.radius.lg)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_weak)
        .child(body)
}

pub fn section(k: &Kit, title: &str, body: El) -> El {
    col()
        .w_full()
        .gap(8.0)
        .child(
            text(title.to_owned())
                .font(k.ty.small_medium)
                .color(k.c.text_muted),
        )
        .child(card(k, body))
}

pub fn badge(k: &Kit, label: &str, tone: Color) -> El {
    row()
        .gap(5.0)
        .px(8.0)
        .h(22.0)
        .center_y()
        .radius(11.0)
        .fill(tone.alpha(0.12))
        .child(dot(tone))
        .child(text(label.to_owned()).font(k.ty.small).color(tone))
        .a11y_hidden()
}

pub fn swatch(k: &Kit, color: Color, size: f32) -> El {
    el().size(size, size)
        .radius(if size >= 24.0 {
            k.radius.md
        } else {
            k.radius.sm
        })
        .fill(color)
        .border(1.0, k.c.border_base)
}

pub fn slider(
    k: &Kit,
    key: &str,
    value: f32,
    label: &str,
    step: f32,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    let (w, h, knob, bar) = (260.0, 24.0, 18.0, 4.0);
    let travel = w - knob;
    let value = value.clamp(0.0, 1.0);
    let on_change = Rc::new(on_change);
    let to_value = move |x: f32| ((x - knob / 2.0) / travel).clamp(0.0, 1.0);
    let change = Rc::clone(&on_change);
    let start = value;

    el().key(key)
        .role("slider")
        .label(label.to_owned())
        .range_value(f64::from(value), 0.0, 1.0)
        .range_step(f64::from(step))
        .size(w, h)
        .motion(Spring::new(0.2, 0.0))
        .on_step({
            let change = Rc::clone(&on_change);
            move |app: &mut App, steps, tx: &mut Tx| {
                let next = (start + steps as f32 * step).clamp(0.0, 1.0);
                change(app, next);
                tx.instant();
                tx.wake_after(std::time::Duration::from_millis(160));
            }
        })
        .on_drag(move |app: &mut App, drag: &mut Drag, tx| match drag.phase {
            Phase::Down => {
                drag.data = value;
                let knob_x = knob / 2.0 + value * travel;
                if (drag.x - knob_x).abs() > knob / 2.0 + 2.0 {
                    change(app, to_value(drag.x));
                    tx.instant();
                    tx.wake_after(std::time::Duration::from_millis(160));
                }
            }
            Phase::Move => {
                let knob_x = knob / 2.0 + drag.data * travel;
                let grab = if (drag.start_x - knob_x).abs() <= knob / 2.0 + 2.0 {
                    drag.start_x - knob_x
                } else {
                    0.0
                };
                change(app, to_value(drag.x - grab));
                tx.instant();
                tx.wake_after(std::time::Duration::from_millis(160));
            }
            Phase::Up => {
                if drag.cancelled() {
                    change(app, drag.data);
                }
            }
        })
        .child(
            el().key("track")
                .size(w, bar)
                .absolute(0.0, (h - bar) / 2.0)
                .radius(bar / 2.0)
                .fill(k.c.layer_03)
                .a11y_hidden(),
        )
        .child(
            el().key("fill")
                .size(knob / 2.0 + value * travel, bar)
                .absolute(0.0, (h - bar) / 2.0)
                .radius(bar / 2.0)
                .fill(k.c.bg_accent)
                .a11y_hidden(),
        )
        .child(
            el().key("knob")
                .size(knob, knob)
                .absolute(value * travel, (h - knob) / 2.0)
                .radius(knob / 2.0)
                .fill(k.c.knob)
                .shadow(1.0, 3.0, k.c.black.alpha(0.18))
                .shadow(0.0, 0.5, k.c.depth_bottom)
                .a11y_hidden(),
        )
}

pub fn segmented(
    k: &Kit,
    labels: &[&str],
    selected: Option<usize>,
    label: &str,
    width: f32,
    on_pick: fn(&mut App, usize),
) -> El {
    SegmentedControl::new(labels, selected)
        .label(label)
        .width(Some(width))
        .on_pick(move |app: &mut App, index, tx: &mut Tx| {
            on_pick(app, index);
            tx.wake_after(std::time::Duration::ZERO);
        })
        .build(k)
}

pub fn shortcut_row(k: &Kit, label: &str, keys: impl Into<String>) -> El {
    row()
        .w_full()
        .justify_between()
        .gap(16.0)
        .child(text(label.to_owned()).font(k.ty.ui).color(k.c.text_base))
        .child(text(keys.into()).font(k.ty.mono).color(k.c.text_muted))
}

pub fn value_text(k: &Kit, value: impl Into<String>) -> El {
    text(value.into())
        .font(k.ty.mono)
        .color(k.c.text_base)
        .align(Align::End)
}

pub fn error_text(k: &Kit, message: &str) -> El {
    text(message.to_owned())
        .font(k.ty.small)
        .color(k.c.danger.icon)
        .wrap()
}
