use std::time::Duration;

use rune::{Cx, El, TextStyle, Tx, col, row, text};
use rune_kit::shell::{Page, Toolbar};
use rune_kit::{Kit, SegmentedControl};

use crate::app::App;
use crate::app::state::Screen;
use crate::ui::widget::BULB;

pub fn view(app: &App, cx: &mut Cx<App>, k: &Kit, main: El) -> El {
    let toolbar = Toolbar::new().leading(brand(k));
    let body = match &app.screen {
        Screen::Dashboard | Screen::Sync | Screen::Settings | Screen::Help => col()
            .w_full()
            .h_full()
            .min_h(0.0)
            .child(nav(app, k))
            .child(main),
        Screen::Form(_) | Screen::Remove(_) | Screen::Color => main,
    };
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
        Screen::Help => Some(3),
        Screen::Form(_) | Screen::Remove(_) | Screen::Color => None,
    };
    col().w_full().px(16.0).pt(12.0).pb(10.0).child(
        SegmentedControl::new(&["Control", "Sync", "Settings", "Help"], selected)
            .label("Sections")
            .width(None)
            .on_pick(|app: &mut App, index: usize, tx: &mut Tx| {
                match index {
                    0 => app.open_overview(),
                    1 => app.open_sync(),
                    2 => app.open_settings(),
                    _ => app.open_help(),
                }
                tx.wake_after(Duration::ZERO);
            })
            .build(k),
    )
}
