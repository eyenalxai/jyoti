use rune::TextEdit;

use crate::app::App;
use crate::app::state::ToastKind;
use crate::app::sync::restore::{RestoreControl, restore_control};
use crate::app::view::{ColorEditor, ColorSession};
use crate::bulb::model::{ColorMode, Rgb};
use crate::bulb::worker::Command;
use crate::screen::color::{Hsb, hsb_to_rgb, rgb_to_hsb};

impl App {
    pub(crate) fn color(&self) -> Option<&ColorEditor> {
        self.view.color()
    }

    pub(crate) fn color_is_open(&self) -> bool {
        self.color().is_some_and(ColorEditor::is_open)
    }

    pub(crate) fn color_session(&self) -> Option<&ColorSession> {
        self.color().and_then(ColorEditor::session)
    }

    fn color_mut(&mut self) -> Option<&mut ColorEditor> {
        self.view.as_control_mut().map(|control| &mut control.color)
    }

    // The hex input binds to a `&mut TextEdit` even while the editor is closed.
    pub(crate) fn color_hex_mut(&mut self) -> &mut TextEdit {
        if let Some(control) = self.view.as_control_mut() {
            return control.color.hex_mut();
        }
        &mut self.empty_edit
    }

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
        let pre_open = targets
            .iter()
            .map(|id| (id.clone(), self.states.get(id).cloned()))
            .collect();
        if let Some(control) = self.view.as_control_mut() {
            control.color = ColorEditor::open(targets, first, pre_open);
        }
    }

    pub(crate) fn update_color_from_hsb(&mut self) {
        let color = {
            let Some(session) = self.color_mut().and_then(ColorEditor::session_mut) else {
                return;
            };
            let hsb = Hsb {
                hue: session.hue,
                saturation: session.saturation,
                value: session.value,
            };
            let color = hsb_to_rgb(hsb);
            session.preview = color;
            session.hex = TextEdit::new(color.hex());
            session.error = None;
            color
        };
        self.apply_color_targets(color);
    }

    pub(crate) fn update_color_from_hex(&mut self) {
        let Some(session) = self.color_mut().and_then(ColorEditor::session_mut) else {
            return;
        };
        let value = session.hex.text.clone();
        let Some(color) = Rgb::parse_hex(&value) else {
            session.error = Some("Use a six-digit hex colour, for example #FF8A00.".to_owned());
            return;
        };
        let hsb = rgb_to_hsb(color);
        session.hue = hsb.hue;
        session.saturation = hsb.saturation;
        session.value = hsb.value;
        session.preview = color;
        session.error = None;
        self.apply_color_targets(color);
    }

    pub(crate) fn set_color_hue(&mut self, hue: f32) {
        if let Some(session) = self.color_mut().and_then(ColorEditor::session_mut) {
            session.hue = hue;
        }
        self.update_color_from_hsb();
    }

    pub(crate) fn set_color_saturation(&mut self, saturation: f32) {
        if let Some(session) = self.color_mut().and_then(ColorEditor::session_mut) {
            session.saturation = saturation;
        }
        self.update_color_from_hsb();
    }

    pub(crate) fn set_color_value(&mut self, value: f32) {
        if let Some(session) = self.color_mut().and_then(ColorEditor::session_mut) {
            session.value = value;
        }
        self.update_color_from_hsb();
    }

    fn apply_color_targets(&mut self, color: Rgb) {
        let Some(session) = self.color_session() else {
            return;
        };
        let targets = session.targets.clone();
        for id in targets {
            if let Some(state) = self.states.get_mut(&id) {
                state.mode = ColorMode::Color;
                state.color = Some(color);
            }
            self.edits.touch(&id);
            self.dashboard.pending_color.insert(id, color);
        }
        self.mark_pending();
    }

    pub(crate) fn cancel_color(&mut self) {
        let Some(session) = self.color_mut().and_then(ColorEditor::take) else {
            return;
        };
        for id in &session.targets {
            self.dashboard.pending_color.remove(id);
            self.dashboard.pending_temperature.remove(id);
            self.dashboard.pending_brightness.remove(id);
        }
        for (id, previous) in session.into_pre_open() {
            let Some(previous) = previous else {
                continue;
            };
            self.states.insert(id.clone(), previous.clone());
            self.edits.touch(&id);
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
    }

    pub(crate) fn commit_color(&mut self) {
        self.update_color_from_hex();
        let Some(editor) = self.color_mut() else {
            return;
        };
        if editor
            .session()
            .is_some_and(|session| session.error.is_some())
        {
            return;
        }
        let _ = editor.commit();
    }
}
