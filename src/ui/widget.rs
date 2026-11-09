use std::rc::Rc;
use std::time::{Duration, Instant};

use rune::{Align, Color, Cursor, Drag, El, Icon, Phase, Spring, Tx, col, el, row, text};
use rune_kit::{Kit, StateColors};

use crate::app::App;
use crate::app::sync::run::RunPhase;
use crate::ui::theme;

/// Side padding of a screen page.
pub const PAGE_PAD: f32 = 14.0;
/// The one slider width that fits the 480-point minimum page.
pub const SLIDER_WIDTH: f32 = 400.0;

pub static BULB: Icon = Icon {
    name: "jyoti-bulb",
    view: 16.0,
    body: r#"<path d="M8 2.2a4.1 4.1 0 0 0-2.6 7.3c.5.4.8 1 .9 1.6l.1.4h3.2l.1-.4c.1-.6.4-1.2.9-1.6A4.1 4.1 0 0 0 8 2.2Z" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M6.4 13.2h3.2M7 14.6h2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#,
};
pub static FLAME: Icon = Icon {
    name: "jyoti-flame",
    view: 16.0,
    body: r#"<path d="M8.2 1.8c.2 1.7 1.2 2.7 2 3.8.9 1.2 1.4 2.3 1.4 3.5a3.7 3.7 0 0 1-7.4 0c0-1.1.4-2.2 1.3-3.2.2.7.5 1.2 1 1.6.1-1.9.6-3.8 1.7-5.7Z" fill="currentColor"/>"#,
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
pub static ALERT: Icon = Icon {
    name: "jyoti-alert",
    view: 16.0,
    body: r#"<path d="M8 2.6 14 13H2L8 2.6Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/><path d="M8 6.4v3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><circle cx="8" cy="11.2" r=".6" fill="currentColor"/>"#,
};
pub static CHECK: Icon = Icon {
    name: "jyoti-check",
    view: 16.0,
    body: r#"<path d="M3.2 8.4 6.4 11.6 12.8 5.2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};
pub static COPY: Icon = Icon {
    name: "jyoti-copy",
    view: 16.0,
    body: r#"<rect x="5.6" y="3.2" width="7.2" height="9.2" rx="1.4" stroke="currentColor" stroke-width="1.1" fill="none"/><path d="M3.6 5.6v6.1a1.6 1.6 0 0 0 1.6 1.6h5.2" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" fill="none"/>"#,
};
pub static LOCK: Icon = Icon {
    name: "jyoti-lock",
    view: 16.0,
    body: r#"<rect x="3.8" y="7" width="8.4" height="6.2" rx="1.6" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M6 7V5.4a2 2 0 0 1 4 0V7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>"#,
};

pub fn sync_status_tone(k: &Kit, status: &RunPhase) -> Color {
    match status {
        RunPhase::Running(_) | RunPhase::Starting(_) => k.c.success.icon,
        RunPhase::Paused(_) | RunPhase::Stopping(_) => k.c.warning.icon,
        RunPhase::Failed { .. } => k.c.danger.icon,
        RunPhase::Idle => k.c.text_faint,
    }
}

/// The light field shared by Control and Sync: the exact fill, a lit top edge,
/// a grounded bottom and one soft colour halo.
pub fn stage_surface(k: &Kit, height: f32, fill: Color, glow: f32, plate: El) -> El {
    col()
        .w_full()
        .h(height)
        .min_h(height)
        .justify_end()
        .padding(12.0)
        .radius(k.radius.xl)
        .clip_soft(k.radius.xl)
        .fill(fill)
        .linear_gradient(
            std::f32::consts::PI,
            [
                (0.0, k.c.white.alpha(0.26)),
                (0.38, k.c.white.alpha(0.02)),
                (1.0, theme::ink().alpha(0.16)),
            ],
        )
        .border(1.0, theme::ink().alpha(0.1))
        .shadow(0.0, 22.0, fill.alpha(glow))
        .child(plate)
}

/// The cream plate a stage's readout is printed on, in both appearances.
pub fn nameplate(k: &Kit, body: El) -> El {
    let plate = col()
        .w_full()
        .padding(14.0)
        .radius(k.radius.md)
        .fill(theme::plate())
        .border(1.0, theme::ink().alpha(0.08))
        .child(body);
    k.elevate(plate, &k.elevation.floating)
}

/// Dims a control while a sync run owns it, without hiding its value.
pub fn dim_when(e: El, disabled: bool) -> El {
    if disabled {
        e.inert().layer_opacity(0.5)
    } else {
        e
    }
}

/// Mutually exclusive choices behind one control, separated so they read as
/// distinct choices rather than one split label. The kit's segmented control has
/// no gap between choices; the mode and appearance controls need that separation.
pub fn segmented_switch(
    k: &Kit,
    key: &str,
    label: &str,
    labels: &[&str],
    selected: Option<usize>,
    on_pick: impl Fn(&mut App, usize, &mut Tx) + 'static,
) -> El {
    let pick = Rc::new(on_pick);
    let count = labels.len();
    let mut root = row()
        .key(key)
        .role("radiogroup")
        .label(label.to_owned())
        .focus_group()
        .w_full()
        .h(32.0)
        .gap(6.0)
        .padding(2.0)
        .radius(k.radius.md)
        .fill(k.c.layer_01);
    for (index, label) in labels.iter().enumerate() {
        let on = selected == Some(index);
        let pick = Rc::clone(&pick);
        let mut segment = row()
            .key(format!("{key}-{index}"))
            .role("radio")
            .label((*label).to_owned())
            .position_in_set(index + 1, count)
            .focusable(true)
            .focus_selected(on)
            .focus_ring(2.0, 1.0, k.c.border_focus)
            .grow(1.0)
            .h(28.0)
            .center()
            .radius(k.radius.sm)
            .motion(Spring::new(0.25, 0.1))
            .on_press_tx(move |app: &mut App, tx: &mut Tx| pick(app, index, tx))
            .child(
                text((*label).to_owned())
                    .font(k.ty.ui_medium)
                    .color(if on { k.c.text_base } else { k.c.text_muted })
                    .a11y_hidden(),
            );
        if on {
            segment = k
                .elevate(segment.fill(k.c.bg_base), &k.elevation.knob)
                .border(1.0, k.c.border_strong);
        } else {
            segment = segment.hover(k.c.hover).press(k.c.pressed);
        }
        root = root.child(segment);
    }
    root
}

/// A raised surface. One family for every panel: the page shows the paper, the
/// panel catches the light.
pub fn surface(k: &Kit, body: El) -> El {
    let panel = col()
        .w_full()
        .min_w(0.0)
        .radius(k.radius.lg)
        .clip_soft(k.radius.lg)
        .fill(k.c.bg_base)
        .border(1.0, k.c.border_muted)
        .child(body);
    k.elevate(panel, &k.elevation.raised)
}

pub fn hairline(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden()
}

pub fn label(k: &Kit, value: &str) -> El {
    text(value.to_owned())
        .font(theme::label())
        .color(k.c.text_muted)
}

pub fn group(k: &Kit, caption: &str, body: El) -> El {
    col()
        .w_full()
        .min_w(0.0)
        .gap(7.0)
        .child(row().w_full().child(label(k, caption)))
        .child(surface(k, body))
}

pub fn setting_row(k: &Kit, title: &str, description: &str, value: El) -> El {
    row()
        .w_full()
        .min_w(0.0)
        .gap(16.0)
        .center_y()
        .px(14.0)
        .py(10.0)
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
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
                ),
        )
        .child(value)
}

