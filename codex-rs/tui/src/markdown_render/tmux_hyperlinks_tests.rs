use super::TmuxHyperlinks;
use super::parse_clients;
use pretty_assertions::assert_eq;

#[test]
fn every_attached_client_must_report_hyperlinks() {
    for (output, expected) in [
        (
            "xterm-ghostty\tRGB,hyperlinks,clipboard\n",
            TmuxHyperlinks::Supported,
        ),
        (
            "xterm-ghostty\thyperlinks\nxterm-kitty\tRGB,hyperlinks\n",
            TmuxHyperlinks::Supported,
        ),
        (
            "xterm-ghostty\thyperlinks\nxterm-256color\tRGB\n",
            TmuxHyperlinks::Unknown,
        ),
        ("tmux-256color\thyperlinks\n", TmuxHyperlinks::Unknown),
        ("screen-256color\thyperlinks\n", TmuxHyperlinks::Unknown),
        ("dumb\thyperlinks\n", TmuxHyperlinks::Unknown),
        ("xterm-ghostty\tnohyperlinks\n", TmuxHyperlinks::Unknown),
        ("xterm-ghostty\t\n", TmuxHyperlinks::Unknown),
        ("\thyperlinks\n", TmuxHyperlinks::Unknown),
        ("unknown-format\n", TmuxHyperlinks::Unknown),
        ("", TmuxHyperlinks::Unknown),
    ] {
        assert_eq!(parse_clients(output), expected, "output: {output:?}");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn probe_falls_back_on_failure_timeout_or_excessive_output() {
    use tokio::process::Command;

    for (script, expected) in [
        (
            "printf 'xterm-ghostty\thyperlinks\n'",
            TmuxHyperlinks::Supported,
        ),
        (
            "printf 'xterm-ghostty\thyperlinks\n'; exit 1",
            TmuxHyperlinks::Unknown,
        ),
        ("exec /bin/sleep 5", TmuxHyperlinks::Unknown),
        (
            "while :; do printf 'xterm-ghostty\thyperlinks\n'; done",
            TmuxHyperlinks::Unknown,
        ),
        ("printf '\\377'", TmuxHyperlinks::Unknown),
    ] {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        assert_eq!(
            super::query(&mut command).await,
            expected,
            "script: {script}"
        );
    }
    assert_eq!(
        super::query(&mut Command::new("/nonexistent/codex-tmux-test")).await,
        TmuxHyperlinks::Unknown,
    );
}
