use std::time::Duration;

use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Switch, TextInput,
};

use crate::app::App;
use crate::app::state::{FormMode, Screen};
use crate::bulb::model::BulbField;
use crate::ui::widget::{
    BACK, TRASH, badge, error_text, fact_row, notice, panel, panel_body, panel_footer,
    panel_header, subpage_header,
};

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
        .control_size(ControlSize::Regular)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);
    let save = Button::new(if editing { "Save" } else { "Add bulb" })
        .variant(ButtonVariant::Contrast)
        .on_press_tx(|app: &mut App, tx| {
            app.save_form();
            tx.wake_after(Duration::ZERO);
        })
        .build(k);

    let name = TextInput::new(&app.form.name)
        .label("Name")
        .placeholder("Desk lamp")
        .width(None)
        .on_edit(|app: &mut App| &mut app.form.name)
        .build(k);
    let address = TextInput::new(&app.form.address)
        .label("IP address")
        .placeholder("192.168.1.42")
        .width(None)
        .on_edit(|app: &mut App| &mut app.form.address)
        .build(k);
    let model = TextInput::new(&app.form.model)
        .label("Model")
        .placeholder("Optional")
        .width(None)
        .on_edit(|app: &mut App| &mut app.form.model)
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
        .on_edit(|app: &mut App| &mut app.form.token)
        .build(k);
    let reveal = Switch::new(app.form.reveal)
        .control_size(ControlSize::Small)
        .label("Show token")
        .on_toggle(|app: &mut App| app.form.reveal = !app.form.reveal)
        .build(k);

    let mut fields = col().w_full().gap(10.0);
    if let Some(error) = app.form.errors.values().next() {
        fields = fields.child(notice(k, &TRASH, error, k.c.danger));
    }
    fields = fields
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
                .gap(6.0)
                .child(field(
                    k,
                    "Token",
                    "Stored in the OS keyring.",
                    token,
                    app.form.errors.get(&BulbField::Token),
                ))
                .child(reveal),
        );

    let form = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Bulb details",
                Some(if editing {
                    "Leave the token blank to keep the stored one."
                } else {
                    "The token never enters config or logs."
                }),
                vec![badge(
                    k,
                    if editing { "Editing" } else { "New" },
                    k.c.info.icon,
                )],
            ))
            .child(panel_body(fields))
            .child(panel_footer(k, vec![save])),
    );

    let scroll = col()
        .key("bulb-form")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(subpage_header(k, title, description, back))
        .child(form)
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
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Regular)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);
    let remove = Button::new("Remove bulb")
        .variant(ButtonVariant::Danger)
        .on_press_tx(|app: &mut App, tx| {
            app.confirm_remove();
            tx.wake_after(Duration::ZERO);
        })
        .build(k);

    let warning = row()
        .w_full()
        .padding(11.0)
        .gap(9.0)
        .radius(k.radius.md)
        .fill(k.c.danger.bg)
        .border(1.0, k.c.danger.border)
        .child(rune::icon(&TRASH, 17.0).color(k.c.danger.icon))
        .child(
            text(format!("{name} will be removed from jyoti."))
                .font(k.ty.ui_medium)
                .color(k.c.danger.fg)
                .wrap()
                .grow(1.0),
        );

    let form = panel(
        k,
        col()
            .w_full()
            .child(panel_header(
                k,
                "Confirmation",
                Some("This cannot be undone."),
                vec![badge(k, "Destructive", k.c.danger.icon)],
            ))
            .child(panel_body(
                col()
                    .w_full()
                    .gap(0.0)
                    .child(warning)
                    .child(fact_row(k, "Name", name))
                    .child(fact_row(k, "Address", address)),
            ))
            .child(panel_footer(k, vec![remove])),
    );

    let scroll = col()
        .key("remove")
        .w_full()
        .padding(16.0)
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
        })
        .grow(1.0)
        .min_h(0.0);

    col().w_full().h_full().min_h(0.0).child(scroll)
}

fn field(k: &Kit, label: &str, description: &str, input: El, error: Option<&String>) -> El {
    let mut block = col().w_full().gap(4.0).child(
        text(label.to_owned())
            .font(k.ty.ui_medium)
            .color(k.c.text_base),
    );
    block = block.child(input);
    block = match error {
        Some(error) => block.child(error_text(k, error)),
        None => block.child(
            text(description.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        ),
    };
    block
}
