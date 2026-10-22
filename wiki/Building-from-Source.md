# Building from Source

Needs Rust 1.88 or newer (`rustup` stable).

```
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

The binary is `target/release/openflashback`. Copy it next to your data files, or run it with
`--data <dir>`. CI requires `cargo fmt` and `clippy -D warnings` to pass; see
[CI and Releases](CI-and-Releases).
