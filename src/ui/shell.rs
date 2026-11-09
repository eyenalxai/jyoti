use rune::{Color, Edge, El, Spring, SpringPair, Transition, col, row, text};
use rune_kit::Kit;
use rune_kit::shell::{Page, Toolbar};

use crate::app::App;
use crate::app::state::{NavDirection, Screen};
use crate::ui::theme;
use crate::ui::widget::{FLAME, PAGE_PAD, dot, sync_status_tone};

const CONTENT_MAX_WIDTH: f32 = 484.0;
const TAB_HEIGHT: f32 = 40.0;

pub fn view(app: &App, cx: &mut rune::Cx<App>, k: &Kit, main: El) -> El {
    let mut toolbar = Toolbar::new().center(brand(k));
    if let Some(status) = sync_status(app, k) {
        toolbar = toolbar.trailing(status);
    }
    let edge = match app.navigation.direction {
        NavDirection::Forward => Edge::Trailing,
        NavDirection::Backward => Edge::Leading,
    };
    let entering = app.navigation.pending.is_some() || app.navigation.entering_until.is_some();
    let mut screen = col()
        .key(format!("screen-{}", app.screen.element_key()))
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .fill(k.c.bg_base)
        .motion(Spring::new(0.34, 0.06))
        .transition(push(edge));
    if entering {
        screen = screen.instant_changes();
    }
    let content = col()
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(nav(app, k))
        .child(screen.child(main));
    let body = col().w_full().h_full().min_h(0.0).items_center().child(
        col()
            .w_full()
            .h_full()
            .min_h(0.0)
            .max_w(CONTENT_MAX_WIDTH)
            .child(content),
    );
    Page::new(body)
        .toolbar(toolbar)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .build(k)
}

/// A page push that travels as one surface. The kit's push fades the incoming
/// page in; a section change should arrive whole and slide. An exiting page
/// still fades, because presence always fades a removal.
fn push(edge: Edge) -> Transition {
    let mut transition = Transition::push(edge);
    transition.insertion.opacity = 1.0;
    transition.springs = SpringPair::new(Spring::new(0.34, 0.07), Spring::new(0.27, 0.03));
    transition
}

fn brand(k: &Kit) -> El {
    row()
        .gap(7.0)
        .center_y()
        .child(rune::icon(&FLAME, 16.0).color(k.c.bg_accent).a11y_hidden())
        .child(
            text("jyoti".to_owned())
                .font(theme::wordmark())
                .color(k.c.text_base),
        )
}

fn sync_status(app: &App, k: &Kit) -> Option<El> {
    if app.sync.status == crate::screen::sync::SyncStatus::Idle {
        return None;
    }
    let tone = sync_status_tone(k, &app.sync.status);
    Some(
        row().gap(6.0).center_y().pr(4.0).child(dot(tone)).child(
            text(format!("Sync {}", app.sync.status.label().to_lowercase()))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .max_w(170.0)
                .truncate(),
        ),
    )
}

fn nav(app: &App, k: &Kit) -> El {
    let selected = match &app.screen {
        Screen::Dashboard | Screen::Form(_) | Screen::Remove(_) | Screen::Color => Some(0),
        Screen::Sync => Some(1),
        Screen::Settings => Some(2),
    };
    let labels = ["Control", "Sync", "Settings"];
    let mut tabs = row()
        .w_full()
        .h(TAB_HEIGHT)
        .role("tablist")
        .label("Sections")
        .focus_group();
    for (index, label) in labels.into_iter().enumerate() {
        let on = selected == Some(index);
        let mut tab = col()
            .key(format!("tab-{index}"))
            .grow(1.0)
            .h(TAB_HEIGHT)
            .justify_end()
            .radius(k.radius.sm)
            .role("tab")
            .label(label)
            .position_in_set(index + 1, labels.len())
            .focusable(true)
            .focus_selected(on)
            .focus_ring(2.0, -1.0, k.c.border_focus)
            .hover(k.c.hover)
            .on_press(move |app: &mut App| match index {
                0 => app.open_overview(),
                1 => app.open_sync(),
                _ => app.open_settings(),
            });
        let ink = if on { k.c.text_accent } else { k.c.text_muted };
        tab = tab.child(
            row().w_full().h(TAB_HEIGHT - 2.0).center().child(
                text(label.to_owned())
                    .font(k.ty.ui_medium)
                    .color(ink)
                    .group_color(k.c.text_base)
                    .a11y_hidden(),
            ),
        );
        tab = tab.child(
            col()
                .w_full()
                .h(2.0)
                .radius(1.0)
                .fill(if on {
                    Color {
                        a: 1.0,
                        ..k.c.bg_accent
                    }
                } else {
                    Color {
                        a: 0.0,
                        ..k.c.bg_accent
                    }
                })
                .a11y_hidden(),
        );
        tabs = tabs.child(tab);
    }
    col().w_full().px(PAGE_PAD).pt(3.0).pb(2.0).child(tabs)
}
