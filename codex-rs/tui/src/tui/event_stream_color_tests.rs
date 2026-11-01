use super::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::style::Color;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

struct Source(mpsc::UnboundedReceiver<EventResult>);
impl Default for Source {
    fn default() -> Self {
        Self(mpsc::unbounded_channel().1)
    }
}
impl EventSource for Source {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<EventResult>> {
        self.get_mut().0.poll_recv(cx)
    }
}

#[tokio::test]
async fn color_replies_preserve_interleaved_keys_paste_and_escape_and_skip_unchanged_draws() {
    let initial = crate::terminal_probe::DefaultColors {
        fg: (0, 0, 0),
        bg: (255, 255, 255),
    };
    crate::terminal_palette::update_default_colors(initial);
    let broker = Arc::new(EventBroker::<Source>::new());
    let (tx, rx) = mpsc::unbounded_channel();
    *broker.state.lock().unwrap() = EventBrokerState::Running(Source(rx));
    let (_draw_tx, draw_rx) = broadcast::channel(1);
    let mut stream = TuiEventStream::new(
        broker.clone(),
        draw_rx,
        Arc::new(AtomicBool::new(true)),
        crate::tui::job_control::SuspendContext::new(),
        Arc::new(AtomicBool::new(false)),
    );
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let foreground = EventWithColor::ForegroundColor(Color::Rgb {
        r: 240,
        g: 240,
        b: 240,
    });
    let background = EventWithColor::BackgroundColor(Color::Rgb {
        r: 20,
        g: 20,
        b: 20,
    });
    for event in [
        Event::ColorScheme(crossterm::event::ColorScheme::Dark).into(),
        foreground.clone(),
        Event::Key(key).into(),
        Event::Paste("你好 🦀 café".into()).into(),
        background.clone(),
        Event::Key(escape).into(),
    ] {
        tx.send(Ok(event)).unwrap();
    }
    assert!(matches!(stream.next().await, Some(TuiEvent::Key(actual)) if actual == key));
    assert!(
        matches!(stream.next().await, Some(TuiEvent::Paste(actual)) if actual == "你好 🦀 café")
    );
    assert!(matches!(stream.next().await, Some(TuiEvent::ColorsChanged)));
    assert!(matches!(stream.next().await, Some(TuiEvent::Key(actual)) if actual == escape));
    let revision = crate::render::highlight::syntax_theme_revision();
    broker.refresh_colors(super::super::color_refresh::RefreshReason::Manual);
    for event in [
        foreground.clone(),
        background.clone(),
        Event::Key(key).into(),
    ] {
        tx.send(Ok(event)).unwrap();
    }
    assert!(matches!(stream.next().await, Some(TuiEvent::Key(actual)) if actual == key));
    assert_eq!(crate::render::highlight::syntax_theme_revision(), revision);
    // No active request: even a complete, different pair is ignored.
    for event in [
        EventWithColor::ForegroundColor(Color::Rgb { r: 0, g: 0, b: 0 }),
        background,
        Event::Key(escape).into(),
    ] {
        tx.send(Ok(event)).unwrap();
    }
    assert!(matches!(stream.next().await, Some(TuiEvent::Key(actual)) if actual == escape));
    assert_eq!(crate::render::highlight::syntax_theme_revision(), revision);
    broker.refresh_colors(super::super::color_refresh::RefreshReason::Manual);
    broker.pause_events();
    assert!(matches!(
        *broker.state.lock().unwrap(),
        EventBrokerState::Paused
    ));
    broker.resume_events();
    assert!(matches!(
        *broker.state.lock().unwrap(),
        EventBrokerState::Start
    ));
}
