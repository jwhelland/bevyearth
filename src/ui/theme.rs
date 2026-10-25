//! Application palette layered on Feathers semantic themes.
use bevy::feathers::{
    dark_theme::create_dark_theme,
    palette,
    theme::{SemanticToken, ThemeProps, ThemeToken, UiTheme},
    tokens,
};
use bevy::prelude::*;

fn set_color(theme: &mut ThemeProps, token: ThemeToken, color: Color) {
    // Each override has its own semantic token: shared default tokens must not
    // accidentally recolor a different widget or inherit contextual overrides.
    let semantic = SemanticToken::new(format!("bevyearth.{token}").into());
    theme.token_assignments.insert(token, semantic.clone());
    theme.semantic_base.insert(semantic, color);
}

pub fn app_theme() -> UiTheme {
    let mut theme = create_dark_theme();
    set_color(&mut theme, tokens::TEXT_MAIN, palette::LIGHT_GRAY_1);
    set_color(&mut theme, tokens::TEXT_DIM, palette::LIGHT_GRAY_2);
    set_color(&mut theme, tokens::FOCUS_RING, palette::ACCENT);
    set_color(
        &mut theme,
        tokens::SLIDER_BG,
        Color::srgba(0.04, 0.08, 0.12, 0.9),
    );
    set_color(
        &mut theme,
        tokens::SLIDER_BAR,
        Color::srgba(0.18, 0.7, 0.8, 0.7),
    );
    set_color(
        &mut theme,
        tokens::SLIDER_BAR_DISABLED,
        Color::srgba(0.1, 0.3, 0.35, 0.45),
    );
    set_color(
        &mut theme,
        tokens::SLIDER_TEXT,
        Color::srgba(0.5, 0.9, 0.95, 0.95),
    );
    set_color(
        &mut theme,
        tokens::SLIDER_TEXT_DISABLED,
        Color::srgba(0.4, 0.55, 0.6, 0.7),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_BG,
        Color::srgba(0.06, 0.12, 0.16, 0.9),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_BG_HOVER,
        Color::srgba(0.08, 0.2, 0.26, 0.95),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_BG_PRESSED,
        Color::srgba(0.1, 0.26, 0.32, 0.95),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_BG_DISABLED,
        Color::srgba(0.08, 0.12, 0.15, 0.5),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_TEXT,
        Color::srgba(0.6, 1.0, 1.0, 1.0),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_TEXT_DISABLED,
        Color::srgba(0.45, 0.6, 0.65, 0.7),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_PRIMARY_BG,
        Color::srgba(0.08, 0.22, 0.28, 0.95),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_PRIMARY_BG_HOVER,
        Color::srgba(0.1, 0.28, 0.36, 0.98),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_PRIMARY_BG_PRESSED,
        Color::srgba(0.12, 0.32, 0.4, 0.98),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_PRIMARY_BG_DISABLED,
        Color::srgba(0.08, 0.16, 0.2, 0.6),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_PRIMARY_TEXT,
        Color::srgba(0.7, 1.0, 1.0, 1.0),
    );
    set_color(
        &mut theme,
        tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        Color::srgba(0.45, 0.6, 0.65, 0.7),
    );
    UiTheme(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::feathers::theme::SurfaceLevel;

    #[test]
    fn every_theme_token_resolves_in_every_context() {
        let theme = app_theme();
        for semantic in theme.0.token_assignments.values() {
            assert!(theme.0.semantic_base.contains_key(semantic));
        }
        for context in [
            SurfaceLevel::Base,
            SurfaceLevel::Higher,
            SurfaceLevel::Highest,
            SurfaceLevel::Floating,
        ] {
            assert_eq!(
                theme.context_color(&tokens::BUTTON_BG, context),
                Color::srgba(0.06, 0.12, 0.16, 0.9)
            );
            assert_eq!(
                theme.context_color(&tokens::SLIDER_BG, context),
                Color::srgba(0.04, 0.08, 0.12, 0.9)
            );
            assert_eq!(
                theme.context_color(&tokens::FOCUS_RING, context),
                palette::ACCENT
            );
        }
        assert_ne!(
            theme.0.token_assignments[&tokens::BUTTON_BG],
            theme.0.token_assignments[&tokens::SLIDER_BG]
        );
    }
}
