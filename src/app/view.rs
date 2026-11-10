use std::collections::HashMap;
use std::time::Instant;

use rune::TextEdit;

use crate::bulb::model::{BulbField, BulbState, Rgb};
use crate::screen::color::rgb_to_hsb;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormMode {
    Add,
    Edit(String),
}

pub struct FormState {
    pub name: TextEdit,
    pub address: TextEdit,
    pub model: TextEdit,
    pub token: TextEdit,
    pub reveal: bool,
    pub errors: HashMap<BulbField, String>,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            name: TextEdit::new(""),
            address: TextEdit::new(""),
            model: TextEdit::new(""),
            token: TextEdit::new(""),
            reveal: false,
            errors: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavDirection {
    #[default]
    Forward,
    Backward,
}

#[derive(Default)]
pub struct Navigation {
    pub direction: NavDirection,
    pub pending: Option<(View, Instant)>,
    pub entering_until: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overlay {
    #[default]
    None,
    BulbPicker,
    BulbMenu,
    MonitorPicker,
}

impl Overlay {
    pub fn toggle(self, overlay: Self) -> Self {
        if self == overlay { Self::None } else { overlay }
    }

    pub fn close(self, overlay: Self) -> Self {
        if self == overlay { Self::None } else { self }
    }
}

pub enum ColorEditor {
    Closed,
    Open(Box<ColorSession>),
}

pub struct ColorSession {
    pub targets: Vec<String>,
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub hex: TextEdit,
    pub preview: Rgb,
    pub error: Option<String>,
    pre_open: HashMap<String, Option<BulbState>>,
}

impl ColorSession {
    fn new(targets: Vec<String>, color: Rgb, pre_open: HashMap<String, Option<BulbState>>) -> Self {
        let hsb = rgb_to_hsb(color);
        Self {
            targets,
            hue: hsb.hue,
            saturation: hsb.saturation,
            value: hsb.value,
            hex: TextEdit::new(color.hex()),
            preview: color,
            error: None,
            pre_open,
        }
    }

    pub fn into_pre_open(self) -> HashMap<String, Option<BulbState>> {
        self.pre_open
    }
}

impl ColorEditor {
    pub fn open(
        targets: Vec<String>,
        color: Rgb,
        pre_open: HashMap<String, Option<BulbState>>,
    ) -> Self {
        Self::Open(Box::new(ColorSession::new(targets, color, pre_open)))
    }

    fn closed() -> Self {
        Self::Closed
    }

    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open(_))
    }

    pub fn session(&self) -> Option<&ColorSession> {
        match self {
            Self::Closed => None,
            Self::Open(session) => Some(session),
        }
    }

    pub fn session_mut(&mut self) -> Option<&mut ColorSession> {
        match self {
            Self::Closed => None,
            Self::Open(session) => Some(session),
        }
    }

    pub fn take(&mut self) -> Option<ColorSession> {
        match std::mem::take(self) {
            Self::Open(session) => Some(*session),
            Self::Closed => None,
        }
    }

    pub fn commit(&mut self) -> Option<Rgb> {
        let color = self.session()?.preview;
        *self = Self::closed();
        Some(color)
    }
}

impl Default for ColorEditor {
    fn default() -> Self {
        Self::closed()
    }
}

pub struct ControlView {
    pub overlay: Overlay,
    pub color: ColorEditor,
}

impl Default for ControlView {
    fn default() -> Self {
        Self {
            overlay: Overlay::None,
            color: ColorEditor::default(),
        }
    }
}

pub struct SyncView {
    pub overlay: Overlay,
}

impl Default for SyncView {
    fn default() -> Self {
        Self {
            overlay: Overlay::None,
        }
    }
}

pub enum View {
    Control(ControlView),
    Sync(SyncView),
    Settings,
    Form {
        mode: FormMode,
        state: Box<FormState>,
    },
    Remove(String),
}

impl View {
    pub fn control() -> Self {
        Self::Control(ControlView::default())
    }

    pub fn sync() -> Self {
        Self::Sync(SyncView::default())
    }

