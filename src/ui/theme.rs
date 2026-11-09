//! The Lamplight theme: warm paper in light, espresso in dark, brass for
//! interaction. Every value here is a semantic token; screens never name a raw
//! colour.

use std::sync::OnceLock;

use rune::{Color, Family, Shadow, TextStyle, oklch};
use rune_kit::{Elevation, Kit, Radius, StateColors, Type};

const PAPER: (f32, f32, f32) = (0.958, 0.012, 85.0);
const PAPER_RAISED: (f32, f32, f32) = (0.985, 0.008, 85.0);
const PAPER_DEEP: (f32, f32, f32) = (0.936, 0.015, 84.0);
const INK: (f32, f32, f32) = (0.255, 0.018, 65.0);
const MUTED: (f32, f32, f32) = (0.47, 0.02, 65.0);
const FAINT: (f32, f32, f32) = (0.52, 0.02, 65.0);
const BRASS: (f32, f32, f32) = (0.55, 0.135, 50.0);
const BRASS_DEEP: (f32, f32, f32) = (0.47, 0.13, 48.0);

const NIGHT: (f32, f32, f32) = (0.155, 0.012, 70.0);
const NIGHT_RAISED: (f32, f32, f32) = (0.205, 0.014, 70.0);
const NIGHT_DEEP: (f32, f32, f32) = (0.135, 0.01, 70.0);
const CREAM: (f32, f32, f32) = (0.93, 0.018, 85.0);
const NIGHT_MUTED: (f32, f32, f32) = (0.76, 0.022, 80.0);
const NIGHT_FAINT: (f32, f32, f32) = (0.62, 0.02, 75.0);
const GOLD: (f32, f32, f32) = (0.72, 0.125, 65.0);
const GOLD_BRIGHT: (f32, f32, f32) = (0.78, 0.12, 68.0);

pub fn kit(dark: bool) -> &'static Kit {
    static LIGHT: OnceLock<Kit> = OnceLock::new();
    static DARK: OnceLock<Kit> = OnceLock::new();
    if dark {
        DARK.get_or_init(|| build(true))
    } else {
        LIGHT.get_or_init(|| build(false))
    }
}

fn build(dark: bool) -> Kit {
    let mut kit = Kit::neutral(dark);
    kit.c = palette(dark);
    kit.ty = typography();
    kit.radius = Radius {
        xs: 3.0,
        sm: 6.0,
        md: 10.0,
        lg: 14.0,
        xl: 20.0,
    };
    kit.elevation = elevation(dark);
    kit.invalid_text = true;
    kit
}

