use super::*;
use pretty_assertions::assert_eq;

const COLORS: DefaultColors = DefaultColors {
    fg: (12, 34, 56),
    bg: (210, 220, 230),
};

fn reports(colors: DefaultColors) -> [EventWithColor; 2] {
    let (r, g, b) = colors.fg;
    let fg = EventWithColor::ForegroundColor(Color::Rgb { r, g, b });
    let (r, g, b) = colors.bg;
    [fg, EventWithColor::BackgroundColor(Color::Rgb { r, g, b })]
}

#[tokio::test(start_paused = true)]
async fn complete_pair_and_focus_cooldown_manual_bypass() {
    let mut refresh = ColorRefresh::default();
    let mut output = Vec::new();
    let [fg, bg] = reports(COLORS);
    refresh.request(RefreshReason::Manual, &mut output);
    let query = output.clone();
    assert_eq!(refresh.report(&bg, &mut output), None);
    assert_eq!(refresh.report(&fg, &mut output), Some(COLORS));
    refresh.request(RefreshReason::Focus, &mut output);
    assert_eq!(output, query);
    refresh.request(RefreshReason::Manual, &mut output);
    assert_eq!(output, query.repeat(2));
    refresh.report(&fg, &mut output);
    refresh.report(&bg, &mut output);
    tokio::time::advance(TIMEOUT).await;
    refresh.request(RefreshReason::Focus, &mut output);
    assert_eq!(output, query.repeat(3));
}

#[tokio::test(start_paused = true)]
async fn duplicate_requests_coalesce_but_theme_schedules_one_follow_up() {
    let mut refresh = ColorRefresh::default();
    let mut output = Vec::new();
    refresh.request(RefreshReason::Manual, &mut output);
    let query = output.clone();
    for reason in [
        RefreshReason::Manual,
        RefreshReason::Focus,
        RefreshReason::Theme,
        RefreshReason::Theme,
    ] {
        refresh.request(reason, &mut output);
    }
    assert_eq!(output, query);
    let [fg, bg] = reports(COLORS);
    refresh.report(&fg, &mut output);
    assert_eq!(refresh.report(&bg, &mut output), Some(COLORS));
    assert_eq!(output, query.repeat(2));
    refresh.report(&fg, &mut output);
    assert_eq!(refresh.report(&bg, &mut output), Some(COLORS));
    assert_eq!(output, query.repeat(2));
}

#[tokio::test(start_paused = true)]
async fn partial_timeout_ignores_delayed_and_unsolicited_replies() {
    let mut refresh = ColorRefresh::default();
    let mut output = Vec::new();
    let [fg, bg] = reports(COLORS);
    assert_eq!(refresh.report(&fg, &mut output), None);
    refresh.request(RefreshReason::Manual, &mut output);
    assert_eq!(refresh.report(&fg, &mut output), None);
    tokio::time::advance(TIMEOUT).await;
    assert_eq!(refresh.report(&bg, &mut output), None);
    assert_eq!(refresh.report(&fg, &mut output), None);
    assert!(refresh.pending.is_none());
    assert!(refresh.last_success.is_none());
    refresh.request(RefreshReason::Manual, &mut output);
    assert_eq!(refresh.report(&bg, &mut output), None);
    assert_eq!(refresh.report(&fg, &mut output), Some(COLORS));
}

#[tokio::test(start_paused = true)]
async fn deadline_wakes_without_input_and_runs_pending_theme_query() {
    let mut refresh = ColorRefresh::default();
    let mut output = Vec::new();
    refresh.request(RefreshReason::Manual, &mut output);
    let query = output.clone();
    refresh.request(RefreshReason::Theme, &mut output);
    std::future::poll_fn(|cx| {
        refresh.poll_timeout(cx, &mut output);
        if output.len() == query.len() * 2 {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    assert_eq!(output, query.repeat(2));
}

#[tokio::test]
async fn failed_query_keeps_controller_idle_and_palette_pair_is_atomic() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("closed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut refresh = ColorRefresh::default();
    refresh.request(RefreshReason::Manual, &mut Broken);
    assert!(refresh.pending.is_none());
    crate::terminal_palette::with_test_default_colors(COLORS, || {
        assert!(!crate::terminal_palette::update_default_colors(COLORS));
        let next = DefaultColors {
            fg: COLORS.bg,
            bg: COLORS.fg,
        };
        assert!(crate::terminal_palette::update_default_colors(next));
        assert_eq!(
            (
                crate::terminal_palette::default_fg(),
                crate::terminal_palette::default_bg()
            ),
            (Some(next.fg), Some(next.bg))
        );
    });
}
