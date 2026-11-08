use rune::{Cx, El, ScrollSpec, col, row, text};
use rune_kit::shell::Page;
use rune_kit::{IconButton, IconButtonVariant, Kit};

use crate::app::App;
use crate::ui::widget::{BACK, BULB, HELP, SYNC, section, shortcut_row, shortcut_text};

pub fn view(_app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);

    let keys = section(
        k,
        "Keyboard",
        col()
            .gap(8.0)
            .child(shortcut_row(k, "Add bulb", shortcut_text("⌘N")))
            .child(shortcut_row(k, "Edit selected bulb", shortcut_text("⌘E")))
            .child(shortcut_row(k, "Refresh every bulb", shortcut_text("⌘R")))
            .child(shortcut_row(k, "Toggle power", "Space"))
            .child(shortcut_row(k, "Switch to White", "W"))
            .child(shortcut_row(k, "Switch to Colour", "C"))
            .child(shortcut_row(k, "Screen sync", shortcut_text("⌘S")))
            .child(shortcut_row(k, "Settings", shortcut_text("⌘,")))
            .child(shortcut_row(k, "Back or close", "Esc"))
            .child(shortcut_row(k, "Quit", shortcut_text("⌘Q"))),
    );

    let bulbs = section(
        k,
        "Bulbs",
        col()
            .gap(10.0)
            .child(
                row()
                    .gap(10.0)
                    .center_y()
                    .child(rune::icon(&BULB, 18.0).color(k.c.text_muted))
                    .child(text("Add, edit and remove bulbs".to_owned()).font(k.ty.ui_medium).color(k.c.text_base)),
            )
            .child(
                text("Each bulb has an IP address and a 32-character token. The token is stored in the OS keyring. Metadata is stored in the XDG config folder."
                    .to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .wrap(),
            ),
    );

    let sync = section(
        k,
        "Screen sync",
        col()
            .gap(10.0)
            .child(
                row()
                    .gap(10.0)
                    .center_y()
                    .child(rune::icon(&SYNC, 18.0).color(k.c.text_muted))
                    .child(text("One monitor drives every bulb".to_owned()).font(k.ty.ui_medium).color(k.c.text_base)),
            )
            .child(
                text("Screen sync captures one Wayland monitor, derives its dominant colour and brightness, and writes them to every configured bulb. Pause holds the last colour. Stop restores each bulb's pre-sync colour or temperature and the brightness it drove."
                    .to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .wrap(),
            ),
    );

    let about = section(
        k,
        "About",
        row()
            .gap(8.0)
            .center_y()
            .child(rune::icon(&HELP, 18.0).color(k.c.text_muted))
            .child(
                text("jyoti talks to Xiaomi bulbs directly over miIO on the local network. There is no daemon, no cloud and no account."
                    .to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .wrap(),
            ),
    );

    let body = col()
        .key("help")
        .w_full()
        .max_w(720.0)
        .padding(32.0)
        .gap(20.0)
        .child(keys)
        .child(bulbs)
        .child(sync)
        .child(about)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    let body = row().w_full().h_full().justify_center().child(body);

    Page::new(body)
        .title("Help")
        .leading(back)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .build(k)
}