fn palette(dark: bool) -> rune_kit::Palette {
    let ink = pal(INK);
    let cream = pal(CREAM);
    let accent = pal(if dark { GOLD } else { BRASS });
    let accent_text = pal(if dark { GOLD_BRIGHT } else { BRASS_DEEP });
    let hover = if dark {
        cream.alpha(0.06)
    } else {
        ink.alpha(0.045)
    };
    let pressed = if dark {
        cream.alpha(0.1)
    } else {
        ink.alpha(0.08)
    };
    let text_base = pal(if dark { CREAM } else { INK });
    let text_muted = pal(if dark { NIGHT_MUTED } else { MUTED });
    let text_faint = pal(if dark { NIGHT_FAINT } else { FAINT });
    let layer_01 = pal(if dark {
        (0.19, 0.014, 70.0)
    } else {
        (0.975, 0.01, 85.0)
    });
    let layer_02 = pal(if dark {
        (0.26, 0.018, 70.0)
    } else {
        (0.95, 0.014, 85.0)
    });
    let layer_03 = pal(if dark {
        (0.31, 0.022, 70.0)
    } else {
        (0.925, 0.018, 82.0)
    });
    let layer_04 = pal(if dark {
        (0.36, 0.026, 70.0)
    } else {
        (0.845, 0.022, 80.0)
    });
    let border_muted = pal(if dark {
        (0.288, 0.014, 70.0)
    } else {
        (0.915, 0.014, 82.0)
    });
    let border_base = pal(if dark {
        (0.334, 0.018, 70.0)
    } else {
        (0.875, 0.018, 80.0)
    });
    let border_strong = pal(if dark {
        (0.405, 0.022, 70.0)
    } else {
        (0.81, 0.025, 78.0)
    });
    let bg_base = pal(if dark { NIGHT_RAISED } else { PAPER_RAISED });
    let bg_deep = pal(if dark { NIGHT_DEEP } else { PAPER_DEEP });
    let bg_inverse = if dark { cream } else { ink };
    let text_inverse = if dark { ink } else { cream };

    rune_kit::Palette {
        page: pal(if dark { NIGHT } else { PAPER }),
        bg_base,
        bg_deep,
        layer_01,
        layer_02,
        layer_03,
        layer_04,
        bg_inverse,
        bg_contrast: bg_inverse,
        bg_button_neutral: bg_base,
        bg_icon_button_contrast: bg_inverse,
        bg_accent: accent,
        submit_highlight: cream.alpha(0.2),
        submit_hover: cream.alpha(0.12),
        submit_fg: text_inverse,
        submit_icon: text_inverse,
        text_base,
        text_muted,
        text_faint,
        text_inverse,
        text_contrast: text_inverse,
        text_accent: accent_text,
        icon_base: text_base,
        icon_muted: text_muted,
        icon_faint: text_faint,
        icon_inverse: text_inverse,
        icon_contrast: text_inverse,
        icon_accent: accent,
        border_muted,
        border_base,
        border_strong,
        border_focus: text_base,
        hover,
        pressed,
        contrast_hover: cream.alpha(0.1),
        contrast_pressed: cream.alpha(0.2),
        scrim: Color::BLACK.alpha(if dark { 0.55 } else { 0.35 }),
        depth_top: Color::WHITE.alpha(if dark { 0.04 } else { 0.6 }),
        depth_bottom: Color::WHITE.alpha(0.0),
        white: Color::WHITE,
        black: Color::BLACK,
        knob: pal(CREAM),
        knob_on: pal(CREAM),
        success: state(dark, 145.0),
        warning: state(dark, 80.0),
        danger: state(dark, 28.0),
        info: state(dark, 240.0),
        surface_base: layer_01,
        surface_weak: layer_02,
        surface_hover: hover,
        surface_raised: bg_base,
        surface_raised_base: bg_base,
        surface_raised_hover: hover,
        surface_float: bg_base,
        surface_disabled: layer_02,
        border_weak: border_muted,
        border_hover: border_base,
        border_active: accent,
        border_disabled: border_muted,
        text_strong: text_base,
        text_weak: text_muted,
        text_invert_strong: text_inverse,
        icon_weaker: text_faint,
        std_border: border_base,
        std_icon: text_muted,
        std_icon_invert: text_inverse,
        std_icon_strong: text_base,
        std_text: text_base,
    }
}

fn state(dark: bool, hue: f32) -> StateColors {
    if dark {
        StateColors {
            bg: oklch(0.285, 0.04, hue),
            fg: oklch(0.79, 0.1, hue),
            border: oklch(0.39, 0.05, hue),
            icon: oklch(0.79, 0.1, hue),
        }
    } else {
        StateColors {
            bg: oklch(0.955, 0.035, hue),
            fg: oklch(0.44, 0.13, hue),
            border: oklch(0.87, 0.06, hue),
            icon: oklch(0.44, 0.13, hue),
        }
    }
}

fn typography() -> Type {
    let ui = TextStyle::new(14.0, 450.0).lh(20.0).tracking(-0.05);
    Type {
        ui,
        ui_medium: TextStyle {
            weight: 560.0,
            ..ui
        },
        ui_compact: ui.lh(18.0),
        small: TextStyle::new(12.0, 450.0).lh(16.0).tracking(0.02),
        small_medium: TextStyle::new(12.0, 570.0).lh(16.0).tracking(0.02),
        body: TextStyle::new(15.0, 430.0).lh(22.0),
        body_medium: TextStyle::new(15.0, 580.0).lh(22.0),
        text_small: TextStyle::new(13.0, 450.0).lh(19.0),
        text_small_medium: TextStyle::new(13.0, 570.0).lh(19.0),
        text_xsmall: TextStyle::new(12.0, 450.0).lh(17.0),
        text_xsmall_medium: TextStyle::new(12.0, 570.0).lh(17.0),
        mono: TextStyle::new(12.0, 450.0).mono().lh(18.0).tracking(0.1),
    }
}

