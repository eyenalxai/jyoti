use rune::{Cx, El, Key, ScrollSpec, col, row, text};
use rune_kit::shell::Page;
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, TextInput};

use crate::app::App;
use crate::app::state::ColorTab;
use crate::ui::widget::{BACK, rgb_color, slider, swatch};

fn choose_tab(app: &mut App, index: usize) {
    app.color.tab = if index == 0 {
        ColorTab::Hs
    } else {
        ColorTab::Rgb
    };
}

pub fn view(app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .help("Cancel and restore the previous colour")
        .on_press(|app: &mut App| app.cancel_color())
        .build(k);
    let cancel = Button::new("Cancel")
        .variant(ButtonVariant::Ghost)
        .on_press(|app: &mut App| app.cancel_color())
        .build(k);
    let done = Button::new("Done")
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Regular)
        .on_press(|app: &mut App| app.commit_color())
        .build(k);

    let preview = app.color.preview;
    let hex = TextInput::new(&app.color.hex)
        .label("Hex colour")
        .placeholder("#FF8A00")
        .width(Some(220.0))
        .on_edit(|app: &mut App| &mut app.color.hex)
        .on_key(|app: &mut App, event, tx| {
            if event.key == Key::Enter {
                app.update_color_from_hex();
                tx.wake_after(std::time::Duration::from_millis(160));
                true
            } else {
                false
            }
        })
        .build(k);

    let selected_tab = if app.color.tab == ColorTab::Hs {
        Some(0)
    } else {
        Some(1)
    };
    let tab = crate::ui::widget::segmented(
        k,
        &["Hue / saturation", "RGB"],
        selected_tab,
        "Colour picker mode",
        360.0,
        choose_tab,
    );

    let controls = match app.color.tab {
        ColorTab::Hs => col()
            .gap(14.0)
            .child(
                text("Hue".to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .w(86.0),
            )
            .child(slider(
                k,
                "hue",
                app.color.hue / 360.0,
                "Hue",
                1.0 / 360.0,
                |app: &mut App, fraction| {
                    app.color.hue = fraction * 360.0;
                    app.update_color_from_hsb();
                },
            ))
            .child(
                text("Saturation".to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .w(86.0),
            )
            .child(slider(
                k,
                "saturation",
                app.color.saturation / 100.0,
                "Saturation",
                1.0 / 100.0,
                |app: &mut App, fraction| {
                    app.color.saturation = fraction * 100.0;
                    app.update_color_from_hsb();
                },
            )),
        ColorTab::Rgb => col()
            .gap(14.0)
            .child(
                text("Red".to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .w(86.0),
            )
            .child(slider(
                k,
                "red",
                f32::from(preview.red) / 255.0,
                "Red",
                1.0 / 255.0,
                |app: &mut App, fraction| {
                    let mut color = app.color.preview;
                    color.red = (fraction * 255.0).round() as u8;
                    app.update_color_from_rgb(color);
                },
            ))
            .child(
                text("Green".to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .w(86.0),
            )
            .child(slider(
                k,
                "green",
                f32::from(preview.green) / 255.0,
                "Green",
                1.0 / 255.0,
                |app: &mut App, fraction| {
                    let mut color = app.color.preview;
                    color.green = (fraction * 255.0).round() as u8;
                    app.update_color_from_rgb(color);
                },
            ))
            .child(
                text("Blue".to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .w(86.0),
            )
            .child(slider(
                k,
                "blue",
                f32::from(preview.blue) / 255.0,
                "Blue",
                1.0 / 255.0,
                |app: &mut App, fraction| {
                    let mut color = app.color.preview;
                    color.blue = (fraction * 255.0).round() as u8;
                    app.update_color_from_rgb(color);
                },
            )),
    };

    let mut body = col()
        .key("colour")
        .w_full()
        .max_w(620.0)
        .padding(32.0)
        .gap(20.0)
        .child(
            row()
                .gap(18.0)
                .center_y()
                .child(swatch(k, rgb_color(preview), 84.0))
                .child(
                    col()
                        .gap(4.0)
                        .child(text(preview.hex()).font(k.ty.mono).color(k.c.text_base))
                        .child(
                            text(format!(
                                "Applying to {} {}",
                                app.color.targets.len(),
                                if app.color.targets.len() == 1 {
                                    "bulb"
                                } else {
                                    "bulbs"
                                }
                            ))
                            .font(k.ty.small)
                            .color(k.c.text_muted),
                        ),
                ),
        )
        .child(tab)
        .child(controls)
        .child(hex);

    if let Some(error) = &app.color.error {
        body = body.child(
            text(error.clone())
                .font(k.ty.small)
                .color(k.c.danger.icon)
                .wrap(),
        );
    }

    body = body.child(
        text("Changes preview on the bulbs as you adjust. Done keeps the colour; Cancel restores each bulb's previous colour."
            .to_owned())
            .font(k.ty.small)
            .color(k.c.text_muted)
            .wrap(),
    );

    let body = body
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    let body = row().w_full().h_full().justify_center().child(body);
    Page::new(body)
        .title("Colour")
        .leading(back)
        .trailing(row().gap(8.0).child(cancel).child(done))
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .build(k)
}
