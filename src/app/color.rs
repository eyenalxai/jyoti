use rune::TextEdit;

use crate::app::App;
use crate::app::state::{ColorState, ToastKind};
use crate::bulb::model::{ColorMode, Rgb};
use crate::bulb::worker::Command;
use crate::screen::color::{Hsb, hsb_to_rgb, rgb_to_hsb};
use crate::screen::sync::{RestoreControl, restore_control};

impl App {
    pub(crate) fn open_color(&mut self) {
        self.close_overlays();
        if !self.sync.controls().manual {
            self.notify(
                "Screen sync owns colour and temperature while it runs.",
                ToastKind::Info,
            );
            return;
        }
        let targets = self.selected_ids();
        if targets.is_empty() {
            return;
        }
        self.flush_pending();
        let first = targets
            .iter()
            .find_map(|id| self.states.get(id).and_then(|state| state.color))
            .unwrap_or(Rgb::WHITE);
        let hsb = rgb_to_hsb(first);
        let revert = targets
            .iter()
            .filter_map(|id| {
                self.states
                    .get(id)
                    .cloned()
                    .map(|state| (id.clone(), state))
            })
            .collect();
        self.color = ColorState {
            open: true,
            targets,
            hue: hsb.hue,
            saturation: hsb.saturation,
            value: hsb.value,
            hex: TextEdit::new(first.hex()),
            preview: first,
            error: None,
            revert,
        };
    }

    pub(crate) fn update_color_from_hsb(&mut self) {
        let hsb = Hsb {
            hue: self.color.hue,
            saturation: self.color.saturation,
            value: self.color.value,
        };
        let color = hsb_to_rgb(hsb);
        self.color.preview = color;
        self.color.hex = TextEdit::new(color.hex());
        self.color.error = None;
        self.apply_color_targets(color);
    }

    pub(crate) fn update_color_from_hex(&mut self) {
        let value = self.color.hex.text.clone();
        let Some(color) = Rgb::parse_hex(&value) else {
            self.color.error = Some("Use a six-digit hex colour, for example #FF8A00.".to_owned());
            return;
        };
        let hsb = rgb_to_hsb(color);
        self.color.hue = hsb.hue;
        self.color.saturation = hsb.saturation;
        self.color.value = hsb.value;
        self.color.preview = color;
        self.color.error = None;
        self.apply_color_targets(color);
    }

    fn apply_color_targets(&mut self, color: Rgb) {
        let targets = self.color.targets.clone();
        for id in targets {
            if let Some(state) = self.states.get_mut(&id) {
                state.mode = ColorMode::Color;
                state.color = Some(color);
            }
            self.dashboard.pending_color.insert(id, color);
        }
        self.mark_pending();
    }

    pub(crate) fn cancel_color(&mut self) {
        let targets = self.color.targets.clone();
        let revert = self.color.revert.clone();
        for id in &targets {
            self.dashboard.pending_color.remove(id);
            self.dashboard.pending_temperature.remove(id);
            self.dashboard.pending_brightness.remove(id);
        }
        for id in targets {
            let Some(previous) = revert.get(&id).cloned() else {
                continue;
            };
            self.states.insert(id.clone(), previous.clone());
            match restore_control(&previous) {
                Some(RestoreControl::Color(color)) => {
                    self.send(&id, Command::SetColor(color));
                }
                Some(RestoreControl::Temperature(temperature)) => {
                    self.send(&id, Command::SetTemperature(temperature));
                }
                None => {}
            }
        }
        self.color.open = false;
    }

    pub(crate) fn commit_color(&mut self) {
        self.update_color_from_hex();
        if self.color.error.is_some() {
            return;
        }
        self.color.open = false;
    }
}
