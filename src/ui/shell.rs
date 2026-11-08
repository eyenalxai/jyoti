use rune::{Cx, El, TextStyle, col, row, text};
use rune_kit::Kit;
use rune_kit::shell::{Page, Toolbar};

use crate::app::App;
use crate::app::state::Screen;
use crate::ui::widget::BULB;

const CONTENT_MAX_WIDTH: f32 = 484.0;

pub fn view(app: &App, cx: &mut Cx<App>, k: &Kit, main: El) -> El {
    let toolbar = Toolbar::new().center(brand(k));
    let content = match &app.screen {
        Screen::Dashboard | Screen::Sync | Screen::Settings => col()
            .w_full()
            .h_full()
            .min_h(0.0)
            .child(nav(app, k))
            .child(main),
        Screen::Form(_) | Screen::Remove(_) | Screen::Color => main,
    };
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

fn brand(k: &Kit) -> El {
    row()
        .gap(6.0)
        .center_y()
        .child(rune::icon(&BULB, 15.0).color(k.c.text_muted))
        .child(
            text("jyoti".to_owned())
                .font(TextStyle::new(15.0, 600.0).lh(18.0))
                .color(k.c.text_base),
        )
}

fn nav(app: &App, k: &Kit) -> El {
    let selected = match &app.screen {
        Screen::Dashboard => Some(0),
        Screen::Sync => Some(1),
        Screen::Settings => Some(2),
        Screen::Form(_) | Screen::Remove(_) | Screen::Color => None,
    };
    let labels = ["Control", "Sync", "Settings"];
    let mut tabs = row()
        .w_full()
        .h(32.0)
        .gap(2.0)
        .role("tablist")
        .label("Sections")
        .focus_group();
    for (index, label) in labels.into_iter().enumerate() {
        let on = selected == Some(index);
        let mut tab = row()
            .key(format!("tab-{index}"))
            .grow(1.0)
            .h(32.0)
            .center()
            .radius(k.radius.md)
            .role("tab")
            .label(label)
            .position_in_set(index + 1, labels.len())
            .focusable(true)
            .focus_selected(on)
            .focus_ring(2.0, 1.0, k.c.border_focus)
            .press(k.c.pressed)
            .on_press(move |app: &mut App| match index {
                0 => app.open_overview(),
                1 => app.open_sync(),
                _ => app.open_settings(),
            });
        if on {
            tab = tab
                .fill(k.c.bg_accent.alpha(0.14))
                .border(1.0, k.c.bg_accent.alpha(0.3))
                .child(
                    text(label.to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_accent)
                        .a11y_hidden(),
                );
        } else {
            tab = tab.hover(k.c.hover).child(
                text(label.to_owned())
                    .font(k.ty.ui_medium)
                    .color(k.c.text_muted)
                    .a11y_hidden(),
            );
        }
        tabs = tabs.child(tab);
    }
    row().w_full().px(16.0).pt(10.0).pb(8.0).child(tabs)
}
