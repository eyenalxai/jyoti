use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Accordion, AccordionItem, Kit};

use crate::app::App;
use crate::ui::widget::dot;

pub fn view(app: &App, k: &Kit) -> El {
    let open = app.help_open;
    let accordion = Accordion::new()
        .item(
            AccordionItem::new("Add a bulb")
                .expanded(open == Some(0))
                .on_toggle(move |app: &mut App| {
                    app.help_open = if open == Some(0) { None } else { Some(0) };
                })
                .child(content(
                    k,
                    "Use Add bulb on Control. A bulb needs a name, its LAN address and the 32-character token from the Xiaomi Home app. The token is stored in the OS keyring.",
                )),
        )
        .item(
            AccordionItem::new("Screen sync")
                .expanded(open == Some(1))
                .on_toggle(move |app: &mut App| {
                    app.help_open = if open == Some(1) { None } else { Some(1) };
                })
                .child(content(
                    k,
                    "Sync captures one monitor and writes its dominant colour and mean brightness to every bulb. Stop restores the colour or warmth and brightness each bulb had before the run.",
                )),
        )
        .item(
            AccordionItem::new("Troubleshooting")
                .expanded(open == Some(2))
                .on_toggle(move |app: &mut App| {
                    app.help_open = if open == Some(2) { None } else { Some(2) };
                })
                .child(
                    col()
                        .w_full()
                        .padding(12.0)
                        .gap(7.0)
                        .child(tip(k, "A bulb does not answer", "Check that the address is on this LAN and local control is enabled."))
                        .child(tip(k, "The token is rejected", "Edit the bulb and paste the token again. Leaving it blank keeps the stored one."))
                        .child(tip(k, "No monitors found", "Capture needs a Wayland compositor with wlr-screencopy, such as Hyprland or Sway.")),
                ),
        )
        .item(
            AccordionItem::new("Light controls")
                .expanded(open == Some(3))
                .on_toggle(move |app: &mut App| {
                    app.help_open = if open == Some(3) { None } else { Some(3) };
                })
                .child(content(
                    k,
                    "Brightness sets the light level, warmth sets the white side, and Colour intensity sets how saturated the current RGB hue is. Pick opens the full colour picker.",
                )),
        )
        .build(k);

    let scroll = col()
        .key("help")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(
            text("Everything runs on this machine. No daemon, account or cloud.".to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(accordion)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn content(k: &Kit, message: &str) -> El {
    col().w_full().padding(12.0).child(
        text(message.to_owned())
            .font(k.ty.small)
            .color(k.c.text_muted)
            .wrap(),
    )
}

fn tip(k: &Kit, title: &str, message: &str) -> El {
    row()
        .w_full()
        .gap(7.0)
        .items_start()
        .child(dot(k.c.text_faint).mt(5.0))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(1.0)
                .child(
                    text(title.to_owned())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(message.to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap(),
                ),
        )
}