    pub fn element_key(&self) -> &'static str {
        match self {
            Self::Control(_) => "control",
            Self::Sync(_) => "sync",
            Self::Settings => "settings",
            Self::Form { .. } => "form",
            Self::Remove(_) => "remove",
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Control(_) => 0,
            Self::Sync(_) => 1,
            Self::Settings => 2,
            Self::Form { .. } | Self::Remove(_) => 3,
        }
    }

    pub fn same_place(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Control(_), Self::Control(_))
            | (Self::Sync(_), Self::Sync(_))
            | (Self::Settings, Self::Settings) => true,
            (Self::Form { mode: left, .. }, Self::Form { mode: right, .. }) => left == right,
            (Self::Remove(left), Self::Remove(right)) => left == right,
            _ => false,
        }
    }

    pub fn as_control(&self) -> Option<&ControlView> {
        match self {
            Self::Control(control) => Some(control),
            _ => None,
        }
    }

    pub fn as_control_mut(&mut self) -> Option<&mut ControlView> {
        match self {
            Self::Control(control) => Some(control),
            _ => None,
        }
    }

    pub fn overlay(&self) -> Overlay {
        match self {
            Self::Control(control) => control.overlay,
            Self::Sync(sync) => sync.overlay,
            _ => Overlay::None,
        }
    }

    pub fn overlay_mut(&mut self) -> Option<&mut Overlay> {
        match self {
            Self::Control(control) => Some(&mut control.overlay),
            Self::Sync(sync) => Some(&mut sync.overlay),
            _ => None,
        }
    }

    pub fn color(&self) -> Option<&ColorEditor> {
        self.as_control().map(|control| &control.color)
    }

    pub fn form(&self) -> Option<&FormState> {
        match self {
            Self::Form { state, .. } => Some(state.as_ref()),
            _ => None,
        }
    }

    pub fn form_mut(&mut self) -> Option<&mut FormState> {
        match self {
            Self::Form { state, .. } => Some(state.as_mut()),
            _ => None,
        }
    }

    pub fn close_transients(&mut self) {
        match self {
            Self::Control(control) => {
                control.overlay = Overlay::None;
                control.color = ColorEditor::default();
            }
            Self::Sync(sync) => sync.overlay = Overlay::None,
            _ => {}
        }
    }

    /// Navigation closes the Control overlay and colour editor, and the Sync
    /// overlay. A Control or Sync destination starts closed, even when it is
    /// the current screen.
    pub fn navigate(&mut self, next: Self) {
        *self = match next {
            Self::Control(_) => Self::control(),
            Self::Sync(_) => Self::sync(),
            view => view,
        };
    }
}

