use super::*;
use crate::terminal_palette::with_test_default_colors;
use crate::terminal_probe::DefaultColors;
use pretty_assertions::assert_eq;

#[test]
fn automatic_theme_follows_palette_and_preview_cancel_uses_current_palette() {
    let light = DefaultColors {
        fg: (0, 0, 0),
        bg: (255, 255, 255),
    };
    let dark = DefaultColors {
        fg: light.bg,
        bg: light.fg,
    };
    with_test_default_colors(light, || {
        set_automatic_theme();
        let original = current_syntax_theme();
        let preview = ThemePreview::begin();
        let selected = resolve_theme_by_name("dracula", None).unwrap();
        set_syntax_theme(selected.clone());
        with_test_default_colors(dark, || {
            refresh_terminal_colors();
            assert_eq!(current_syntax_theme().settings, selected.settings);
            preview.cancel();
            assert_eq!(
                current_syntax_theme().settings,
                resolve_theme_with_override(None, None).settings
            );
        });
        refresh_terminal_colors();
        assert_eq!(current_syntax_theme().settings, original.settings);
        let preview = ThemePreview::begin();
        set_syntax_theme(selected.clone());
        preview.confirm();
        drop(preview);
        with_test_default_colors(dark, || {
            refresh_terminal_colors();
            assert_eq!(current_syntax_theme().settings, selected.settings);
        });
    });
}

#[test]
fn prompt_uses_complete_palette_pair_across_light_dark_light() {
    let light = DefaultColors {
        fg: (30, 30, 30),
        bg: (250, 250, 250),
    };
    let dark = DefaultColors {
        fg: (230, 230, 230),
        bg: (20, 20, 20),
    };
    let styles = [light, dark, light]
        .map(|colors| with_test_default_colors(colors, crate::style::user_message_style));
    assert_eq!(styles[0], styles[2]);
    insta::assert_debug_snapshot!(styles);
}
