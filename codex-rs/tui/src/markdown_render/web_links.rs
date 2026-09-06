//! Presentation policy for web-link destinations in semantic Markdown output.
//!
//! Terminal identity is a conservative heuristic, not end-to-end capability negotiation. Unknown
//! terminals retain the destination so a label never hides the only usable URL. tmux may opt into
//! label-only presentation after a successful, bounded startup capability probe.

use crate::terminal_hyperlinks::web_destination;
use codex_terminal_detection::Multiplexer;
use codex_terminal_detection::TerminalInfo;
use codex_terminal_detection::TerminalName;
use codex_terminal_detection::terminal_info;
use std::io::IsTerminal;
use std::sync::LazyLock;
use std::sync::OnceLock;

#[path = "tmux_hyperlinks.rs"]
mod tmux_hyperlinks;
use tmux_hyperlinks::TmuxHyperlinks;

static TMUX_HYPERLINKS: OnceLock<TmuxHyperlinks> = OnceLock::new();

pub(crate) async fn probe_tmux_hyperlinks() {
    if std::io::stdout().is_terminal()
        && std::env::var_os("STY").is_none()
        && std::env::var_os("ZELLIJ").is_none()
        && std::env::var_os("ZELLIJ_SESSION_NAME").is_none()
        && std::env::var_os("ZELLIJ_VERSION").is_none()
        && matches!(terminal_info().multiplexer, Some(Multiplexer::Tmux { .. }))
    {
        let support = tmux_hyperlinks::probe().await;
        let _ = TMUX_HYPERLINKS.set(support);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WebLinkDisplay {
    LabelOnly,
    WithDestination,
}

impl WebLinkDisplay {
    fn for_terminal(
        terminal: &TerminalInfo,
        term: Option<&str>,
        tmux_hyperlinks: TmuxHyperlinks,
    ) -> Self {
        if term == Some("dumb") {
            return Self::WithDestination;
        }
        if matches!(terminal.multiplexer, Some(Multiplexer::Tmux { .. }))
            && tmux_hyperlinks == TmuxHyperlinks::Supported
        {
            return Self::LabelOnly;
        }
        if terminal.multiplexer.is_some()
            || term.is_some_and(|term| term.starts_with("screen") || term.starts_with("tmux"))
        {
            return Self::WithDestination;
        }
        match terminal.name {
            TerminalName::Ghostty
            | TerminalName::Iterm2
            | TerminalName::WezTerm
            | TerminalName::Kitty
            | TerminalName::VsCode
            | TerminalName::Alacritty
            | TerminalName::WindowsTerminal
            | TerminalName::Konsole
            | TerminalName::GnomeTerminal
            | TerminalName::Vte => Self::LabelOnly,
            TerminalName::AppleTerminal
            | TerminalName::WarpTerminal
            | TerminalName::Dumb
            | TerminalName::Unknown => Self::WithDestination,
        }
    }

    fn hide_destination(self, destination: &str) -> bool {
        self == Self::LabelOnly && web_destination(destination).is_some()
    }
}

pub(crate) fn hide_web_link_destination(destination: &str) -> bool {
    static DISPLAY: LazyLock<WebLinkDisplay> = LazyLock::new(|| {
        if !std::io::stdout().is_terminal() || std::env::var_os("STY").is_some() {
            return WebLinkDisplay::WithDestination;
        }
        WebLinkDisplay::for_terminal(
            &terminal_info(),
            std::env::var("TERM").ok().as_deref(),
            TMUX_HYPERLINKS
                .get()
                .copied()
                .unwrap_or(TmuxHyperlinks::Unknown),
        )
    });
    DISPLAY.hide_destination(destination)
}

#[cfg(test)]
#[path = "web_links_tests.rs"]
mod tests;
