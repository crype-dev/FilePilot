# Contributing

Use Windows 10/11 x64, Node 24 LTS, Rust stable (at least 1.90), Visual Studio C++ Build Tools and WebView2. Run `npm ci`, `npm run desktop`.

Before submitting: `npm run lint`, `npm test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml --locked`.

Never test file mutations against real Downloads, Desktop or Documents. Preserve backward compatibility and add regression tests for file integrity, collisions and undo. Do not introduce overwrite policies or arbitrary shell execution.