pub fn nav_direction(from: &View, to: &View) -> NavDirection {
    if matches!(from, View::Form { .. } | View::Remove(_)) {
        if matches!(to, View::Control(_)) {
            NavDirection::Backward
        } else {
            NavDirection::Forward
        }
    } else if matches!(to, View::Form { .. } | View::Remove(_)) {
        NavDirection::Forward
    } else if to.rank() < from.rank() {
        NavDirection::Backward
    } else {
        NavDirection::Forward
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rune::TextEdit;

    use super::{
        ColorEditor, ControlView, FormMode, FormState, NavDirection, Overlay, View, nav_direction,
    };
    use crate::bulb::model::{BulbState, ColorMode, Rgb};

    fn editor() -> ColorEditor {
        ColorEditor::open(vec!["desk".to_owned()], Rgb::new(255, 128, 0), pre_open())
    }

    fn control(overlay: Overlay) -> View {
        View::Control(ControlView {
            overlay,
            color: editor(),
        })
    }

    #[test]
    fn navigating_from_control_closes_the_overlay_and_colour_editor() {
        let destinations = [
            View::sync(),
            View::Settings,
            View::Form {
                mode: FormMode::Add,
                state: Box::new(FormState::default()),
            },
            View::Remove("desk".to_owned()),
            control(Overlay::BulbMenu),
        ];

        for destination in destinations {
            let mut view = control(Overlay::BulbPicker);
            view.navigate(destination);

            match &view {
                View::Control(control) => {
                    assert_eq!(control.overlay, Overlay::None);
                    assert!(!control.color.is_open());
                }
                other => {
                    assert!(other.as_control().is_none());
                    assert!(other.color().is_none());
                }
            }
        }
    }

    #[test]
    fn only_control_carries_the_overlay_and_colour_editor() {
        let views = [
            View::sync(),
            View::Settings,
            View::Form {
                mode: FormMode::Add,
                state: Box::new(FormState::default()),
            },
            View::Remove("desk".to_owned()),
        ];

        for mut view in views {
            assert!(view.as_control().is_none());
            assert!(view.as_control_mut().is_none());
            assert_eq!(view.overlay(), Overlay::None);
            assert!(view.color().is_none());

            view.navigate(control(Overlay::BulbMenu));
            assert!(matches!(view, View::Control(_)));
            let View::Control(control) = view else {
                return;
            };
            assert_eq!(control.overlay, Overlay::None);
            assert!(!control.color.is_open());
        }
    }

    #[test]
    fn navigating_between_forms_keeps_the_destination_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut view = View::Form {
            mode: FormMode::Add,
            state: Box::new(FormState {
                name: TextEdit::new("Desk"),
                ..FormState::default()
            }),
        };
        let destination = View::Form {
            mode: FormMode::Edit("desk".to_owned()),
            state: Box::new(FormState {
                name: TextEdit::new("Shelf"),
                ..FormState::default()
            }),
        };

        view.navigate(destination);

        let View::Form { mode, state } = &view else {
            return Err("the destination should be a form".into());
        };
        assert_eq!(mode, &FormMode::Edit("desk".to_owned()));
        assert_eq!(state.name.text, "Shelf");
        Ok(())
    }

    #[test]
    fn same_place_ignores_the_form_contents() {
        let typed = View::Form {
            mode: FormMode::Add,
            state: Box::new(FormState {
                name: TextEdit::new("Desk"),
                ..FormState::default()
            }),
        };
        let fresh = View::Form {
            mode: FormMode::Add,
            state: Box::new(FormState::default()),
        };

        assert!(typed.same_place(&fresh));
        assert!(!typed.same_place(&View::sync()));
    }

    #[test]
    fn section_changes_push_in_tab_order() {
        assert_eq!(
            nav_direction(&View::control(), &View::sync()),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(&View::sync(), &View::Settings),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(&View::Settings, &View::control()),
            NavDirection::Backward
        );
    }

    #[test]
    fn subpages_open_forward_and_return_backward() {
        let form = View::Form {
            mode: FormMode::Add,
            state: Box::new(FormState::default()),
        };
        assert_eq!(
            nav_direction(&View::control(), &form),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(
                &View::Form {
                    mode: FormMode::Edit("id".to_owned()),
                    state: Box::new(FormState::default()),
                },
                &View::control(),
            ),
            NavDirection::Backward
        );
        assert_eq!(
            nav_direction(&View::Settings, &View::Remove("id".to_owned())),
            NavDirection::Forward
        );
    }

    #[test]
    fn leaving_a_subpage_follows_the_tab_order() {
        let form = View::Form {
            mode: FormMode::Add,
            state: Box::new(FormState::default()),
        };
        assert_eq!(nav_direction(&form, &View::sync()), NavDirection::Forward);
        assert_eq!(nav_direction(&form, &View::Settings), NavDirection::Forward);
        assert_eq!(
            nav_direction(&form, &View::control()),
            NavDirection::Backward
        );
    }

    #[test]
    fn only_one_overlay_is_open() {
        assert_eq!(
            Overlay::None.toggle(Overlay::BulbPicker),
            Overlay::BulbPicker
        );
        assert_eq!(
            Overlay::BulbPicker.toggle(Overlay::BulbMenu),
            Overlay::BulbMenu
        );
        assert_eq!(Overlay::BulbMenu.toggle(Overlay::BulbMenu), Overlay::None);
        assert_eq!(
            Overlay::BulbMenu.close(Overlay::BulbPicker),
            Overlay::BulbMenu
        );
        assert_eq!(Overlay::BulbMenu.close(Overlay::BulbMenu), Overlay::None);
    }

    #[test]
    fn a_closed_editor_has_no_session() {
        let mut editor = ColorEditor::default();

        assert!(!editor.is_open());
        assert!(editor.session().is_none());
        assert!(editor.session_mut().is_none());
        assert!(editor.take().is_none());
        assert!(!editor.is_open());
    }

    fn state(color: Rgb) -> BulbState {
        BulbState {
            power: Some(true),
            brightness: Some(40),
            color: Some(color),
            mode: ColorMode::Color,
            ..BulbState::default()
        }
    }

    fn pre_open() -> HashMap<String, Option<BulbState>> {
        HashMap::from([
            ("desk".to_owned(), Some(state(Rgb::new(10, 20, 30)))),
            ("shelf".to_owned(), None),
        ])
    }

    #[test]
    fn opening_captures_targets_and_pre_open_values() -> Result<(), Box<dyn std::error::Error>> {
        let before = pre_open();
        let mut editor = ColorEditor::open(
            vec!["desk".to_owned(), "shelf".to_owned()],
            Rgb::new(255, 128, 0),
            before.clone(),
        );

        let session = editor.session().ok_or("the editor should be open")?;
        assert_eq!(session.targets, vec!["desk".to_owned(), "shelf".to_owned()]);
        assert_eq!(session.preview, Rgb::new(255, 128, 0));
        assert_eq!(session.hex.text, "#FF8000");

        let cancelled = editor
            .take()
            .ok_or("the editor should be open")?
            .into_pre_open();
        assert_eq!(cancelled, before);
        assert!(!editor.is_open());
        Ok(())
    }

    #[test]
    fn committing_returns_the_preview_and_closes() -> Result<(), Box<dyn std::error::Error>> {
        let mut editor =
            ColorEditor::open(vec!["desk".to_owned()], Rgb::new(255, 128, 0), pre_open());

        let preview = editor.commit().ok_or("the editor should commit")?;

        assert_eq!(preview, Rgb::new(255, 128, 0));
        assert!(!editor.is_open());
        assert!(editor.take().is_none());
        Ok(())
    }
}