fn elevation(dark: bool) -> Elevation {
    let shadow = if dark {
        Color::BLACK.alpha(0.5)
    } else {
        oklch(0.35, 0.03, 60.0).alpha(0.1)
    };
    let soft = if dark {
        Color::BLACK.alpha(0.35)
    } else {
        oklch(0.35, 0.03, 60.0).alpha(0.07)
    };
    let ring = |c: Color| Shadow {
        spread: 0.5,
        color: c,
        ..Shadow::default()
    };
    let drop = |y: f32, blur: f32, color: Color| Shadow {
        y,
        blur,
        color,
        ..Shadow::default()
    };
    let border = if dark {
        oklch(0.36, 0.02, 70.0).alpha(0.9)
    } else {
        oklch(0.7, 0.025, 75.0).alpha(0.35)
    };
    let mut elevation = Elevation::default();
    elevation.raised = vec![drop(1.0, 2.0, soft), ring(border)];
    elevation.floating = vec![drop(5.0, 12.0, shadow), ring(border)];
    elevation.overlay = vec![drop(12.0, 28.0, shadow), ring(border)];
    elevation.button_neutral = vec![drop(1.0, 1.5, soft), ring(border)];
    elevation.button_contrast = vec![drop(1.0, 2.0, shadow)];
    elevation.elements = vec![drop(0.5, 0.5, soft)];
    elevation.md = vec![drop(4.0, 10.0, soft)];
    elevation.knob = vec![drop(1.0, 3.0, shadow)];
    elevation
}

/// A face with a point of view: Fraunces carries names and page titles; Inter
/// carries the interface.
pub fn display(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight)
        .family(Family::Named("Fraunces"))
        .lh((size * 1.08).ceil())
        .tracking(-0.2)
}

/// A data value (hex, address, path) in the mono face. Every glyph is the
/// same width, so a changing value never shifts its neighbours.
pub fn code(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight)
        .mono()
        .lh((size * 1.2).ceil())
        .tracking(0.0)
}

/// Live figures never jiggle: tabular widths, centred on the line.
pub fn number(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight)
        .lh((size * 1.12).ceil())
        .tracking(-0.4)
        .features("'tnum' 1")
}

/// A quiet control caption, set like a label on an instrument.
pub fn label() -> TextStyle {
    TextStyle::new(12.0, 600.0)
        .lh(16.0)
        .tracking(0.7)
        .features("'case' 1")
}

pub fn wordmark() -> TextStyle {
    TextStyle::new(17.0, 600.0)
        .family(Family::Named("Fraunces"))
        .lh(20.0)
        .tracking(0.1)
}

pub fn ink() -> Color {
    pal(INK)
}

/// The warm paper a stage's nameplate is printed on, in both appearances.
pub fn plate() -> Color {
    oklch(0.982, 0.008, 85.0)
}

/// Fixed state inks for a cream nameplate: the plate does not follow the
/// window appearance, so its dots must not either.
pub fn positive_on_plate() -> Color {
    oklch(0.52, 0.12, 148.0)
}

pub fn warning_on_plate() -> Color {
    oklch(0.6, 0.12, 72.0)
}

pub fn danger_on_plate() -> Color {
    oklch(0.53, 0.16, 28.0)
}

pub fn quiet_on_plate() -> Color {
    ink().alpha(0.4)
}

fn pal((l, c, h): (f32, f32, f32)) -> Color {
    oklch(l, c, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_use_the_serif_and_numbers_are_tabular() {
        assert!(matches!(
            display(28.0, 600.0).family,
            Family::Named("Fraunces")
        ));
        assert!(number(40.0, 600.0).features.contains("tnum"));
        assert!(matches!(code(30.0, 500.0).family, Family::Mono));
    }
}
