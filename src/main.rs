mod app;
mod bulb;
mod config;
mod error;
mod miio;
mod screen;
mod ui;

use std::time::Instant;

use rune::{AppSpec, KeyEvent, Tx, Waker};

use crate::app::App;
use crate::app::state::{Screen, ToastKind};
use crate::bulb::model::Rgb;
use crate::screen::monitor::{MonitorInfo, Scene};
use crate::screen::sync::SyncStatus;

fn setup(app: &mut App, waker: Waker) {
    app.start(waker);
}

fn poll(app: &mut App, dt: f64, tx: &mut Tx) -> bool {
    app.poll(dt, tx)
}

fn on_key(app: &mut App, event: &KeyEvent, tx: &mut Tx) -> bool {
    app.on_key(event, tx)
}

fn on_quit(app: &mut App, tx: &mut Tx) {
    app.quit(tx);
}

type SceneFn = fn(&mut App);

const SCENES: &[(&str, SceneFn)] = &[
    ("dashboard", |app| app.screen = Screen::Dashboard),
    ("empty", |app| {
        app.bulbs.clear();
        app.states.clear();
        app.errors.clear();
        app.selection = None;
        app.toasts.clear();
        app.screen = Screen::Dashboard;
    }),
    ("form", |app| app.open_add()),
    ("edit", |app| {
        if let Some(id) = app.bulbs.first().map(|bulb| bulb.id.clone()) {
            app.open_edit(&id);
        }
    }),
    ("color", |app| app.open_color()),
    ("remove", |app| {
        if let Some(id) = app.bulbs.first().map(|bulb| bulb.id.clone()) {
            app.open_remove(&id);
        }
    }),
    ("sync-live", |app| app.screen = Screen::Sync),
    ("sync", |app| {
        app.screen = Screen::Sync;
        app.sync.monitors = vec![
            MonitorInfo {
                name: "eDP-1".to_owned(),
                description: "Built-in display".to_owned(),
                width: 2560,
                height: 1600,
            },
            MonitorInfo {
                name: "DP-1".to_owned(),
                description: "External display".to_owned(),
                width: 3440,
                height: 1440,
            },
        ];
        app.sync.selected_monitor = Some("eDP-1".to_owned());
        app.sync.status = SyncStatus::Running;
        app.sync.gain = 1.2;
        app.sync.derived = Some(Rgb::new(255, 140, 64));
        app.sync.brightness = 57;
        app.sync.last_scene = Some(Scene {
            color: Rgb::new(255, 140, 64),
            brightness: 0.52,
        });
        app.sync.last_scene_at = Some(Instant::now());
    }),
    ("toast", |app| {
        app.screen = Screen::Dashboard;
        app.notify("The bulb at 192.168.1.42 did not answer.", ToastKind::Error);
    }),
    ("settings", |app| app.screen = Screen::Settings),
    ("help", |app| app.screen = Screen::Help),
];

fn main() {
    rune::set_app_id("dev.jyoti.Jyoti");
    rune::App::new(AppSpec {
        min_size: Some((820.0, 600.0)),
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        scenes: SCENES,
        ..AppSpec::new("jyoti", (1120.0, 760.0), App::new, ui::view)
    })
    .on_quit(on_quit)
    .standard_menus()
    .run();
}
