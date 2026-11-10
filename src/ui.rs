mod color;
mod dashboard;
mod forms;
mod settings;
mod shell;
mod sync;
mod theme;
mod toast;
mod widget;

use rune::{Cx, El};
use rune_kit::Kit;

use crate::app::App;
use crate::app::view::View;

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let k: &Kit = theme::kit(app.appearance.dark(cx.system_dark()));
    let mut layers = Vec::new();
    let page = match &app.view {
        View::Control(_) => dashboard::content(app, k, &mut layers),
        View::Sync => sync::view(app, k, &mut layers),
        View::Settings => settings::view(app, k),
        View::Form { .. } => forms::form_view(app, k),
        View::Remove(_) => forms::remove_view(app, k),
    };
    let shell = shell::view(app, cx, k, page);
    if !app.toasts.is_empty() {
        layers.push(toast::layer(app, k));
    }
    rune_kit::root(shell, layers)
}
