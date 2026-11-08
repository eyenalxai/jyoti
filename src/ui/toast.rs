use rune::El;
use rune_kit::{Kit, Toast, ToastRegion};

use crate::app::App;
use crate::app::state::ToastKind;
use crate::ui::widget::dot;

pub fn layer(app: &App, k: &Kit) -> El {
    let mut region = ToastRegion::new();
    for toast in &app.toasts {
        let tone = match toast.kind {
            ToastKind::Info => k.c.info.icon,
            ToastKind::Success => k.c.success.icon,
            ToastKind::Error => k.c.danger.icon,
        };
        let title = match toast.kind {
            ToastKind::Info => "Notice",
            ToastKind::Success => "Done",
            ToastKind::Error => "Error",
        };
        let id = toast.id;
        region = region.toast(
            Toast::new(format!("toast-{id}"), title)
                .description(toast.message.clone())
                .lead(dot(tone))
                .on_close(move |app: &mut App| app.dismiss_toast(id))
                .build(k),
        );
    }
    region.build(k)
}
