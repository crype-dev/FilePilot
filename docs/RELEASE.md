# Windows distribution

`npm run build:windows` runs frontend lint/tests/build, Rust fmt/clippy/tests and Tauri's NSIS build. Result: `src-tauri/target/release/bundle/nsis/FilePilot_0.1.0_x64-setup.exe` (the build prints the actual absolute path). Install is per-user; WebView2 bootstrapper requires internet when the runtime is missing. Application data are under the OS app-data directory for `dev.crype.filepilot` and remain separate from installation files.

Local builds deliberately have no production updater key and `createUpdaterArtifacts: false`. The update-check action explains this. Do not distribute a local build as a signed auto-updating release.

## Owner setup before first signed release

1. Generate a genuine key using `npm run tauri -- signer generate -w <secure-path-outside-repository>`. Keep the private key and its password offline with a backup. Never commit it.
2. Configure the GitHub `release` environment with required reviewers. Store `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as environment secrets. Set environment variable `TAURI_UPDATER_PUBLIC_KEY` to the **contents** of the generated `.pub` file.
3. Ensure repository/tag protection prevents unreviewed changes to the release workflow and signing scripts. The release workflow only accepts tags whose commit belongs to `main`; it never runs on pull requests.
4. Update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` and `CHANGELOG.md` consistently. `node scripts/release.mjs check` validates the three source manifests.
5. Test installation and a two-version signed update in an isolated Windows account/VM. Check corrupt signature refusal and persistence. Tag the validated commit with `v<version>` and push that tag.

The release pipeline fails without signing keys, tests before building, embeds the public key and HTTPS GitHub endpoint, generates `.exe`, `.exe.sig` and `latest.json`, creates a draft, checks all three uploads then publishes. Tauri verifies update signatures and compares SemVer. Installation is explicitly requested and blocked while file operations are active. An exclusive runtime guard prevents new file work during download/installation; persisted preferences remain unchanged.

Tauri updater signatures do not provide Windows Authenticode signing. Without an Authenticode certificate, SmartScreen may display a warning. Key rotation requires a planned transition signed with the existing trusted key; do not silently replace keys.

## Distribution verification

Use an isolated standard Windows user/VM. Install by double-click; check Start menu, launch, native folder picker, onboarding, preview, move, collisions, undo, restart persistence, tray hide/show/pause/quit, optional startup and notifications. Uninstall and confirm organized documents and application data were preserved. Run two signed versions with the same production-compatible test key and verify a modified signature is refused. None of these UI/installer checks may be inferred from a successful compilation.
