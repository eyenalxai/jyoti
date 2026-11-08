use rune::{Cx, El, ScrollSpec, col, row, text};
use rune_kit::shell::Page;
use rune_kit::{IconButton, IconButtonVariant, Kit};

use crate::app::App;
use crate::ui::widget::{BACK, section, shortcut_row, shortcut_text};

pub fn view(_app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);

    let shortcuts = section(
        k,
        "Keyboard",
        col()
            .gap(8.0)
            .child(shortcut_row(k, "Add bulb", shortcut_text("⌘N")))
            .child(shortcut_row(k, "Refresh bulbs", shortcut_text("⌘R")))
            .child(shortcut_row(k, "Toggle power", "Space"))
            .child(shortcut_row(k, "White light", "W"))
            .child(shortcut_row(k, "Colour light", "C"))
            .child(shortcut_row(k, "Edit bulb", shortcut_text("⌘E")))
            .child(shortcut_row(k, "Screen sync", shortcut_text("⌘S")))
            .child(shortcut_row(k, "Settings", shortcut_text("⌘,")))
            .child(shortcut_row(k, "Back", "Esc")),
    );

    let about = section(
        k,
        "About",
        col()
            .gap(6.0)
            .child(
                text("jyoti controls Xiaomi light bulbs over the local network. Tokens live in the OS keyring; metadata lives in the XDG config folder."
                    .to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .wrap(),
            )
            .child(
                text("Screen sync captures one Wayland monitor, derives its colour and drives every configured bulb."
                    .to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .wrap(),
            ),
    );

    let body = col()
        .key("settings")
        .w_full()
        .max_w(720.0)
        .padding(32.0)
        .gap(20.0)
        .child(shortcuts)
        .child(about)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    let body = row().w_full().h_full().justify_center().child(body);

    Page::new(body)
        .title("Settings")
        .leading(back)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .build(k)
}
