# cosmic-applet-claude-usage

COSMIC panel applet showing Claude 5-hour and weekly usage. It reads the
Claude Code OAuth credentials from the local machine; no token is stored here.

    cargo build --release
    install -Dm755 target/release/cosmic-applet-claude-usage ~/.local/bin/
    install -Dm644 res/io.github.claude_usage.desktop ~/.local/share/applications/

## AI-generated

This project is vibecoded: all of the code, patches and docs were written by
[Claude Code](https://claude.com/claude-code) using **Claude Opus 5.5**
(`claude-opus-5-5`) at **medium effort**. A human gave the directions and did
the testing, but did not review the code line by line.

## License

MIT, see [LICENSE](LICENSE).
