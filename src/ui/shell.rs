use rune::{Cx, El, Spring, row};
use rune_kit::shell::{
    PaneMotion, Sidebar, SidebarPeek, SidebarRow, SidebarToggle, SplitView, Toolbar,
};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit};

use crate::app::App;
use crate::app::state::Screen;
use crate::bulb::aggregate::AggregatePower;
use crate::bulb::selection::Selection;
use crate::ui::dashboard;
use crate::ui::widget::{BULB, GEAR, HELP, PLUS, REFRESH, SYNC, dot, sync_status_tone};

pub fn view(app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let sidebar = sidebar(app, k);
    let main = dashboard::content(app, k);

    let add = IconButton::new(&PLUS)
        .variant(IconButtonVariant::GhostMuted)
        .help("Add bulb")
        .shortcut("cmd+n")
        .on_press(|app: &mut App| app.open_add())
        .build(k);
    let refresh = IconButton::new(&REFRESH)
        .variant(IconButtonVariant::GhostMuted)
        .help("Refresh bulbs")
        .shortcut("cmd+r")
        .on_press_tx(|app: &mut App, tx| app.refresh_all(tx))
        .build(k);
    let sync = IconButton::new(&SYNC)
        .variant(IconButtonVariant::GhostMuted)
        .help("Screen sync")
        .shortcut("cmd+s")
        .on_press(|app: &mut App| app.open_sync())
        .build(k);
    let settings = IconButton::new(&GEAR)
        .variant(IconButtonVariant::GhostMuted)
        .help("Settings")
        .shortcut("cmd+,")
        .on_press(|app: &mut App| app.open_settings())
        .build(k);

    let toolbar = Toolbar::new().title(app.selection_label()).trailing(
        row()
            .gap(2.0)
            .child(refresh)
            .child(sync)
            .child(settings)
            .child(add),
    );

    SplitView::new(main)
        .sidebar(&app.sidebar, |app: &mut App| &mut app.sidebar, sidebar)
        .toolbar(toolbar)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .sidebar_padding(18.0, 10.0)
        .sidebar_toggle(
            SidebarToggle::new()
                .size(ControlSize::Large)
                .past_lights(16.0)
                .edge(18.0)
                .gap(4.0)
                .in_header(),
        )
        .pane_motion(
            PaneMotion::new()
                .spring(Spring::new(0.3, 0.0))
                .scale_with_width(480.0, 0.15, 0.3)
                .fade_tail(0.15),
        )
        .sidebar_peek(
            SidebarPeek::new()
                .edge(16.0, std::time::Duration::from_millis(40))
                .toggle_delay(std::time::Duration::from_millis(80))
                .below(56.0)
                .hold(15.0, std::time::Duration::from_millis(100))
                .spring(Spring::new(0.18, 0.0)),
        )
        .reduced_motion(cx.is_reduced_motion())
        .build(k)
}

fn sidebar(app: &App, k: &Kit) -> El {
    let add = SidebarRow::new("Add bulb")
        .icon(&PLUS)
        .height(34.0)
        .on_select(|app: &mut App, _| app.open_add())
        .build(k)
        .key("sidebar-add");
    let mut sidebar = Sidebar::new()
        .label("Bulbs")
        .inset(0.0)
        .header_gap(4.0)
        .header_padding(0.0, 16.0)
        .row_gap(4.0)
        .header(add);

    if app.bulbs.len() >= 2 {
        let aggregate = app.aggregate();
        let selected = app.selection == Some(Selection::All);
        let tone = match aggregate.power {
            AggregatePower::On => k.c.success.icon,
            AggregatePower::Off | AggregatePower::Mixed => k.c.warning.icon,
            AggregatePower::Unknown => k.c.text_faint,
        };
        sidebar = sidebar.child(
            SidebarRow::new("All bulbs")
                .icon(&BULB)
                .height(38.0)
                .detail(format!("{}/{}", aggregate.reachable, aggregate.total))
                .trailing(dot(tone))
                .selected(selected)
                .on_select(|app: &mut App, _| app.selection = Some(Selection::All))
                .build(k)
                .key("sidebar-all"),
        );
    }

    for bulb in &app.bulbs {
        let selected = app.selection == Some(Selection::Bulb(bulb.id.clone()));
        let state = app.states.get(&bulb.id);
        let error = app.errors.get(&bulb.id);
        let unreachable = app.sync.unreachable.contains(&bulb.id);
        let tone = if error.is_some() || unreachable {
            k.c.danger.icon
        } else {
            match state.and_then(|state| state.power) {
                Some(true) => k.c.success.icon,
                Some(false) => k.c.text_faint,
                None => k.c.warning.icon,
            }
        };
        let subtitle = match state.and_then(|state| state.power) {
            Some(true) => format!("{} · on", bulb.address),
            Some(false) => format!("{} · off", bulb.address),
            None => format!("{} · connecting", bulb.address),
        };
        let id = bulb.id.clone();
        let edit_id = bulb.id.clone();
        sidebar = sidebar.child(
            SidebarRow::new(bulb.name.clone())
                .subtitle(subtitle)
                .height(46.0)
                .trailing(dot(tone))
                .selected(selected)
                .on_select(move |app: &mut App, _| {
                    app.selection = Some(Selection::Bulb(id.clone()));
                })
                .on_double_click(move |app: &mut App, _| app.open_edit(&edit_id))
                .build(k)
                .key(bulb.id.clone()),
        );
    }

    let sync_tone = sync_status_tone(k, &app.sync.status);
    sidebar
        .footer(
            SidebarRow::new("Screen sync")
                .icon(&SYNC)
                .height(32.0)
                .trailing(dot(sync_tone))
                .on_select(|app: &mut App, _| app.open_sync())
                .build(k)
                .key("sidebar-sync"),
        )
        .footer(
            SidebarRow::new("Settings")
                .icon(&GEAR)
                .height(32.0)
                .on_select(|app: &mut App, _| app.open_settings())
                .build(k)
                .key("sidebar-settings"),
        )
        .footer(
            SidebarRow::new("Help")
                .icon(&HELP)
                .height(32.0)
                .on_select(|app: &mut App, _| app.screen = Screen::Help)
                .build(k)
                .key("sidebar-help"),
        )
        .build(k)
}
