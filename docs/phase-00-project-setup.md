# Phase 0: Project Setup & Architecture

## Context

ClipSlot is a keyboard-first, cross-platform clipboard manager built with:
- **Client:** Tauri 2 (Rust backend + React/TypeScript frontend)
- **Backend (later):** Rust (Axum) for sync service
- **Storage:** SQLite (local), encrypted blob store (cloud - later)
- **Sync:** WebSockets (later)

This phase sets up the project skeleton so all future phases have a working foundation.

## Scope

- Initialize a Tauri 2 project with React + TypeScript frontend
- Configure the project structure for scalability
- Set up Rust dependencies for core logic
- Configure system tray (empty, placeholder)
- Ensure the app builds and runs on macOS (and Windows if available)

## Project Structure

```
clipslot/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── main.rs      # Tauri entry point
│   │   ├── lib.rs       # Library root
│   │   ├── clipboard/   # Clipboard monitoring (Phase 1)
│   │   ├── storage/     # Local storage (Phase 2)
│   │   ├── slots/       # Permanent slots (Phase 3)
│   │   ├── crypto/      # Encryption (Phase 6)
│   │   └── sync/        # Sync client (Phase 8)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                 # React/TypeScript frontend
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   └── hooks/
├── docs/                # Phase documentation
├── package.json
└── tsconfig.json
```

## Dependencies (Rust - Cargo.toml)

Core dependencies to add:
- `tauri` v2 — framework
- `tauri-plugin-global-shortcut` — keyboard shortcuts
- `tauri-plugin-clipboard-manager` — clipboard access
- `serde` / `serde_json` — serialization
- `rusqlite` with `bundled` feature — local database
- `chrono` — timestamps
- `uuid` — unique IDs

## Dependencies (Frontend - package.json)

- `react` + `react-dom`
- `@tauri-apps/api` v2
- `@tauri-apps/plugin-global-shortcut`
- `@tauri-apps/plugin-clipboard-manager`
- `typescript`
- `vite`

## Tauri Configuration

Key settings in `tauri.conf.json`:
- `"withGlobalTauri": true` — expose Tauri API to frontend
- System tray enabled
- No visible window on launch (background app)
- App identifier: `com.clipslot.app`

## Acceptance Criteria

- [ ] `cargo tauri dev` starts the app without errors
- [ ] System tray icon appears in macOS menu bar
- [ ] No visible window opens on launch (tray-only app)
- [ ] Clicking the tray icon shows a placeholder menu
- [ ] The app can be quit from the tray menu
- [ ] Project structure matches the layout above

## Manual Test Steps

1. Run `cargo tauri dev` from the project root
2. Verify no window appears
3. Check the macOS menu bar — a tray icon should be visible
4. Click the tray icon — a menu with "ClipSlot" title and "Quit" option should appear
5. Click "Quit" — the app should exit cleanly
6. Run `cargo tauri build` — verify it compiles to a `.dmg` / `.app`

## Notes for Future Context

- This phase does NOT include clipboard monitoring, storage, or shortcuts
- The tray menu is a placeholder — Phase 5 will build the real UI
- The Rust module folders (`clipboard/`, `storage/`, etc.) will be empty stubs with `mod.rs` files
