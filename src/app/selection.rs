use crate::app::App;
use crate::app::state::Screen;
use crate::bulb::selection::Selection;

impl App {
    pub(crate) fn leave_color(&mut self) {
        if self.screen == Screen::Color {
            self.cancel_color();
        }
    }

    pub(crate) fn open_overview(&mut self) {
        self.leave_color();
        self.screen = Screen::Dashboard;
    }

    pub(crate) fn open_help(&mut self) {
        self.leave_color();
        self.screen = Screen::Help;
    }

    pub(crate) fn select_all(&mut self) {
        self.leave_color();
        self.selection = Some(Selection::All);
        self.table.clear_selection();
        self.screen = Screen::Dashboard;
    }

    pub(crate) fn select_bulb(&mut self, id: &str) {
        self.leave_color();
        self.selection = Some(Selection::Bulb(id.to_owned()));
        self.table.select(id.to_owned());
        self.screen = Screen::Dashboard;
    }

    pub(crate) fn sync_table_selection(&mut self) {
        self.table.clear_selection();
        if let Some(Selection::Bulb(id)) = &self.selection {
            self.table.select(id.clone());
        }
    }
}
