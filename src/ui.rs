mod color;
mod dashboard;
mod forms;
mod settings;
mod shell;
mod sync;
mod toast;
mod widget;

use rune::{Cx, El, TextStyle};
use rune_kit::Kit;

use crate::app::App;
use crate::app::state::Screen;

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let k = readable_kit(cx.system_dark());
    let mut layers = Vec::new();
    let page = match &app.screen {
        Screen::Dashboard => dashboard::content(app, &k, &mut layers),
        Screen::Sync => sync::view(app, &k, &mut layers),
        Screen::Settings => settings::view(app, &k),
        Screen::Form(_) => forms::form_view(app, &k),
        Screen::Remove(_) => forms::remove_view(app, &k),
        Screen::Color => color::view(app, &k),
    };
    let shell = shell::view(app, cx, &k, page);
    if !app.toasts.is_empty() {
        layers.push(toast::layer(app, &k));
    }
    rune_kit::root(shell, layers)
}

fn readable_kit(dark: bool) -> Kit {
    let mut kit = Kit::neutral(dark);
    kit.ty.ui = TextStyle::new(14.0, 440.0).lh(21.0).tracking(-0.04);
    kit.ty.ui_medium = TextStyle::new(14.0, 530.0).lh(21.0).tracking(-0.04);
    kit.ty.ui_compact = TextStyle::new(14.0, 440.0).lh(20.0).tracking(-0.04);
    kit.ty.small = TextStyle::new(13.0, 440.0).lh(18.0).tracking(0.05);
    kit.ty.small_medium = TextStyle::new(13.0, 530.0).lh(18.0).tracking(0.05);
    kit.ty.body = TextStyle::new(15.0, 400.0).lh(22.0);
    kit.ty.body_medium = TextStyle::new(15.0, 500.0).lh(22.0);
    kit.ty.text_small = TextStyle::new(14.0, 400.0).lh(20.0);
    kit.ty.text_small_medium = TextStyle::new(14.0, 500.0).lh(20.0);
    kit.ty.text_xsmall = TextStyle::new(13.0, 400.0).lh(18.0);
    kit.ty.text_xsmall_medium = TextStyle::new(13.0, 500.0).lh(18.0);
    kit.ty.mono = TextStyle::new(13.0, 400.0).mono().lh(19.0);
    kit
}
