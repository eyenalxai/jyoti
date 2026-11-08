mod color;
mod dashboard;
mod forms;
mod help;
mod settings;
mod shell;
mod sync;
mod toast;
mod widget;

use rune::{Cx, El};
use rune_kit::Kit;

use crate::app::App;
use crate::app::state::Screen;

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let page = match &app.screen {
        Screen::Dashboard => shell::view(app, cx, &k),
        Screen::Sync => sync::view(app, cx, &k),
        Screen::Settings => settings::view(app, cx, &k),
        Screen::Help => help::view(app, cx, &k),
        Screen::Form(_) => forms::form_view(app, cx, &k),
        Screen::Remove(_) => forms::remove_view(app, cx, &k),
        Screen::Color => color::view(app, cx, &k),
    };
    let mut layers = Vec::new();
    if !app.toasts.is_empty() {
        layers.push(toast::layer(app, &k));
    }
    rune_kit::root(page, layers)
}
