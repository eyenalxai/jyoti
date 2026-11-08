use rune::{Cx, El, ScrollSpec, col, row, text};
use rune_kit::shell::Page;
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Switch, TextInput,
};

use crate::app::App;
use crate::app::state::{FormMode, Screen};
use crate::bulb::model::BulbField;
use crate::ui::widget::{BACK, TRASH, error_text};

pub fn form_view(app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let editing = matches!(&app.screen, Screen::Form(FormMode::Edit(_)));
    let (title, save_label) = if editing {
        ("Edit bulb", "Save changes")
    } else {
        ("Add bulb", "Add bulb")
    };
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .help("Back")
        .on_press(|app: &mut App| app.back())
        .build(k);
    let save = Button::new(save_label)
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Regular)
        .on_press_tx(|app: &mut App, tx| {
            app.save_form();
            tx.wake_after(std::time::Duration::ZERO);
        })
        .build(k);

    let name = TextInput::new(&app.form.name)
        .label("Name")
        .placeholder("Desk lamp")
        .width(Some(520.0))
        .on_edit(|app: &mut App| &mut app.form.name)
        .build(k);
    let address = TextInput::new(&app.form.address)
        .label("IP address")
        .placeholder("192.168.1.42")
        .width(Some(520.0))
        .on_edit(|app: &mut App| &mut app.form.address)
        .build(k);
    let model = TextInput::new(&app.form.model)
        .label("Model (optional)")
        .placeholder("yeelink.light.color3")
        .width(Some(520.0))
        .on_edit(|app: &mut App| &mut app.form.model)
        .build(k);
    let token = TextInput::new(&app.form.token)
        .label("Token")
        .placeholder(if matches!(&app.screen, Screen::Form(FormMode::Add)) {
            "32 hexadecimal characters"
        } else {
            "Leave blank to keep the stored token"
        })
        .width(Some(520.0))
        .secure()
        .revealed(app.form.reveal)
        .on_edit(|app: &mut App| &mut app.form.token)
        .build(k);

    let reveal = row().gap(10.0).center_y().child(
        Switch::new(app.form.reveal)
            .control_size(ControlSize::Regular)
            .label("Show token")
            .on_toggle(|app: &mut App| app.form.reveal = !app.form.reveal)
            .build(k),
    );

    let mut body = col()
        .key("bulb-form")
        .w_full()
        .max_w(640.0)
        .padding(32.0)
        .gap(20.0)
        .child(
            text("The token authenticates every miIO packet. Jyoti stores it in the OS keyring, never in the config file."
                .to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(field(k, "Name", name, app.form.errors.get(&BulbField::Name)))
        .child(field(k, "IP address", address, app.form.errors.get(&BulbField::Address)))
        .child(field(k, "Model", model, app.form.errors.get(&BulbField::Model)))
        .child(field(k, "Token", token, app.form.errors.get(&BulbField::Token)))
        .child(reveal);

    if !app.form.errors.is_empty() {
        let empty = String::new();
        let message = app.form.errors.values().next().cloned().unwrap_or(empty);
        body = body.child(error_text(k, &message));
    }

    let body = body
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0);
    let body = row().w_full().h_full().justify_center().child(body);
    Page::new(body)
        .title(title)
        .leading(back)
        .trailing(save)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .build(k)
}

pub fn remove_view(app: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let id = match &app.screen {
        crate::app::state::Screen::Remove(id) => id.clone(),
        _ => String::new(),
    };
    let bulb = app.bulbs.iter().find(|bulb| bulb.id == id);
    let name = bulb.map_or_else(|| "this bulb".to_owned(), |bulb| bulb.name.clone());
    let back = IconButton::new(&BACK)
        .variant(IconButtonVariant::GhostMuted)
        .help("Back")
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
        .h_full()
        .center()
        .padding(32.0)
        .gap(16.0)
        .child(
            row()
                .size(44.0, 44.0)
                .center()
                .radius(22.0)
                .fill(k.c.danger.bg)
                .child(rune::icon(&TRASH, 20.0).color(k.c.danger.icon)),
        )
        .child(text(format!("Remove {name}?")).font(k.ty.body_medium).color(k.c.text_base))
        .child(
            text("The bulb's token is deleted from the OS keyring and its metadata leaves the config file. This cannot be undone."
                .to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap()
                .max_w(420.0),
        )
        .child(
            row()
                .gap(8.0)
                .child(
                    Button::new("Cancel")
                        .variant(ButtonVariant::Ghost)
                        .on_press(|app: &mut App| app.back())
                        .build(k),
                )
                .child(remove),
        );
    let body = row().w_full().h_full().justify_center().child(body);
    Page::new(body)
        .title("Remove bulb")
        .leading(back)
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .build(k)
}

fn field(k: &Kit, label: &str, input: El, error: Option<&String>) -> El {
    let mut field = col().w_full().gap(6.0).child(
        text(label.to_owned())
            .font(k.ty.small_medium)
            .color(k.c.text_muted),
    );
    field = field.child(input);
    if let Some(error) = error {
        field = field.child(error_text(k, error));
    }
    field
}
