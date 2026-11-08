use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Divider, Kit, Switch, TextInput};

use crate::app::App;
use crate::app::state::{FormMode, Screen};
use crate::bulb::model::BulbField;
use crate::ui::widget::{
    TRASH, badge, notice, panel, panel_body, panel_footer, panel_header, setting_row,
};

pub fn form_view(app: &App, k: &Kit) -> El {
    let editing = matches!(&app.screen, Screen::Form(FormMode::Edit(_)));
    let (title, save_label) = if editing {
        ("Bulb details", "Save changes")
    } else {
        ("New bulb", "Add bulb")
    };
    let save = Button::new(save_label)
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Regular)
        .on_press_tx(|app: &mut App, tx| {
            app.save_form();
            tx.wake_after(std::time::Duration::ZERO);
        })
        .build(k);
    let cancel = Button::new("Cancel")
        .variant(ButtonVariant::Ghost)
        .on_press(|app: &mut App| app.back())
        .build(k);

    let name = TextInput::new(&app.form.name)
        .label("Name")
        .placeholder("Desk lamp")
        .width(Some(300.0))
        .on_edit(|app: &mut App| &mut app.form.name)
        .build(k);
    let address = TextInput::new(&app.form.address)
        .label("IP address")
        .placeholder("192.168.1.42")
        .width(Some(300.0))
        .on_edit(|app: &mut App| &mut app.form.address)
        .build(k);
    let model = TextInput::new(&app.form.model)
        .label("Model")
        .placeholder("yeelink.light.color3")
        .width(Some(300.0))
        .on_edit(|app: &mut App| &mut app.form.model)
        .build(k);
    let token = TextInput::new(&app.form.token)
        .label("Token")
        .placeholder(if editing {
            "Leave blank to keep the stored token"
        } else {
            "32 hexadecimal characters"
        })
        .width(Some(300.0))
        .secure()
        .revealed(app.form.reveal)
        .on_edit(|app: &mut App| &mut app.form.token)
        .build(k);

    let reveal = Switch::new(app.form.reveal)
        .control_size(ControlSize::Small)
        .label("Show token")
        .on_toggle(|app: &mut App| app.form.reveal = !app.form.reveal)
        .build(k);

    let mut fields = col().w_full().gap(0.0);
    if let Some(message) = app.form.errors.values().next() {
        fields = fields.child(col().w_full().pb(10.0).child(notice(
            k,
            &crate::ui::widget::HELP,
            message,
            k.c.danger,
        )));
    }
    fields = fields
        .child(setting_row(
            k,
            "Name",
            "Shown in the sidebar.",
            control(k, name, app.form.errors.get(&BulbField::Name)),
        ))
        .child(Divider::horizontal().build(k))
        .child(setting_row(
            k,
            "IP address",
            "Address on this LAN.",
            control(k, address, app.form.errors.get(&BulbField::Address)),
        ))
        .child(Divider::horizontal().build(k))
        .child(setting_row(
            k,
            "Model",
            "Optional label.",
            control(k, model, app.form.errors.get(&BulbField::Model)),
        ))
        .child(Divider::horizontal().build(k))
        .child(setting_row(
            k,
            "Token",
            "Stored in the OS keyring.",
            col()
                .w(320.0)
                .gap(6.0)
                .child(control(k, token, app.form.errors.get(&BulbField::Token)))
                .child(reveal),
        ));

    let form = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                title,
                Some(if editing {
                    "Update the metadata, or leave the token blank to keep it."
                } else {
                    "Add the bulb before its first state arrives."
                }),
                vec![badge(
                    k,
                    if editing { "Editing" } else { "New" },
                    k.c.info.icon,
                )],
            ))
            .child(panel_body(fields))
            .child(panel_footer(k, vec![cancel, save])),
    );

    let storage = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Storage",
                Some("What leaves this form."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(fact(k, "Token", "OS keyring", k.c.success.icon))
                    .child(Divider::horizontal().build(k))
                    .child(fact(k, "Metadata", "bulbs.json", k.c.info.icon))
                    .child(Divider::horizontal().build(k))
                    .child(fact(k, "Transport", "miIO over UDP", k.c.info.icon))
                    .child(Divider::horizontal().build(k))
                    .child(notice(
                        k,
                        &crate::ui::widget::BULB,
                        "The token never enters the config file, a log line or an error message.",
                        k.c.info,
                    )),
            )),
    );

    let rules = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Checks",
                Some("Before the bulb is saved."),
                Vec::new(),
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(fact(k, "Name", "Required · unique", k.c.text_faint))
                    .child(Divider::horizontal().build(k))
                    .child(fact(k, "Address", "IPv4 · unique", k.c.text_faint))
                    .child(Divider::horizontal().build(k))
                    .child(fact(
                        k,
                        "Token",
                        "32 hexadecimal characters",
                        k.c.text_faint,
                    )),
            )),
    );

    let scroll = col()
        .key("bulb-form")
        .w_full()
        .padding(18.0)
        .gap(14.0)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .items_start()
                .child(col().grow(2.0).min_w(0.0).child(form))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(290.0)
                        .gap(14.0)
                        .child(storage)
                        .child(rules),
                ),
        )
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);

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
    let model = bulb
        .and_then(|bulb| bulb.model.clone())
        .unwrap_or_else(|| "Unknown model".to_owned());

    let cancel = Button::new("Cancel")
        .variant(ButtonVariant::Ghost)
        .on_press(|app: &mut App| app.back())
        .build(k);
    let remove = Button::new("Remove bulb")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Regular)
        .on_press_tx(|app: &mut App, tx| {
            app.confirm_remove();
            tx.wake_after(std::time::Duration::ZERO);
        })
        .build(k);

    let body = col()
        .w_full()
        .child(panel_header(
            k,
            "Remove bulb",
            Some("This removes the metadata and the keyring entry."),
            vec![badge(k, "Cannot undo", k.c.danger.icon)],
        ))
        .child(panel_body(
            col()
                .w_full()
                .gap(0.0)
                .child(
                    row()
                        .w_full()
                        .gap(10.0)
                        .padding(12.0)
                        .radius(k.radius.md)
                        .fill(k.c.danger.bg)
                        .child(rune::icon(&TRASH, 18.0).color(k.c.danger.icon))
                        .child(
                            text(format!("{name} will be removed from this app."))
                                .font(k.ty.ui_medium)
                                .color(k.c.danger.fg)
                                .wrap(),
                        ),
                )
                .child(fact(k, "Name", name, k.c.text_faint))
                .child(Divider::horizontal().build(k))
                .child(fact(k, "Address", address, k.c.text_faint))
                .child(Divider::horizontal().build(k))
                .child(fact(k, "Model", model, k.c.text_faint)),
        ))
        .child(panel_footer(k, vec![cancel, remove]));

    let panel = panel(k, body);
    col()
        .key("remove")
        .w_full()
        .h_full()
        .center()
        .padding(24.0)
        .child(col().w_full().max_w(600.0).child(panel))
}

fn control(k: &Kit, input: El, error: Option<&String>) -> El {
    let mut body = col().w(320.0).gap(4.0).child(input);
    if let Some(error) = error {
        body = body.child(crate::ui::widget::error_text(k, error));
    }
    body
}

fn fact(k: &Kit, label: &str, value: impl Into<String>, tone: rune::Color) -> El {
    row()
        .w_full()
        .justify_between()
        .gap(12.0)
        .py(8.0)
        .center_y()
        .child(
            text(label.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            row()
                .gap(6.0)
                .center_y()
                .child(crate::ui::widget::dot(tone))
                .child(text(value.into()).font(k.ty.small).color(k.c.text_base)),
        )
}