pub fn fact_row(k: &Kit, title: &str, value: impl Into<String>) -> El {
    row()
        .w_full()
        .justify_between()
        .gap(16.0)
        .center_y()
        .py(8.0)
        .child(
            text(title.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(value_text(k, value))
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
        .h(40.0)
        .pl(12.0)
        .pr(9.0)
        .gap(9.0)
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
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .truncate()
                .grow(1.0)
                .min_w(0.0),
        )
        .child(rune::icon(chevron, 18.0).color(k.c.icon_muted))
}

pub fn subpage_header(k: &Kit, title: &str, description: &str, back: El) -> El {
    row().w_full().gap(10.0).center_y().child(back).child(
        col()
            .grow(1.0)
            .min_w(0.0)
            .gap(1.0)
            .child(
                text(title.to_owned())
                    .font(theme::display(24.0, 580.0))
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

pub fn notice(k: &Kit, icon: &'static Icon, message: &str, state: StateColors) -> El {
    row()
        .w_full()
        .padding(10.0)
        .gap(9.0)
        .center_y()
        .radius(k.radius.md)
        .fill(state.bg)
        .border(1.0, state.border)
        .child(rune::icon(icon, 15.0).color(state.icon).a11y_hidden())
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
    el().size(7.0, 7.0).radius(4.0).fill(color).a11y_hidden()
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

#[derive(Clone, Copy)]
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
    let (w, h, knob) = (width, 40.0, 20.0);
    let track_is_fill = matches!(track, SliderTrack::Fill);
    let bar = if track_is_fill { 5.0 } else { 10.0 };
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
        .cursor(Cursor::Pointer)
        .focusable(true)
        .focus_ring(2.0, 2.0, k.c.border_focus)
        .motion(Spring::new(0.2, 0.0))
        .on_step({
            let change = Rc::clone(&on_change);
            move |app: &mut App, steps, tx: &mut Tx| {
                let next = (start + steps as f32 * step).clamp(0.0, 1.0);
                change(app, next);
                tx.instant();
                tx.wake_after(Duration::from_millis(160));
            }
        })
        .on_drag(move |app: &mut App, drag: &mut Drag, tx| match drag.phase {
            Phase::Down => {
                drag.data = value;
                let knob_x = knob / 2.0 + value * travel;
                if (drag.x - knob_x).abs() > knob / 2.0 + 2.0 {
                    change(app, to_value(drag.x));
                    tx.instant();
                    tx.wake_after(Duration::from_millis(160));
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
                tx.wake_after(Duration::from_millis(160));
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
            .border(1.0, k.c.border_base)
            .shadow(1.0, 3.0, k.c.black.alpha(0.18))
            .shadow(0.0, 0.5, k.c.depth_bottom)
            .a11y_hidden(),
    )
}

pub fn frame_age(at: Option<Instant>) -> String {
    let Some(at) = at else {
        return "—".to_owned();
    };
    format_frame_age(at.elapsed())
}

fn format_frame_age(elapsed: Duration) -> String {
    if elapsed < Duration::from_secs(10) {
        format!("{} ms", elapsed.as_millis())
    } else {
        format!("{:.1} s", elapsed.as_secs_f32())
    }
}

pub fn value_text(k: &Kit, value: impl Into<String>) -> El {
    text(value.into())
        .font(theme::number(13.0, 560.0))
        .color(k.c.text_base)
        .align(Align::End)
}

pub fn error_text(k: &Kit, message: &str) -> El {
    text(message.to_owned())
        .font(k.ty.small)
        .color(k.c.danger.icon)
        .wrap()
}

#[cfg(test)]
mod tests {
    use super::format_frame_age;
    use std::time::Duration;

    #[test]
    fn frame_age_switches_to_seconds_after_ten() {
        assert_eq!(format_frame_age(Duration::from_millis(0)), "0 ms");
        assert_eq!(format_frame_age(Duration::from_millis(163)), "163 ms");
        assert_eq!(format_frame_age(Duration::from_millis(9_999)), "9999 ms");
        assert_eq!(format_frame_age(Duration::from_millis(10_000)), "10.0 s");
        assert_eq!(format_frame_age(Duration::from_millis(12_340)), "12.3 s");
    }
}
