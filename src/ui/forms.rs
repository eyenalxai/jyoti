use std::time::Duration;

use rune::{El, ScrollSpec, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Switch, TextInput,
};

use crate::app::App;
use crate::app::state::{FormMode, Screen};
use crate::bulb::model::BulbField;
use crate::ui::widget::{ALERT, BACK, error_text, fact_row, hairline, subpage_header, surface};

fn submit(app: &mut App, tx: &mut Tx) {
    app.save_form();
    tx.wake_after(Duration::ZERO);
}

pub fn form_view(app: &App, k: &Kit) -> El {
    let editing = matches!(&app.screen, Screen::Form(FormMode::Edit(_)));
    let title = if editing { "Edit bulb" } else { "Add bulb" };
    let description = if editing {
        "Update metadata or replace the token."
    } else {
        "The bulb needs an address and token."
    };
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::ExtraLarge)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k)
        .size(40.0, 40.0)
        .radius(k.radius.md);
    let save = Button::new(if editing { "Save" } else { "Add bulb" })
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
        .on_press_tx(|app: &mut App, tx| {
            app.save_form();
            tx.wake_after(Duration::ZERO);
        })
        .build(k)
        .h(40.0);

    let name = TextInput::new(&app.form.name)
        .label("Name")
        .placeholder("Desk lamp")
        .width(None)
        .invalid(app.form.errors.contains_key(&BulbField::Name))
        .on_edit(|app: &mut App| &mut app.form.name)
        .on_submit(submit)
        .build(k);
    let address = TextInput::new(&app.form.address)
        .label("IP address")
        .placeholder("192.168.1.42")
        .width(None)
        .invalid(app.form.errors.contains_key(&BulbField::Address))
        .on_edit(|app: &mut App| &mut app.form.address)
        .on_submit(submit)
        .build(k);
    let model = TextInput::new(&app.form.model)
        .label("Model")
        .placeholder("Optional")
        .width(None)
        .invalid(app.form.errors.contains_key(&BulbField::Model))
        .on_edit(|app: &mut App| &mut app.form.model)
        .on_submit(submit)
        .build(k);
    let token = TextInput::new(&app.form.token)
        .label("Token")
        .placeholder(if editing {
            "Leave blank to keep"
        } else {
            "32 characters"
        })
        .width(None)
        .secure()
        .revealed(app.form.reveal)
        .invalid(app.form.errors.contains_key(&BulbField::Token))
        .on_edit(|app: &mut App| &mut app.form.token)
        .on_submit(submit)
        .build(k);
    let reveal = row().w_full().justify_end().child(
        Switch::new(app.form.reveal)
            .control_size(ControlSize::Regular)
            .label("Show token")
            .on_toggle(|app: &mut App| app.form.reveal = !app.form.reveal)
            .build(k)
            .padding(6.0),
    );

    let fields = col()
        .w_full()
        .gap(12.0)
        .child(field(
            k,
            "Name",
            "Shown in the bulb selector.",
            name,
            app.form.errors.get(&BulbField::Name),
        ))
        .child(field(
            k,
            "IP address",
            "The address on this LAN.",
            address,
            app.form.errors.get(&BulbField::Address),
        ))
        .child(field(
            k,
            "Model",
            "Optional label.",
            model,
            app.form.errors.get(&BulbField::Model),
        ))
        .child(
            col()
                .w_full()
                .gap(7.0)
                .child(field(
                    k,
                    "Token",
                    "Stored in the OS keyring.",
                    token,
                    app.form.errors.get(&BulbField::Token),
                ))
                .child(reveal),
        );

    let form = surface(
        k,
        col()
            .w_full()
            .padding(14.0)
            .child(fields)
            .child(row().w_full().py(12.0).child(hairline(k)))
            .child(row().w_full().justify_end().child(save)),
    );

    let viewport = col()
        .key("bulb-form")
        .w_full()
        .padding(14.0)
        .gap(12.0)
        .child(subpage_header(k, title, description, back))
        .child(form)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

pub fn remove_view(app: &App, k: &Kit) -> El {
    let id = match &app.screen {
        Screen::Remove(id) => Some(id),
        _ => None,
    };
    let bulb = id.and_then(|id| app.bulbs.iter().find(|bulb| &bulb.id == id));
    let name = bulb.map_or_else(|| "this bulb".to_owned(), |bulb| bulb.name.clone());
    let address = bulb.map_or_else(|| "Unknown address".to_owned(), |bulb| bulb.address.clone());
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::ExtraLarge)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k)
        .size(40.0, 40.0)
        .radius(k.radius.md);
    let remove = Button::new("Remove bulb")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Large)
        .on_press_tx(|app: &mut App, tx| {
            app.confirm_remove();
            tx.wake_after(Duration::ZERO);
        })
        .build(k)
        .h(40.0);

    let warning = row()
        .w_full()
        .padding(11.0)
        .gap(9.0)
        .radius(k.radius.md)
        .fill(k.c.danger.bg)
        .border(1.0, k.c.danger.border)
        .child(
            rune::icon(&ALERT, 16.0)
                .color(k.c.danger.icon)
                .a11y_hidden(),
        )
        .child(
            text(format!("{name} will be removed from jyoti."))
                .font(k.ty.ui_medium)
                .color(k.c.danger.fg)
                .wrap()
                .grow(1.0),
        );

    let form = surface(
        k,
        col()
            .w_full()
            .padding(14.0)
            .gap(10.0)
            .child(warning)
            .child(
                col()
                    .w_full()
                    .padding(4.0)
                    .child(fact_row(k, "Name", name))
                    .child(fact_row(k, "Address", address)),
            )
            .child(row().w_full().py(2.0).child(hairline(k)))
            .child(row().w_full().justify_end().child(remove)),
    );

    let viewport = col()
        .key("remove")
        .w_full()
        .padding(14.0)
        .gap(12.0)
        .child(subpage_header(
            k,
            "Remove bulb",
            "The metadata and keyring entry go away.",
            back,
        ))
        .child(form)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    let scroll = k.scroll_bar(viewport).grow(1.0).min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn field(k: &Kit, label: &str, description: &str, input: El, error: Option<&String>) -> El {
    let hint = match error {
        Some(error) => row()
            .w_full()
            .gap(6.0)
            .center_y()
            .child(
                rune::icon(&ALERT, 13.0)
                    .color(k.c.danger.icon)
                    .a11y_hidden(),
            )
            .child(error_text(k, error).grow(1.0)),
        None => text(description.to_owned())
            .font(k.ty.small)
            .color(k.c.text_muted),
    };
    col()
        .w_full()
        .gap(3.0)
        .child(
            text(label.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(input)
        .child(hint)
}
