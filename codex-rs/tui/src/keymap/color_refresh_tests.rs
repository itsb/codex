use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn refresh_colors_resolves_single_keys_unbinding_and_chords() {
    for (configured, expected) in [
        (json!("f12"), vec![key_hint::plain(KeyCode::F(12))]),
        (json!([]), Vec::new()),
    ] {
        let config =
            serde_json::from_value(json!({"global":{"refresh_colors":configured}})).unwrap();
        let runtime = RuntimeKeymap::from_config(&config).unwrap();
        assert_eq!(
            bindings_for_action(&runtime, "global", "refresh_colors"),
            Some(expected.as_slice())
        );
    }

    let config = serde_json::from_value(json!({"global":{"refresh_colors":"ctrl-x r"}})).unwrap();
    let runtime = RuntimeKeymap::from_config(&config).unwrap();
    let mut matcher = KeyChordMatcher::default();
    let context = KeymapContextSet::new(KeymapContext::Global);
    assert!(matches!(
        matcher.advance(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            &runtime.chords,
            context
        ),
        KeyChordMatch::Pending(_)
    ));
    let KeyChordMatch::Completed(key) =
        matcher.advance(KeyCode::Char('r').into(), &runtime.chords, context)
    else {
        panic!("refresh colors chord must complete")
    };
    assert!(runtime.app.refresh_colors.is_pressed(key));

    let config = serde_json::from_value(json!({"global":{
        "refresh_colors":"f12", "copy":"f12"
    }}))
    .unwrap();
    let error = RuntimeKeymap::from_config(&config).expect_err("ambiguous global binding");
    assert!(
        error.contains("refresh_colors") && error.contains("copy"),
        "{error}"
    );
}
