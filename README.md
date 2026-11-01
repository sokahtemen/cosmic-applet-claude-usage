# cosmic-applet-claude-usage

COSMIC panel applet showing Claude 5-hour and weekly usage. It reads the
Claude Code OAuth credentials from the local machine; no token is stored here.

    cargo build --release
    install -Dm755 target/release/cosmic-applet-claude-usage ~/.local/bin/
    install -Dm644 res/io.github.claude_usage.desktop ~/.local/share/applications/
