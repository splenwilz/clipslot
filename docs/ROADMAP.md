# ClipSlot — Build Roadmap

## Stack

- **Client:** Tauri 2 (Rust backend + React/TypeScript frontend)
- **Backend:** Rust (Axum) sync relay service
- **Database:** SQLite (local), PostgreSQL (server)
- **Encryption:** AES-256-GCM, Argon2id key derivation

## Phases

Each phase is self-contained with its own documentation, acceptance criteria, and manual test steps. Complete and test each phase before moving to the next.

| Phase | Name | Description | Dependencies |
|---|---|---|---|
| [Phase 0](phase-00-project-setup.md) | Project Setup | Tauri 2 project, tray app skeleton | None |
| [Phase 1](phase-01-clipboard-monitoring.md) | Clipboard Monitoring | Detect and capture clipboard changes | Phase 0 |
| [Phase 2](phase-02-local-storage.md) | Local Storage | SQLite persistence, history, search | Phase 1 |
| [Phase 3](phase-03-permanent-slots.md) | Permanent Slots | Save to slots via keyboard shortcuts | Phase 2 |
| [Phase 4](phase-04-slot-pasting.md) | Slot Pasting | Paste from slots via keyboard shortcuts | Phase 3 |
| [Phase 5](phase-05-tray-ui.md) | Tray UI & Settings | History window, settings, shortcut config | Phase 4 |
| [Phase 6](phase-06-encryption.md) | Encryption | E2EE for all stored data | Phase 5 |
| [Phase 7](phase-07-backend-sync-service.md) | Backend Sync Service | Axum server, auth, WebSocket relay | Phase 6 |
| [Phase 8](phase-08-cross-device-sync.md) | Cross-Device Sync | Client sync integration, key exchange | Phase 7 |
| [Phase 9](phase-09-app-exclusion.md) | App Exclusion | Privacy rules, private mode | Phase 8 |
| [Phase 10](phase-10-polish-packaging.md) | Polish & Packaging | Installer, auto-update, onboarding | Phase 9 |

## How to Use These Docs

1. Open the phase doc you're working on
2. Read the **Context** section — it gives you full background even without prior conversation
3. Check **Prerequisites** — make sure previous phases are done
4. Follow the **Technical Design** for implementation guidance
5. Verify all **Acceptance Criteria** are met
6. Run through the **Manual Test Steps** and provide feedback
7. Move to the next phase

## MVP = Phases 0-6

Phases 0-6 deliver a fully functional **local clipboard manager** with encryption. This is usable and shippable without sync.

## Full Product = Phases 0-10

Phases 7-10 add cross-device sync, privacy controls, and production polish.
