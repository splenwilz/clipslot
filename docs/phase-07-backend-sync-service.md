# Phase 7: Backend Sync Service

## Context

ClipSlot is a Tauri 2 clipboard manager. Phases 0-6 built the complete local experience with encryption. This phase builds the backend sync service that will relay encrypted clipboard data between devices.

## Prerequisites

- Phases 0-6 completed: Local app with encryption
- Note: This phase is backend-only. No client integration yet (that's Phase 8).

## Scope

- Build a stateless sync relay service in Rust (Axum)
- User authentication (device-linked accounts)
- Encrypted blob storage (server cannot read content)
- WebSocket connections for real-time sync
- REST API for initial sync and history fetch
- Database for user accounts and encrypted blobs

## Technical Design

### Tech Stack

- **Framework:** Axum (Rust)
- **Database:** PostgreSQL
- **Real-time:** WebSockets (via Axum)
- **Auth:** JWT tokens + device registration
- **Storage:** Encrypted blobs in PostgreSQL (or S3-compatible for scale)
- **Deployment:** Docker container

### Architecture

```
[Device A] ←──WebSocket──→ [Sync Service] ←──WebSocket──→ [Device B]
                                │
                          [PostgreSQL]
                          (encrypted blobs only)
```

The server is a **dumb relay** — it stores and forwards encrypted blobs. It cannot decrypt content.

### API Endpoints

#### Auth
```
POST /api/auth/register     — Create account (email + password)
POST /api/auth/login        — Get JWT token
POST /api/auth/device       — Register a new device
DELETE /api/auth/device/:id  — Remove a device
GET  /api/auth/devices      — List registered devices
```

#### Sync
```
GET  /api/sync/slots                — Get all encrypted slot blobs
PUT  /api/sync/slots/:number       — Update an encrypted slot blob
GET  /api/sync/history              — Get encrypted history items (paginated)
POST /api/sync/history              — Push new encrypted history items
DELETE /api/sync/history/:id        — Delete a synced history item
```

#### WebSocket
```
WS /api/sync/ws    — Real-time sync channel (authenticated)
```

### Database Schema (PostgreSQL)

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    device_type TEXT NOT NULL,  -- 'macos', 'windows'
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE synced_slots (
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    slot_number INTEGER NOT NULL,
    encrypted_blob BYTEA NOT NULL,  -- Server cannot read this
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    updated_by UUID REFERENCES devices(id),
    PRIMARY KEY (user_id, slot_number)
);

CREATE TABLE synced_history (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    encrypted_blob BYTEA NOT NULL,
    content_hash TEXT NOT NULL,  -- For dedup across devices
    device_id UUID REFERENCES devices(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_synced_history_user ON synced_history(user_id, created_at DESC);
```

### WebSocket Protocol

Messages are JSON:

```json
// Client → Server: Slot update
{
    "type": "slot_update",
    "slot_number": 1,
    "encrypted_blob": "base64...",
    "timestamp": 1234567890
}

// Server → Client: Slot update from another device
{
    "type": "slot_updated",
    "slot_number": 1,
    "encrypted_blob": "base64...",
    "updated_by": "device-uuid",
    "timestamp": 1234567890
}

// Client → Server: New history item
{
    "type": "history_push",
    "id": "uuid",
    "encrypted_blob": "base64...",
    "content_hash": "sha256..."
}

// Server → Client: New history item from another device
{
    "type": "history_new",
    "id": "uuid",
    "encrypted_blob": "base64...",
    "content_hash": "sha256...",
    "device_id": "uuid"
}
```

### Conflict Resolution

- **Slots:** Last-write-wins based on timestamp
- **History:** Merge by content_hash (dedup), order by created_at
- Server stores `updated_by` device ID for auditability

### Rust Dependencies (Backend)

- `axum` — web framework
- `tokio` — async runtime
- `sqlx` — PostgreSQL driver
- `jsonwebtoken` — JWT auth
- `argon2` — password hashing
- `serde` / `serde_json` — serialization
- `uuid` — unique IDs
- `tower-http` — CORS, logging middleware

### Project Structure (Backend)

```
clipslot-server/
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── sync.rs
│   │   └── ws.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── device.rs
│   │   └── sync.rs
│   ├── db/
│   │   ├── mod.rs
│   │   └── migrations/
│   └── middleware/
│       ├── mod.rs
│       └── auth.rs
├── Cargo.toml
├── Dockerfile
└── docker-compose.yml
```

## Acceptance Criteria

- [ ] Server starts and listens on configured port
- [ ] User registration and login work (JWT tokens)
- [ ] Device registration works
- [ ] Encrypted slot blobs can be stored and retrieved
- [ ] Encrypted history items can be pushed and fetched
- [ ] WebSocket connection authenticates and stays alive
- [ ] Slot updates via WebSocket are relayed to other connected devices
- [ ] History pushes via WebSocket are relayed to other connected devices
- [ ] Server cannot decrypt any stored content (verify by inspecting DB)
- [ ] Docker build works

## Manual Test Steps

1. Start the server: `cargo run` (or `docker-compose up`)
2. Register a user: `curl -X POST localhost:3000/api/auth/register -d '{"email":"test@test.com","password":"test123"}'`
3. Login: `curl -X POST localhost:3000/api/auth/login -d '{"email":"test@test.com","password":"test123"}'`
4. Register a device using the JWT token
5. Push an encrypted slot: `PUT /api/sync/slots/1` with encrypted blob
6. Retrieve the slot: `GET /api/sync/slots` — verify the blob matches
7. Connect two WebSocket clients (simulating two devices)
8. Send a slot update from client A — verify client B receives it
9. Send a history push from client B — verify client A receives it
10. Inspect the PostgreSQL database — all `encrypted_blob` columns should be opaque binary data
11. Verify the Docker image builds and runs

## Notes for Future Context

- Phase 8 integrates this service with the Tauri client
- The server is intentionally "dumb" — it cannot read clipboard content
- Rate limiting and abuse prevention should be added before production
- The sync service is stateless per-request (WebSocket connections are stateful)
- For scale: encrypted blobs can be moved to S3/R2, with only metadata in PostgreSQL
