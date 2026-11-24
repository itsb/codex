//! Preserve explicit selections and live previews while automatic themes follow the terminal.
use super::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub(crate) fn set_automatic_theme() {
    set_syntax_theme(resolve_theme_with_override(None, None));
    AUTOMATIC_THEME.store(true, Ordering::Release);
}

pub(crate) fn refresh_terminal_colors() {
    if AUTOMATIC_THEME.load(Ordering::Acquire) {
        set_automatic_theme();
    } else {
        // Diff, markdown, and active-cell caches also depend on the terminal palette.
        THEME_REVISION.fetch_add(1, Ordering::Release);
    }
}

/// Keeps preview colors stable until confirmation or cancellation, including view teardown.
pub(crate) struct ThemePreview {
    original: Theme,
    automatic: bool,
    finished: AtomicBool,
}

impl ThemePreview {
    pub(crate) fn begin() -> Arc<Self> {
        Arc::new(Self {
            original: current_syntax_theme(),
            automatic: AUTOMATIC_THEME.swap(false, Ordering::AcqRel),
            finished: AtomicBool::new(false),
        })
    }

    pub(crate) fn confirm(&self) {
        self.finished.store(true, Ordering::Release);
    }

    pub(crate) fn cancel(&self) {
        if !self.finished.swap(true, Ordering::AcqRel) {
            if self.automatic {
                set_automatic_theme();
            } else {
                set_syntax_theme(self.original.clone());
            }
        }
    }
}

impl Drop for ThemePreview {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
#[path = "highlight_theme_refresh_tests.rs"]
mod tests;
