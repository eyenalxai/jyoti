use crate::app::App;
use crate::app::view::{Overlay, View};
use crate::bulb::selection::Selection;

impl App {
    pub(crate) fn overlay(&self) -> Overlay {
        self.view.overlay()
    }

    pub(crate) fn close_overlays(&mut self) {
        if let Some(control) = self.view.as_control_mut() {
            control.overlay = Overlay::None;
        }
    }

    pub(crate) fn toggle_overlay(&mut self, overlay: Overlay) {
        if let Some(control) = self.view.as_control_mut() {
            control.overlay = control.overlay.toggle(overlay);
        }
    }

    pub(crate) fn close_overlay(&mut self, overlay: Overlay) {
        if let Some(control) = self.view.as_control_mut() {
            control.overlay = control.overlay.close(overlay);
        }
    }

    /// Close the popovers and the colour editor before an action that may be
    /// refused, so a refused form does not leave them over Control.
    pub(crate) fn leave_control(&mut self) {
        self.cancel_color();
        self.view.close_transients();
    }

    pub(crate) fn open_overview(&mut self) {
        self.navigate(View::control());
    }

    pub(crate) fn select_all(&mut self) {
        self.selection = Some(Selection::All);
        self.navigate(View::control());
    }

    pub(crate) fn select_bulb(&mut self, id: &str) {
        self.selection = Some(Selection::Bulb(id.to_owned()));
        self.navigate(View::control());
    }
}
