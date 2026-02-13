# Phase 10: Polish, Packaging & Distribution

## Context

ClipSlot is a Tauri 2 clipboard manager. Phases 0-9 built all core features. This phase focuses on production readiness: polish, packaging, auto-updates, and distribution.

## Prerequisites

- Phases 0-9 completed: All features functional

## Scope

- App icon and branding
- Auto-start on login
- Auto-update mechanism
- Installer packaging (macOS .dmg, Windows .msi/.exe)
- Code signing
- Onboarding flow (first launch)
- Error handling and crash reporting
- Performance optimization
- Accessibility permissions guidance (macOS)

## Technical Design

### App Branding

- App icon (1024x1024 source, all required sizes generated)
- Tray icon (template image for macOS menu bar: 22x22 @1x, 44x44 @2x)
- App name: "ClipSlot"
- Bundle ID: `com.clipslot.app`
- Window titles, about dialog

### Auto-Start on Login

**macOS:**
- Use `SMAppService` (modern) or Login Items via Tauri plugin
- Add to System Settings → General → Login Items

**Windows:**
- Registry key: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- Or Task Scheduler for more reliability

### Auto-Update

- Use `tauri-plugin-updater`
- Update server: GitHub Releases or custom endpoint
- Check for updates on launch + every 6 hours
- Notify user of available updates (don't auto-install)
- Background download, install on next restart

### First Launch Onboarding

Simple 3-step flow (one-time):

1. **Welcome** — "ClipSlot runs in your menu bar. Everything you copy is remembered."
2. **Permissions** — Request accessibility permission (macOS). Explain why it's needed (keyboard shortcuts, paste simulation).
3. **Shortcuts** — Show the default shortcuts. Option to customize.

No forced account creation. Sync is opt-in later.

### Installer Packaging

**macOS:**
- `.dmg` with drag-to-Applications installer
- Code-signed with Apple Developer ID
- Notarized with Apple

**Windows:**
- `.msi` or NSIS `.exe` installer
- Code-signed with EV certificate
- Include uninstaller

Use `cargo tauri build` with appropriate config.

### Error Handling

- Wrap all Tauri commands with proper error types
- Log errors to a local log file (`~/Library/Logs/ClipSlot/` on macOS)
- Optional: crash reporting via Sentry (opt-in, no clipboard content sent)
- Graceful degradation: if sync fails, local mode continues working

### Performance Optimization

- Audit clipboard polling — ensure <2% CPU idle
- Lazy-load history window (virtual scrolling for large lists)
- Database vacuuming on startup (if needed)
- Minimize memory footprint (<50MB RSS)

### Tauri Config Updates

```json
{
    "productName": "ClipSlot",
    "version": "1.0.0",
    "identifier": "com.clipslot.app",
    "bundle": {
        "icon": ["icons/icon.icns", "icons/icon.ico", "icons/icon.png"],
        "macOS": {
            "minimumSystemVersion": "12.0"
        },
        "windows": {
            "certificateThumbprint": "...",
            "timestampUrl": "..."
        }
    },
    "plugins": {
        "updater": {
            "endpoints": ["https://releases.clipslot.com/{{target}}/{{arch}}/{{current_version}}"],
            "pubkey": "..."
        }
    }
}
```

## Acceptance Criteria

- [ ] App has a proper icon in the dock, tray, and installer
- [ ] Auto-start on login works (macOS and Windows)
- [ ] First-launch onboarding guides user through permissions and shortcuts
- [ ] Auto-update checks for new versions and notifies user
- [ ] macOS .dmg installer works (drag to Applications)
- [ ] Windows installer works (.msi or .exe)
- [ ] App is code-signed (no security warnings on install)
- [ ] macOS app is notarized
- [ ] Errors are logged to a file (not swallowed silently)
- [ ] App uses <50MB RAM and <2% CPU when idle
- [ ] Uninstaller cleanly removes the app and data (optional: keep data)
- [ ] About dialog shows version, build info

## Manual Test Steps

1. Build release: `cargo tauri build`
2. Install from the generated .dmg (macOS) or .msi (Windows)
3. Launch — onboarding should appear on first run
4. Grant accessibility permissions when prompted
5. Complete onboarding — app should minimize to tray
6. Verify auto-start: restart computer → ClipSlot should start automatically
7. Verify tray icon looks correct in both light and dark menu bar
8. Copy text → verify capture works
9. Test all shortcuts
10. Open About → verify version info
11. Check Activity Monitor: RAM <50MB, CPU <2% idle
12. Force quit and relaunch — verify clean restart, data intact
13. Check for updates (if update server configured)
14. Uninstall and verify clean removal

## Notes for Future Context

- This is the final MVP phase
- Post-MVP features (images, rich text, teams, AI labeling, mobile) are separate initiatives
- Consider a beta testing period before public release
- App Store distribution (macOS) requires additional sandboxing work
- The free tier (2 slots, no sync) vs Pro tier (5 slots, sync) should be enforced at this stage
