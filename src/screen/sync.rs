use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::time::Instant;

use crate::bulb::model::{BulbState, ColorMode, Rgb};
use crate::bulb::worker::Command;
use crate::screen::brightness::sync_brightness_for;
use crate::screen::color::Lab;
use crate::screen::flash::FlashGuard;
use crate::screen::intensity::intensify;
use crate::screen::monitor::Scene;
use crate::screen::reducer::{DerivedColor, lab_for_color, should_publish};

#[derive(Clone, Debug)]
pub struct SyncTarget {
    pub id: String,
    pub sender: Sender<Command>,
    pub on: bool,
}

#[derive(Clone, Debug)]
pub struct SyncConfig {
    pub gain: f32,
    pub intensity: f32,
    pub targets: Vec<SyncTarget>,
}

#[derive(Clone, Debug)]
pub struct SyncFrame {
    pub scene: Scene,
    pub output: Rgb,
    pub brightness: u8,
    pub flash_until: Option<Instant>,
    pub drove_color: Vec<String>,
    pub drove_brightness: Vec<String>,
}

pub struct SyncRun {
    config: SyncConfig,
    flash: FlashGuard,
    last_publish: Option<Lab>,
    brightness: u8,
    drove_color: HashSet<String>,
    drove_brightness: HashSet<String>,
    republish: bool,
}

