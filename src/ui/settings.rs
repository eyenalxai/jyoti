use rune::{El, ScrollSpec, Tx, col, row, text};
use rune_kit::Kit;

use crate::app::App;
use crate::app::state::ToastKind;
use crate::bulb::store::Appearance;
use crate::ui::widget::{
    COPY, LOCK, PAGE_PAD, fact_row, group, hairline, segmented_switch, setting_row,
};

pub fn view(app: &App, k: &Kit) -> El {
    let appearance = group(
        k,
        "Appearance",
        col()
            .w_full()
            .gap(8.0)
            .padding(10.0)
            .child(segmented_switch(
                k,
                "appearance",
                "Theme",
                &["Light", "Dark", "System"],
                Some(app.appearance.index()),
                |app: &mut App, index, tx| {
                    app.set_appearance(Appearance::from_index(index), tx);
                },
            ))
            .child(
                text("System follows the desktop.".to_owned())
                    .font(k.ty.small)
                    .color(k.c.text_muted),
            ),
    );

    let metadata = app.config.config_path.display().to_string();
    let preferences = app.config.preferences_path.display().to_string();
    let storage = group(
        k,
        "Storage",
        col()
            .w_full()
            .child(path_row(
                k,
                "Bulb metadata",
                "Name, address and model.",
                &metadata,
            ))
            .child(hairline(k))
            .child(path_row(
                k,
                "Preferences",
                "Sync gain, colour intensity and appearance.",
                &preferences,
            ))
            .child(hairline(k))
            .child(setting_row(
                k,
                "Tokens",
                "One OS keyring entry per bulb.",
                row()
                    .gap(6.0)
                    .center_y()
                    .child(rune::icon(&LOCK, 14.0).color(k.c.icon_muted).a11y_hidden())
                    .child(
                        text("OS keyring".to_owned())
                            .font(k.ty.small_medium)
                            .color(k.c.text_muted),
                    ),
            )),
    );

    let about = group(
        k,
        "About",
        col()
            .w_full()
            .padding(12.0)
            .child(fact_row(k, "Version", env!("CARGO_PKG_VERSION")))
            .child(fact_row(k, "Protocol", "miIO over UDP"))
            .child(fact_row(k, "Capture", "libwayshot · EGL/DMA-BUF")),
    );

    let viewport = col()
        .key("settings")
        .w_full()
        .padding(PAGE_PAD)
        .gap(11.0)
        .child(appearance)
        .child(storage)
        .child(about)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn path_row(k: &Kit, title: &str, description: &str, path: &str) -> El {
    let full_path = path.to_owned();
    col()
        .w_full()
        .gap(2.0)
        .px(14.0)
        .py(10.0)
        .key(format!("copy-{title}"))
        .radius(k.radius.sm)
        .role("button")
        .label(format!("Copy {title} path"))
        .a11y_value(path.to_owned())
        .focusable(true)
        .focus_ring(2.0, 1.0, k.c.border_focus)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press_tx(move |app: &mut App, tx: &mut Tx| {
            tx.copy(full_path.clone());
            app.notify("Path copied.", ToastKind::Success);
        })
        .child(
            text(title.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(
            text(description.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(
                    text(path.to_owned())
                        .font(k.ty.mono)
                        .color(k.c.text_faint)
                        .wrap_anywhere()
                        .grow(1.0)
                        .min_w(0.0)
                        .a11y_hidden(),
                )
                .child(rune::icon(&COPY, 14.0).color(k.c.icon_muted).a11y_hidden()),
        )
}
