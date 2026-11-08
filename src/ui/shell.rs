use rune::{Cx, El, Spring, TextStyle, Tx, row, text};
use rune_kit::shell::{
    PaneMotion, Sidebar, SidebarHeader, SidebarPeek, SidebarRow, SidebarToggle, SplitView, Toolbar,
};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit};

use crate::app::App;
use crate::app::state::{FormMode, Screen};
use crate::bulb::aggregate::AggregatePower;
use crate::bulb::selection::Selection;
use crate::ui::widget::{BULB, GEAR, HELP, PLUS, SYNC, dot, sync_status_tone};

pub fn view(app: &App, cx: &mut Cx<App>, k: &Kit, main: El) -> El {
    let sidebar = sidebar(app, k);

    let toolbar = Toolbar::new()
        .leading(
            rune::col()
                .gap(1.0)
                .child(
                    text(page_title(app))
                        .font(TextStyle::new(14.0, 600.0).lh(17.0))
                        .color(k.c.text_base),
                )
                .child(
                    text(page_subtitle(app))
                        .font(k.ty.small)
                        .color(k.c.text_muted),
                ),
        )
        .trailing(toolbar_actions(app, k));

    SplitView::new(main)
        .sidebar(&app.sidebar, |app: &mut App| &mut app.sidebar, sidebar)
        .toolbar(toolbar)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .sidebar_padding(16.0, 8.0)
        .sidebar_toggle(
            SidebarToggle::new()
                .size(ControlSize::Large)
                .past_lights(16.0)
                .edge(18.0)
                .gap(4.0)
                .shortcut("")
                .in_header(),
        )
        .pane_motion(
            PaneMotion::new()
                .spring(Spring::new(0.3, 0.0))
                .scale_with_width(420.0, 0.15, 0.3)
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

fn toolbar_actions(app: &App, k: &Kit) -> El {
    let mut actions = row().gap(6.0).center_y();
    if app.screen == Screen::Dashboard {
        actions = actions.child(
            Button::new("Refresh")
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Small)
                .on_press_tx(|app: &mut App, tx: &mut Tx| app.refresh_all(tx))
                .build(k),
        );
        actions = actions.child(
            Button::new("Add bulb")
                .variant(ButtonVariant::Contrast)
                .control_size(ControlSize::Small)
                .on_press(|app: &mut App| app.open_add())
                .build(k),
        );
    }
    actions
}

fn sidebar(app: &App, k: &Kit) -> El {
    let brand = rune::col()
        .w_full()
        .gap(1.0)
        .child(
            text("jyoti".to_owned())
                .font(TextStyle::new(17.0, 650.0).lh(20.0))
                .color(k.c.text_base),
        )
        .child(
            text("Local bulb control".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        );

    let add = IconButton::new(&PLUS)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Mini)
        .help("Add bulb")
        .on_press(|app: &mut App| app.open_add())
        .build(k)
        .key("sidebar-add");

    let bulb_screen = matches!(
        app.screen,
        Screen::Dashboard | Screen::Form(_) | Screen::Remove(_) | Screen::Color
    );
    let overview = SidebarRow::new("Overview")
        .icon(&BULB)
        .height(32.0)
        .selected(app.screen == Screen::Dashboard)
        .on_select(|app: &mut App, _| app.open_overview())
        .build(k)
        .key("sidebar-overview");
    let sync = SidebarRow::new("Screen sync")
        .icon(&SYNC)
        .height(32.0)
        .trailing(dot(sync_status_tone(k, &app.sync.status)))
        .selected(app.screen == Screen::Sync)
        .on_select(|app: &mut App, _| app.open_sync())
        .build(k)
        .key("sidebar-sync");

    let settings = SidebarRow::new("Settings")
        .icon(&GEAR)
        .height(30.0)
        .selected(app.screen == Screen::Settings)
        .on_select(|app: &mut App, _| app.open_settings())
        .build(k)
        .key("sidebar-settings");
    let help = SidebarRow::new("Help")
        .icon(&HELP)
        .height(30.0)
        .selected(app.screen == Screen::Help)
        .on_select(|app: &mut App, _| app.open_help())
        .build(k)
        .key("sidebar-help");

    let workspace_header = SidebarHeader::new("Workspace").build(k);
    let bulbs_header = SidebarHeader::new("Bulbs")
        .count(app.bulbs.len())
        .action(add)
        .build(k);
    let show_all = app.bulbs.len() >= 2;
    let bulb_count = app.bulbs.len() + usize::from(show_all);
    let kit = k.clone();
    let build_bulb = move |app: &App, index: usize| -> El {
        if show_all && index == 0 {
            let aggregate = app.aggregate();
            let tone = match aggregate.power {
                AggregatePower::On => kit.c.success.icon,
                AggregatePower::Off | AggregatePower::Mixed => kit.c.warning.icon,
                AggregatePower::Unknown => kit.c.text_faint,
            };
            return SidebarRow::new("All bulbs")
                .icon(&BULB)
                .height(42.0)
                .detail(format!("{}/{}", aggregate.reachable, aggregate.total))
                .trailing(dot(tone))
                .selected(bulb_screen && app.selection == Some(Selection::All))
                .on_select(|app: &mut App, _| app.select_all())
                .build(&kit)
                .key("sidebar-all");
        }
        let Some(bulb) = app.bulbs.get(index.saturating_sub(usize::from(show_all))) else {
            return rune::el().size(0.0, 0.0);
        };
        let state = app.states.get(&bulb.id);
        let unreachable = app.sync.unreachable.contains(&bulb.id);
        let tone = if app.errors.contains_key(&bulb.id) || unreachable {
            kit.c.danger.icon
        } else {
            match state.and_then(|state| state.power) {
                Some(true) => kit.c.success.icon,
                Some(false) => kit.c.text_faint,
                None => kit.c.warning.icon,
            }
        };
        let subtitle = match state.and_then(|state| state.power) {
            Some(true) => format!("{} · On", bulb.address),
            Some(false) => format!("{} · Off", bulb.address),
            None => format!("{} · Connecting", bulb.address),
        };
        let id = bulb.id.clone();
        SidebarRow::new(bulb.name.clone())
            .subtitle(subtitle)
            .height(42.0)
            .trailing(dot(tone))
            .selected(bulb_screen && app.selection == Some(Selection::Bulb(id.clone())))
            .on_select(move |app: &mut App, _| app.select_bulb(&id))
            .build(&kit)
            .key(bulb.id.clone())
    };

    Sidebar::new()
        .label("Sidebar")
        .inset(0.0)
        .header_gap(2.0)
        .header_padding(9.0, 8.0)
        .row_gap(1.0)
        .row_height(42.0)
        .header(brand)
        .header(workspace_header)
        .header(overview)
        .header(sync)
        .header(bulbs_header)
        .virtual_rows(
            bulb_count,
            move |app: &App, index| {
                if show_all && index == 0 {
                    "sidebar-all".into()
                } else {
                    app.bulbs
                        .get(index.saturating_sub(usize::from(show_all)))
                        .map_or("", |bulb| bulb.id.as_str())
                        .into()
                }
            },
            build_bulb,
        )
        .footer(settings)
        .footer(help)
        .build(k)
}

fn page_title(app: &App) -> &'static str {
    match &app.screen {
        Screen::Dashboard => "Overview",
        Screen::Sync => "Screen sync",
        Screen::Settings => "Settings",
        Screen::Help => "Help",
        Screen::Form(FormMode::Add) => "Add bulb",
        Screen::Form(FormMode::Edit(_)) => "Edit bulb",
        Screen::Remove(_) => "Remove bulb",
        Screen::Color => "Colour",
    }
}

fn page_subtitle(app: &App) -> &'static str {
    match &app.screen {
        Screen::Dashboard => "Power, light and connection at a glance",
        Screen::Sync => "One monitor drives every bulb",
        Screen::Settings => "Local configuration and storage",
        Screen::Help => "How the app and screen sync work",
        Screen::Form(_) => "Name, address and token",
        Screen::Remove(_) => "This action cannot be undone",
        Screen::Color => "Preview the colour before keeping it",
    }
}
