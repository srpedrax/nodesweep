# Desktop cleanup UX — v2.1.0-alpha.2

## Delivered

- Primary navigation separates Overview, System Cleanup, Developer Cleanup, Storage and Settings.
- Overview contains only aggregate System and Developer recoverable totals.
- System results no longer share the Node.js/Gradle tabs.
- Startup discovery marks every fixed drive for automatic selection; removable and network drives stay manual and optical drives remain ignored.
- System, storage, saved Node.js location and saved Gradle location scans start independently, allowing partial results to render as each task completes.
- The last explicitly added developer location participates in subsequent Quick Scans.
- Release builds use the Windows GUI subsystem and do not allocate a console window.
- Node.js cleanup returns structured per-target failures, skipped bytes and retry/elevation capabilities.
- Windows sharing violations are reported as `FileInUse`; permission denial is only presented as a possible elevation case while the app is not elevated.
- If access is still denied while elevated, the UI explains that an ACL, antivirus or active process may be responsible and does not request elevation again.
- Settings and the header show the current elevation mode. Relaunch uses `ShellExecuteW` with `runas`, so Windows owns the UAC prompt.
- Failed Node.js items stay selected for retry. Successful items are removed from the result list.
- NodeSweep never terminates VS Code or any other user process.

## Scan boundary

The startup Quick Scan intentionally does not recurse through entire fixed disks. System known paths and the previously saved developer location are scanned automatically. A future explicit Deep Scan remains required for whole-drive project discovery.

## Verification

- `cargo clippy --all-targets -- -D warnings`: passed.
- Rust tests: 41 passed, 0 failed.
- Vite production build: passed.
- Tauri/NSIS release build: passed.
- PE subsystem: `2` (`Windows GUI`), verified from the release executable header.
- NSIS silent installation: exit code 0.
- Installed version in Windows registry: `2.1.0-alpha.2`.
- Installed executable launched successfully.
- Production installer SHA-256: `F4A6199226B01F6139520AB92740E6B2E9B15C715280A39B77092B94BA73F01E`.

The installed application was closed after the launch check. No cleanup was executed against user data during installer validation.