impl SyncRun {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            flash: FlashGuard::default(),
            last_publish: None,
            brightness: 0,
            drove_color: HashSet::new(),
            drove_brightness: HashSet::new(),
            republish: false,
        }
    }

    pub fn configure(&mut self, config: SyncConfig) {
        let targets_changed = config.targets.len() != self.config.targets.len()
            || config
                .targets
                .iter()
                .zip(self.config.targets.iter())
                .any(|(next, current)| next.id != current.id || next.on != current.on);
        if targets_changed {
            self.republish = true;
        }
        if (config.intensity - self.config.intensity).abs() > f32::EPSILON {
            self.last_publish = None;
        }
        self.config = config;
    }

    pub fn drove_color(&self) -> Vec<String> {
        self.drove_color.iter().cloned().collect()
    }

    pub fn drove_brightness(&self) -> Vec<String> {
        self.drove_brightness.iter().cloned().collect()
    }

    pub fn apply(&mut self, scene: Scene, now: Instant) -> SyncFrame {
        let output = intensify(scene.color, self.config.intensity);
        let brightness_target = sync_brightness_for(scene.brightness, self.config.gain);
        let flashing = self.flash.guard(brightness_target, now);
        let flash_until = if flashing {
            self.flash.held_until()
        } else {
            None
        };
        let lab = lab_for_color(output);
        let republish = std::mem::take(&mut self.republish);
        let publish_color = republish
            || should_publish(
                DerivedColor {
                    rgb: output,
                    lab,
                    scene_brightness: scene.brightness,
                },
                self.last_publish,
            );
        let publish_brightness = !flashing && (republish || brightness_target != self.brightness);
        let mut drove_color = Vec::new();
        let mut drove_brightness = Vec::new();

        if publish_color {
            self.last_publish = Some(lab);
            for target in &self.config.targets {
                if target.sender.send(Command::SetColor(output)).is_ok()
                    && self.drove_color.insert(target.id.clone())
                {
                    drove_color.push(target.id.clone());
                }
            }
        }

        if publish_brightness {
            for target in &self.config.targets {
                if target.on
                    && target
                        .sender
                        .send(Command::SetBrightness(brightness_target))
                        .is_ok()
                    && self.drove_brightness.insert(target.id.clone())
                {
                    drove_brightness.push(target.id.clone());
                }
            }
        }

        if !flashing {
            self.brightness = brightness_target;
        }

        SyncFrame {
            scene,
            output,
            brightness: self.brightness,
            flash_until,
            drove_color,
            drove_brightness,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreControl {
    Color(crate::bulb::model::Rgb),
    Temperature(u16),
}

pub fn restore_control(state: &BulbState) -> Option<RestoreControl> {
    if state.mode == ColorMode::Temperature
        && let Some(temperature) = state.temperature
    {
        return Some(RestoreControl::Temperature(temperature));
    }
    if let Some(color) = state.color {
        return Some(RestoreControl::Color(color));
    }
    state.temperature.map(RestoreControl::Temperature)
}

#[derive(Clone, Debug, Default)]
pub struct RestoreReport {
    pub restored: usize,
    pub not_restored: Vec<String>,
    pub total: usize,
}

impl RestoreReport {
    pub fn message(&self) -> String {
        if self.total == 0 {
            return "Nothing to restore.".to_owned();
        }
        if self.not_restored.is_empty() {
            return format!("Restored {} of {} bulbs.", self.restored, self.total);
        }
        format!(
            "Restored {} of {} bulbs. Not restored: {}.",
            self.restored,
            self.total,
            self.not_restored.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn run(on: bool) -> (SyncRun, std::sync::mpsc::Receiver<Command>) {
        let (sender, receiver) = channel();
        let config = SyncConfig {
            gain: 1.0,
            intensity: 0.0,
            targets: vec![SyncTarget {
                id: "bulb".to_owned(),
                sender,
                on,
            }],
        };
        (SyncRun::new(config), receiver)
    }

    fn scene(color: Rgb, brightness: f32) -> Scene {
        Scene { color, brightness }
    }

    #[test]
    fn a_run_publishes_colour_and_brightness_to_an_on_target() {
        let (mut run, receiver) = run(true);
        let published = run.apply(scene(Rgb::new(255, 0, 0), 1.0), Instant::now());

        assert_eq!(published.brightness, 100);
        assert_eq!(published.drove_color, vec!["bulb".to_owned()]);
        assert_eq!(published.drove_brightness, vec!["bulb".to_owned()]);
        assert_eq!(run.drove_color(), vec!["bulb".to_owned()]);
        let commands: Vec<Command> = receiver.try_iter().collect();
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::SetColor(color) if color.red > 200))
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::SetBrightness(100)))
        );
    }

    #[test]
    fn an_unchanged_scene_does_not_touch_the_bulb() {
        let (mut run, receiver) = run(true);
        let now = Instant::now();
        run.apply(scene(Rgb::new(255, 0, 0), 1.0), now);
        receiver.try_iter().for_each(drop);

        let published = run.apply(
            scene(Rgb::new(255, 0, 0), 1.0),
            now + Duration::from_secs(1),
        );

        assert!(receiver.try_iter().next().is_none());
        assert!(published.flash_until.is_none());
        assert!(published.drove_color.is_empty());
    }

    #[test]
    fn an_off_target_gets_colour_but_no_brightness() {
        let (mut run, receiver) = run(false);
        let published = run.apply(scene(Rgb::new(255, 0, 0), 1.0), Instant::now());

        assert_eq!(published.drove_color, vec!["bulb".to_owned()]);
        assert!(published.drove_brightness.is_empty());
        let commands: Vec<Command> = receiver.try_iter().collect();
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::SetColor(_)))
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::SetBrightness(_)))
        );
    }

    #[test]
    fn a_target_turning_on_republishes_the_output() {
        let (sender, receiver) = channel();
        let mut run = SyncRun::new(SyncConfig {
            gain: 1.0,
            intensity: 0.0,
            targets: vec![SyncTarget {
                id: "bulb".to_owned(),
                sender: sender.clone(),
                on: false,
            }],
        });
        let now = Instant::now();
        run.apply(scene(Rgb::new(255, 0, 0), 1.0), now);
        receiver.try_iter().for_each(drop);

        run.configure(SyncConfig {
            gain: 1.0,
            intensity: 0.0,
            targets: vec![SyncTarget {
                id: "bulb".to_owned(),
                sender,
                on: true,
            }],
        });
        let published = run.apply(
            scene(Rgb::new(255, 0, 0), 1.0),
            now + Duration::from_millis(700),
        );

        assert!(published.drove_color.is_empty());
        assert_eq!(published.drove_brightness, vec!["bulb".to_owned()]);
        let commands: Vec<Command> = receiver.try_iter().collect();
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::SetColor(_)))
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::SetBrightness(100)))
        );
    }

    #[test]
    fn the_flash_guard_holds_brightness_but_keeps_the_colour() {
        let (mut run, receiver) = run(true);
        let now = Instant::now();
        run.apply(scene(Rgb::new(255, 0, 0), 0.0), now);
        receiver.try_iter().for_each(drop);
        run.apply(
            scene(Rgb::new(0, 255, 0), 1.0),
            now + Duration::from_millis(50),
        );
        receiver.try_iter().for_each(drop);

        let published = run.apply(
            scene(Rgb::new(255, 0, 0), 0.0),
            now + Duration::from_millis(100),
        );

        assert!(published.flash_until.is_some());
        assert_eq!(published.brightness, 100);
        let commands: Vec<Command> = receiver.try_iter().collect();
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::SetColor(_)))
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::SetBrightness(_)))
        );
    }

    #[test]
    fn restore_report_names_the_bulbs_that_did_not_return() {
        let report = RestoreReport {
            restored: 2,
            not_restored: vec!["Shelf lamp".to_owned()],
            total: 3,
        };
        assert_eq!(
            report.message(),
            "Restored 2 of 3 bulbs. Not restored: Shelf lamp."
        );
    }

    #[test]
    fn a_clean_restore_counts_every_bulb() {
        let report = RestoreReport {
            restored: 3,
            not_restored: Vec::new(),
            total: 3,
        };
        assert_eq!(report.message(), "Restored 3 of 3 bulbs.");
    }
}
