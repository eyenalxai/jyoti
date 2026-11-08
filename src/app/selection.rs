use crate::app::App;
use crate::app::state::Screen;
use crate::bulb::selection::Selection;

impl App {
    pub(crate) fn close_overlays(&mut self) {
        self.bulb_picker_open = false;
        self.bulb_menu_open = false;
        self.monitor_picker_open = false;
    }

    pub(crate) fn leave_color(&mut self) {
        self.close_overlays();
        if self.screen == Screen::Color {
            self.cancel_color();
        }
    }

    pub(crate) fn open_overview(&mut self) {
        self.leave_color();
        self.screen = Screen::Dashboard;
    }

    pub(crate) fn select_all(&mut self) {
        self.leave_color();
        self.selection = Some(Selection::All);
        self.screen = Screen::Dashboard;
    }

    pub(crate) fn select_bulb(&mut self, id: &str) {
        self.leave_color();
        self.selection = Some(Selection::Bulb(id.to_owned()));
        self.screen = Screen::Dashboard;
    }
}
