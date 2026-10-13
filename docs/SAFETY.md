# File safety and recovery

FilePilot 0.1.0 never intentionally overwrites an occupied filename. A preview token is single-use, expires after 15 minutes and is invalidated when sources or rules change. File hashes are recalculated at execution. Only registered sources are scanned. Source and destination paths are validated in Rust. Move destinations inside any registered source are rejected.

## Durable operation

SQLite WAL with FULL synchronization records `prepared` and `copying` before file mutation. The source is renamed within its folder to `.filepilot-<operation-id>-source`. FilePilot opens this staged file with Windows sharing that allows readers/deletion but prevents writers. It streams to an exclusively created destination temporary file, flushes it to disk, verifies BLAKE3, then publishes via an atomic no-overwrite hard link. The completed destination is held against Windows writes/deletion while verified and while the staged source is removed. Copy is used even on one volume to keep source/destination independently lockable. Disk space for a second copy is required.

On failure, FilePilot restores the staged file only when the original name is free. Ambiguous cases retain files and become `reconcile`. At restart, unfinished records become `reconcile` and automatic organization enters safe mode. No file is deleted during recovery.

Inspect the original path, destination and `.filepilot-<id>-source` (or `.filepilot-<id>-undo`) in the corresponding source folder. A crash during copying may also leave a `.filepilot-<uuid>.copy` in the destination folder. Preserve these until you have compared the files. Recovery requires human inspection; there is no automatic destructive cleanup.

Undo checks destination size, modification time and BLAKE3, refuses an occupied original path, and uses the same journaled transfer procedure. App-generated destinations and restored sources are excluded from further automation while their size/modification metadata remains unchanged. Manual processing also respects this exclusion.

## Supported scope

Local Windows NTFS paths, regular files and a standard user account. UNC/device paths, alternate data streams, Windows system/program paths, symlinks, junctions, reparse points and offline/cloud placeholders are rejected. Filesystems without hard-link support fail safely with the source retained. Cross-volume moves follow the same streamed-copy algorithm but require separate verification on the destination filesystem.

On Windows, copying uses `CopyFileExW` with no-overwrite flags so NTFS alternate streams, file/security attributes and extended attributes are preserved by the OS. A regression test verifies the `Zone.Identifier` download marker survives both move and undo. BLAKE3 verifies the primary contents; preservation of all possible stream/ACL combinations has not been exhaustively tested. Read-only files are excluded. Concurrent hostile filesystem manipulation, disk/controller faults and changes by another process after an operation completes are outside the integrity guarantee. Network shares and cloud hydration are not supported in V1.

Automatic mode is off by default. Existing files are excluded at activation/startup unless the explicit preference is enabled. New/changed files must have the same size, modification time and BLAKE3 across observations separated by the stabilization interval. `notify` plus periodic reconciliation are used; the process must remain running. Duplicate events are coalesced. One operation runs at a time. Stop waits for the current critical transfer to finish.

History loads up to 10,000 entries in the UI; counters cover the displayed history. Retention is unlimited by default (0 days). An explicit positive retention period enables pruning once per day while the process runs. Manual purge requires paused automation and confirmation. Pruning preserves incomplete/ambiguous operations and the separate suppression table, but removes undo information for deleted records. Recovery acknowledgment marks ambiguous records as manually reviewed without altering files. Keep the database to preserve undo and suppression records; do not delete it while the application runs.
