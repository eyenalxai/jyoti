mod app;
mod bulb;
mod config;
mod error;
mod miio;
mod screen;
mod ui;

use std::time::Instant;

use rune::{AppSpec, Tx, Waker};

use crate::app::App;
use crate::app::state::ToastKind;
use crate::app::sync::run::RunPhase;
use crate::app::view::{Overlay, View};
use crate::bulb::model::Rgb;
use crate::screen::monitor::{MonitorInfo, Scene};

fn setup(app: &mut App, waker: Waker) {
    app.start(waker);
}

fn poll(app: &mut App, dt: f64, tx: &mut Tx) -> bool {
    app.poll(dt, tx)
}

fn on_quit(app: &mut App, tx: &mut Tx) {
    app.quit(tx);
}

type SceneFn = fn(&mut App);

/// Inter carries the interface, Fraunces the display voice, and IBM Plex Mono
/// the addresses and hex values. All three ship with their OFL licence.
const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/InterVariable.ttf"),
    include_bytes!("../assets/fonts/Fraunces.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
];

const SCENES: &[(&str, SceneFn)] = &[
    ("dashboard", |app| app.show_now(View::control())),
    ("empty", |app| {
        app.bulbs.clear();
        app.states.clear();
        app.health.clear();
        app.edits.clear();
        app.selection = None;
        app.toasts.clear();
        app.show_now(View::control());
    }),
    ("form", |app| {
        app.open_add();
        app.show_pending();
    }),
    ("edit", |app| {
        if let Some(id) = app.bulbs.first().map(|bulb| bulb.id.clone()) {
            app.open_edit(&id);
            app.show_pending();
        }
    }),
    ("color", |app| {
        app.show_now(View::control());
        for state in app.states.values_mut() {
            state.mode = crate::bulb::model::ColorMode::Color;
        }
        app.open_color();
    }),
    ("color-control", |app| {
        app.show_now(View::control());
        for state in app.states.values_mut() {
            state.mode = crate::bulb::model::ColorMode::Color;
            state.color = Some(Rgb::new(255, 96, 96));
        }
    }),
    ("remove", |app| {
        if let Some(id) = app.bulbs.first().map(|bulb| bulb.id.clone()) {
            app.open_remove(&id);
            app.show_pending();
        }
    }),
    ("sync-live", |app| app.show_now(View::Sync)),
    ("sync", |app| {
        app.show_now(View::Sync);
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
        app.sync.status = RunPhase::Running(1);
        app.sync.gain = 1.2;
        app.sync.colour_intensity = 35.0;
        app.sync.derived = Some(crate::screen::intensity::intensify(
            Rgb::new(255, 140, 64),
            35.0,
        ));
        app.sync.brightness = 57;
        app.sync.last_scene = Some(Scene {
            color: Rgb::new(255, 140, 64),
            brightness: 0.52,
        });
        app.sync.last_scene_at = Some(Instant::now());
    }),
    ("picker", |app| {
        app.show_now(View::control());
        app.toggle_overlay(Overlay::BulbPicker);
    }),
    ("menu", |app| {
        app.show_now(View::control());
        app.toggle_overlay(Overlay::BulbMenu);
    }),
    ("toast", |app| {
        app.show_now(View::control());
        app.notify("The bulb at 192.168.1.42 did not answer.", ToastKind::Error);
    }),
    ("settings", |app| app.show_now(View::Settings)),
];

fn main() {
    rune::set_app_id("dev.jyoti.Jyoti");
    rune::App::new(AppSpec {
        min_size: Some((480.0, 600.0)),
        setup: Some(setup),
        poll: Some(poll),
        scenes: SCENES,
        fonts: FONTS,
        families: ("Inter, Inter Variable", "IBM Plex Mono"),
        ..AppSpec::new("jyoti", (500.0, 680.0), App::new, ui::view)
    })
    .on_quit(on_quit)
    .run();
}
