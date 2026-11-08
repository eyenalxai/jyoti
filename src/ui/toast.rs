use rune::El;
use rune_kit::{Kit, Toast, ToastRegion};

use crate::app::App;
use crate::app::state::ToastKind;
use crate::ui::widget::{ALERT, CHECK, dot};

pub fn layer(app: &App, k: &Kit) -> El {
    let mut region = ToastRegion::new();
    for toast in &app.toasts {
        let (title, lead) = match toast.kind {
            ToastKind::Info => ("Notice", dot(k.c.info.icon)),
            ToastKind::Success => (
                "Done",
                rune::icon(&CHECK, 15.0)
                    .color(k.c.success.icon)
                    .a11y_hidden(),
            ),
            ToastKind::Error => (
                "Error",
                rune::icon(&ALERT, 15.0)
                    .color(k.c.danger.icon)
                    .a11y_hidden(),
            ),
        };
        let id = toast.id;
        region = region.toast(
            Toast::new(format!("toast-{id}"), title)
                .description(toast.message.clone())
                .lead(lead)
                .on_close(move |app: &mut App| app.dismiss_toast(id))
                .build(k),
        );
    }
    region.build(k)
}
