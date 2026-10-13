# Security policy

Report vulnerabilities privately through GitHub's private vulnerability reporting, if enabled, or contact the repository owner before public disclosure. Do not attach personal documents, credentials or unredacted filesystem paths.

File integrity and prevention of unintended file movement are primary security properties. Every IPC input and imported rule is untrusted. File mutations happen only in Rust. No overwrite, arbitrary shell execution, cloud upload or telemetry is supported.

0.1.0 is an initial test release. Only the latest version receives fixes. See `docs/SAFETY.md` for filesystem assumptions and recovery. Updater signatures are mandatory in signed distribution builds. Local installers have no production updater configured.
