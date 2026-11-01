//! One asynchronous OSC 10/11 transaction at a time, owned by the input broker.
//! Reports have no request IDs: replies received outside a transaction are discarded.
//! A new theme notification during a transaction requests one follow-up transaction.

use crate::terminal_probe::DefaultColors;
use crossterm::event::EventWithColor;
use crossterm::style::Color;
use std::future::Future;
use std::io;
use std::io::Write;
use std::pin::Pin;
use std::task::Context;
use std::time::Duration;
use tokio::time::Instant;
use tokio::time::Sleep;

const TIMEOUT: Duration = Duration::from_secs(/*secs*/ 1);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefreshReason {
    Theme,
    Focus,
    Manual,
}

#[derive(Default)]
pub(super) struct ColorRefresh {
    pending: Option<Pending>,
    last_success: Option<Instant>,
    follow_up: bool,
}

struct Pending {
    foreground: Option<(u8, u8, u8)>,
    background: Option<(u8, u8, u8)>,
    deadline: Pin<Box<Sleep>>,
}

impl ColorRefresh {
    pub(super) fn request(&mut self, reason: RefreshReason, writer: &mut impl Write) {
        if self.pending.is_some() {
            self.follow_up |= reason == RefreshReason::Theme;
            return;
        }
        if reason == RefreshReason::Focus
            && self
                .last_success
                .is_some_and(|last| last.elapsed() < TIMEOUT)
        {
            return;
        }
        if writer
            .write_all(b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\")
            .and_then(|()| writer.flush())
            .is_ok()
        {
            self.pending = Some(Pending {
                foreground: None,
                background: None,
                deadline: Box::pin(tokio::time::sleep(TIMEOUT)),
            });
        }
    }

    pub(super) fn poll_timeout(&mut self, cx: &mut Context<'_>, writer: &mut impl Write) {
        if self
            .pending
            .as_mut()
            .is_some_and(|pending| pending.deadline.as_mut().poll(cx).is_ready())
        {
            self.finish(writer);
        }
    }

    pub(super) fn report(
        &mut self,
        event: &EventWithColor,
        writer: &mut impl Write,
    ) -> Option<DefaultColors> {
        let pending = self.pending.as_mut()?;
        // Check the clock too: a ready input event must not beat an expired deadline.
        if Instant::now() >= pending.deadline.deadline() {
            self.finish(writer);
            return None;
        }
        match event {
            EventWithColor::ForegroundColor(Color::Rgb { r, g, b }) => {
                pending.foreground = Some((*r, *g, *b))
            }
            EventWithColor::BackgroundColor(Color::Rgb { r, g, b }) => {
                pending.background = Some((*r, *g, *b))
            }
            _ => return None,
        }
        let colors = DefaultColors {
            fg: pending.foreground?,
            bg: pending.background?,
        };
        self.last_success = Some(Instant::now());
        self.finish(writer);
        Some(colors)
    }

    fn finish(&mut self, writer: &mut impl Write) {
        self.pending = None;
        if std::mem::take(&mut self.follow_up) {
            self.request(RefreshReason::Theme, writer);
        }
    }
}

pub(super) fn output() -> impl Write {
    #[cfg(not(test))]
    {
        io::stdout()
    }
    #[cfg(test)]
    {
        io::sink()
    }
}

/// Save and restore mode 2031 so terminal handoff preserves the previous owner's mode.
pub(super) fn subscribe(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b[?2031s\x1b[?2031h")?;
    writer.flush()
}

pub(super) fn restore(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b[?2031l\x1b[?2031r")?;
    writer.flush()
}

#[cfg(test)]
#[path = "color_refresh_tests.rs"]
mod tests;
