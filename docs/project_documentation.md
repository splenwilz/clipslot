# Product Design Document (PRD)

## Product Name (Working)

**ClipSlot**
*Tagline: Your clipboard, everywhere. With memory.*

(Names can change later — this is just a placeholder.)

---

## 1. Problem Statement

Modern users copy and paste constantly across devices, but:

* Clipboard content is **ephemeral**
* Important copied content is **easily lost**
* Existing clipboard managers rely heavily on UI and mouse interaction
* Cross-device clipboard sync is unreliable, opaque, or locked to ecosystems
* There is no fast, keyboard-first way to **promote** clipboard items into permanent, reusable snippets

**Result:** Users repeatedly re-copy the same information, lose critical data, or rely on messy notes apps for something that should be instant.

---

## 2. Product Vision

Create a **keyboard-first, cross-platform clipboard manager** that:

* Automatically remembers everything you copy
* Lets you instantly **promote any clipboard item into a permanent slot**
* Syncs seamlessly across **macOS and Windows**
* Is privacy-first and end-to-end encrypted
* Feels invisible until you need it

---

## 3. Target Users

### Primary

* Software developers
* Designers
* Marketers
* Founders
* Knowledge workers
* Power users who live on the keyboard

### Secondary

* Support agents
* Writers
* Sales teams
* Students

---

## 4. Core Value Proposition

> “Anything you copy is never lost — and the important things are always one shortcut away, on every device.”

---

## 5. Key Use Cases

1. Copy an API key on macOS → paste it later on Windows
2. Save frequently used text (emails, addresses, replies) into permanent slots
3. Promote a copied item instantly without opening an app
4. Switch laptops without losing clipboard history
5. Avoid re-copying the same content multiple times per day

---

## 6. Functional Requirements

### 6.1 Clipboard Capture

* Automatically capture all clipboard events
* Supported types (MVP):

  * Plain text
* Metadata stored:

  * Timestamp
  * Device ID
  * Source app (if available)
  * Content hash
  * Content type

---

### 6.2 Clipboard History

* Local history with configurable limit (e.g. last 500 items)
* Searchable
* Items auto-expire unless promoted

---

### 6.3 Promote to Permanent (Core Feature)

* Keyboard shortcut promotes **current clipboard item** to a permanent slot

Example default shortcuts:

* `Cmd + Shift + 1` / `Ctrl + Shift + 1` → Save to Slot 1
* `Cmd + Shift + 2` → Save to Slot 2
* Up to N configurable slots (MVP: 5)

Permanent slots:

* Persist across app restarts
* Never auto-delete
* Sync across devices
* Can be renamed and reordered

---

### 6.4 Slot Pasting

* Keyboard shortcuts paste directly from slot
* Example:

  * `Cmd + Option + 1` → Paste Slot 1
* Slot content does **not** overwrite clipboard unless pasted

---

### 6.5 Cross-Platform Sync

* Devices supported (MVP):

  * macOS
  * Windows
* Clipboard history sync (opt-in)
* Permanent slots always sync
* Near real-time (<300ms target)

---

### 6.6 Encryption & Privacy

* End-to-end encryption (E2EE)
* Encryption keys never stored on server
* Server cannot read clipboard content
* App exclusion list:

  * Password managers
  * Banking apps
  * Private browser windows
* Manual “Pause Sync” shortcut

---

### 6.7 Offline Support

* Full local functionality offline
* Sync resumes automatically when online
* Conflict resolution rules applied on reconnect

---

## 7. Non-Functional Requirements

### Performance

* Clipboard capture latency: <50ms
* Sync latency target: <300ms
* Zero noticeable UI lag

### Reliability

* Must not crash target applications
* Graceful failure on permission denial

### Security

* AES-256 encryption for stored content
* TLS for transport
* Zero plaintext clipboard data on servers

---

## 8. Conflict Resolution Strategy

If two devices update the same slot:

* Last-write-wins
* Previous versions retained locally
* Optional conflict notification

---

## 9. User Experience Principles

* Keyboard-first
* Minimal UI
* No forced onboarding popups
* App runs quietly in background
* UI only appears when explicitly invoked

---

## 10. MVP Scope

### Included

* Text clipboard capture
* Clipboard history
* 5 permanent slots
* macOS + Windows
* Keyboard shortcuts
* End-to-end encrypted sync
* App exclusion rules

### Excluded (Post-MVP)

* Images
* Rich text
* Teams / sharing
* AI labeling
* Mobile apps

---

## 11. Technical Architecture (High-Level)

### Client

* Native OS clipboard listeners
* Background service / tray app
* Local encrypted storage
* Shortcut handler

### Backend

* Stateless sync service
* Encrypted blob storage
* WebSocket or polling-based updates
* Auth via device-linked account

---

## 12. Monetization Strategy

### Free Tier

* Local clipboard history
* Limited slots (e.g. 2)
* No cross-device sync

### Pro (Subscription)

* Unlimited slots
* Cross-device sync
* Encryption
* Advanced rules

### Team (Future)

* Shared slots
* Workspace separation
* Admin controls

---

## 13. Success Metrics

* Daily active users
* Average clipboard items per user
* Slot usage frequency
* Cross-device sync rate
* Retention (7-day / 30-day)

---

## 14. Risks & Mitigations

| Risk                 | Mitigation                        |
| -------------------- | --------------------------------- |
| Privacy concerns     | Transparent encryption model      |
| OS permission issues | Clear onboarding & fallbacks      |
| Sync lag             | Local-first design                |
| User trust           | No analytics on clipboard content |

---

## 15. Open Questions

* Default number of slots?
* Should history sync be optional or default?
* How aggressive should auto-expiration be?
* Cloud vs optional self-hosting?

