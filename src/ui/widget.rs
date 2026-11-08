use std::rc::Rc;
use std::time::{Duration, Instant};

use rune::{Align, Color, Drag, El, Icon, Phase, Spring, TextStyle, Tx, col, el, row, text};
use rune_kit::{Divider, Kit, StateColors};

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

pub static PLUS: Icon = Icon {
    name: "jyoti-plus",
    view: 16.0,
    body: r#"<path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#,
};
pub static SYNC: Icon = Icon {
    name: "jyoti-sync",
    view: 16.0,
    body: r#"<path d="M3 6.5A3 3 0 0 1 6 4h5M10.5 2.5 13 4l-2.5 1.5M13 9.5A3 3 0 0 1 10 12H5M5.5 13.5 3 12l2.5-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};
pub static REFRESH: Icon = Icon {
    name: "jyoti-refresh",
    view: 16.0,
    body: r#"<path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.6v3h-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};
pub static BACK: Icon = Icon {
    name: "jyoti-back",
    view: 16.0,
    body: r#"<path d="M9.8 3.8 5.6 8l4.2 4.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};
pub static MORE: Icon = Icon {
    name: "jyoti-more",
    view: 16.0,
    body: r#"<circle cx="3.6" cy="8" r="1.05" fill="currentColor"/><circle cx="8" cy="8" r="1.05" fill="currentColor"/><circle cx="12.4" cy="8" r="1.05" fill="currentColor"/>"#,
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

pub fn panel(k: &Kit, body: El) -> El {
    col()
        .w_full()
        .min_w(0.0)
        .radius(k.radius.lg)
        .clip_soft(k.radius.lg)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_weak)
        .child(body)
}

pub fn panel_header(k: &Kit, title: &str, description: Option<&str>, actions: Vec<El>) -> El {
    let mut heading = col().grow(1.0).min_w(0.0).gap(2.0).child(
        text(title.to_owned())
            .font(k.ty.ui_medium)
            .color(k.c.text_base)
            .truncate(),
    );
    if let Some(description) = description {
        heading = heading.child(
            text(description.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        );
    }
    let mut header = row()
        .w_full()
        .px(14.0)
        .py(11.0)
        .gap(12.0)
        .center_y()
        .child(heading);
    if !actions.is_empty() {
        header = header.child(row().gap(6.0).center_y().children(actions));
    }
    col()
        .w_full()
        .child(header)
        .child(Divider::horizontal().build(k))
}

pub fn panel_body(body: El) -> El {
    col()
        .w_full()
        .min_w(0.0)
        .padding(14.0)
        .gap(12.0)
        .child(body)
}

pub fn panel_footer(k: &Kit, actions: Vec<El>) -> El {
    col().w_full().child(Divider::horizontal().build(k)).child(
        row()
            .w_full()
            .px(14.0)
            .py(10.0)
            .gap(8.0)
            .justify_end()
            .children(actions),
    )
}

pub fn setting_block(k: &Kit, title: &str, description: &str, control: El) -> El {
    col()
        .w_full()
        .min_w(0.0)
        .items_start()
        .gap(4.0)
        .py(10.0)
        .child(
            text(title.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(
            text(description.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(control)
}

pub fn dropdown_trigger(
    k: &Kit,
    width: f32,
    label: &str,
    value: &str,
    open: bool,
    status: Option<Color>,
    on_press: impl Fn(&mut App) + 'static,
) -> El {
    let chevron = if open {
        k.glyph.chevron_up
    } else {
        k.glyph.chevron_down
    };
    let mut trigger = row()
        .w(width)
        .h(36.0)
        .pl(10.0)
        .pr(8.0)
        .gap(8.0)
        .center_y()
        .role("button")
        .label(label.to_owned())
        .a11y_value(value.to_owned())
        .expanded(open)
        .radius(k.radius.md)
        .fill(k.c.bg_base)
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .focusable(true)
        .focus_ring(2.0, 1.0, k.c.border_focus)
        .on_press(on_press);
    if let Some(status) = status {
        trigger = trigger.child(dot(status));
    }
    trigger
        .child(
            text(value.to_owned())
                .font(k.ty.ui_compact)
                .color(k.c.text_base)
                .truncate()
                .grow(1.0)
                .min_w(0.0),
        )
        .child(rune::icon(chevron, 18.0).color(k.c.icon_muted))
}

pub fn subpage_header(k: &Kit, title: &str, description: &str, back: El) -> El {
    row().w_full().gap(8.0).center_y().child(back).child(
        col()
            .grow(1.0)
            .min_w(0.0)
            .gap(1.0)
            .child(
                text(title.to_owned())
                    .font(TextStyle::new(18.0, 600.0).lh(23.0))
                    .color(k.c.text_base)
                    .truncate(),
            )
            .child(
                text(description.to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .truncate(),
            ),
    )
}

pub fn fact_row(k: &Kit, label: &str, value: impl Into<String>) -> El {
    row()
        .w_full()
        .justify_between()
        .gap(16.0)
        .py(7.0)
        .center_y()
        .child(
            text(label.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(value_text(k, value))
}

pub fn notice(k: &Kit, icon: &'static Icon, message: &str, state: StateColors) -> El {
    row()
        .w_full()
        .padding(10.0)
        .gap(8.0)
        .center_y()
        .radius(k.radius.md)
        .fill(state.bg)
        .border(1.0, state.border)
        .child(rune::icon(icon, 16.0).color(state.icon).a11y_hidden())
        .child(
            text(message.to_owned())
                .font(k.ty.small)
                .color(state.fg)
                .wrap()
                .grow(1.0)
                .min_w(0.0),
        )
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
    el().size(8.0, 8.0).radius(4.0).fill(color)
}

pub fn badge(k: &Kit, label: &str, tone: Color) -> El {
    row()
        .role("status")
        .label(label.to_owned())
        .gap(6.0)
        .px(9.0)
        .h(24.0)
        .center_y()
        .radius(12.0)
        .fill(tone.alpha(0.12))
        .child(dot(tone).a11y_hidden())
        .child(
            text(label.to_owned())
                .font(k.ty.small)
                .color(tone)
                .a11y_hidden(),
        )
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
    width: f32,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    slider_inner(
        k,
        value,
        SliderConfig {
            key,
            label,
            step,
            width,
            track: SliderTrack::Fill,
        },
        on_change,
    )
}

pub struct SliderSpec<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub step: f32,
    pub width: f32,
}

pub fn gradient_slider(
    k: &Kit,
    value: f32,
    spec: SliderSpec<'_>,
    stops: Vec<(f32, Color)>,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    slider_inner(
        k,
        value,
        SliderConfig {
            key: spec.key,
            label: spec.label,
            step: spec.step,
            width: spec.width,
            track: SliderTrack::Gradient(stops),
        },
        on_change,
    )
}

enum SliderTrack {
    Fill,
    Gradient(Vec<(f32, Color)>),
}

struct SliderConfig<'a> {
    key: &'a str,
    label: &'a str,
    step: f32,
    width: f32,
    track: SliderTrack,
}

fn slider_inner(
    k: &Kit,
    value: f32,
    config: SliderConfig<'_>,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    let SliderConfig {
        key,
        label,
        step,
        width,
        track,
    } = config;
    let (w, h, knob) = (width, 28.0, 18.0);
    let track_is_fill = matches!(track, SliderTrack::Fill);
    let bar = if track_is_fill { 4.0 } else { 8.0 };
    let travel = w - knob;
    let value = value.clamp(0.0, 1.0);
    let on_change = Rc::new(on_change);
    let to_value = move |x: f32| ((x - knob / 2.0) / travel).clamp(0.0, 1.0);
    let change = Rc::clone(&on_change);
    let start = value;

    let mut root = el()
        .key(key)
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
        });
    let track = match track {
        SliderTrack::Fill => el()
            .key("track")
            .size(w, bar)
            .absolute(0.0, (h - bar) / 2.0)
            .radius(bar / 2.0)
            .fill(k.c.layer_03)
            .a11y_hidden(),
        SliderTrack::Gradient(stops) => el()
            .key("track")
            .size(w, bar)
            .absolute(0.0, (h - bar) / 2.0)
            .radius(bar / 2.0)
            .linear_gradient(std::f32::consts::FRAC_PI_2, stops)
            .border(0.5, k.c.border_base)
            .a11y_hidden(),
    };
    root = root.child(track);
    if track_is_fill {
        root = root.child(
            el().key("fill")
                .size(knob / 2.0 + value * travel, bar)
                .absolute(0.0, (h - bar) / 2.0)
                .radius(bar / 2.0)
                .fill(k.c.bg_accent)
                .a11y_hidden(),
        );
    }
    root.child(
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
    let mut root = row()
        .w(width)
        .h(36.0)
        .padding(3.0)
        .gap(2.0)
        .role("radiogroup")
        .label(label.to_owned())
        .focus_group()
        .radius(10.0)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base);
    for (index, option) in labels.iter().enumerate() {
        let on = selected == Some(index);
        let mut choice = row()
            .key(format!("choice-{index}"))
            .grow(1.0)
            .h(30.0)
            .center()
            .radius(7.0)
            .role("radio")
            .label((*option).to_owned())
            .position_in_set(index + 1, labels.len())
            .focusable(true)
            .focus_selected(on)
            .focus_ring(2.0, 1.0, k.c.border_focus)
            .on_press_tx(move |app: &mut App, tx: &mut Tx| {
                on_pick(app, index);
                tx.wake_after(Duration::ZERO);
            });
        if on {
            choice = choice
                .fill(k.c.bg_accent.alpha(0.16))
                .border(1.0, k.c.bg_accent.alpha(0.3))
                .child(
                    text((*option).to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_accent)
                        .a11y_hidden(),
                );
        } else {
            choice = choice.hover(k.c.hover).press(k.c.pressed).child(
                text((*option).to_owned())
                    .font(k.ty.ui_medium)
                    .color(k.c.text_muted)
                    .a11y_hidden(),
            );
        }
        root = root.child(choice);
    }
    root
}

pub fn frame_age(at: Option<Instant>) -> String {
    let Some(at) = at else {
        return "—".to_owned();
    };
    let elapsed = at.elapsed();
    if elapsed < Duration::from_secs(1) {
        format!("{} ms", elapsed.as_millis())
    } else {
        format!("{:.1} s", elapsed.as_secs_f32())
    }
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
