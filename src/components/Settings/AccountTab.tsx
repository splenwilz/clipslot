import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SyncState {
  status: string;
  logged_in: boolean;
  email: string | null;
  device_id: string | null;
  history_sync_enabled: boolean;
}

interface DeviceInfo {
  id: string;
  name: string;
  device_type: string;
  last_seen: string;
  created_at: string;
}

export default function AccountTab() {
  const [syncState, setSyncState] = useState<SyncState | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isRegister, setIsRegister] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [linkCode, setLinkCode] = useState("");
  const [linkCodeInput, setLinkCodeInput] = useState("");
  const [linkLoading, setLinkLoading] = useState(false);
  const [linkError, setLinkError] = useState("");
  const [linkSuccess, setLinkSuccess] = useState("");

  const loadStatus = async () => {
    try {
      const state = await invoke<SyncState>("get_sync_status");
      setSyncState(state);
      if (state.logged_in) {
        loadDevices();
      }
    } catch (e) {
      console.error("Failed to load sync status:", e);
    }
  };

  const loadDevices = async () => {
    try {
      const result = await invoke<DeviceInfo[]>("get_linked_devices");
      setDevices(result);
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  };

  useEffect(() => {
    loadStatus();
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);

    try {
      const command = isRegister ? "sync_register" : "sync_login";
      const state = await invoke<SyncState>(command, { email, password });
      setSyncState(state);
      setEmail("");
      setPassword("");
      loadDevices();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("sync_logout");
      setSyncState(null);
      setDevices([]);
      loadStatus();
    } catch (e) {
      console.error("Failed to logout:", e);
    }
  };

  if (!syncState || !syncState.logged_in) {
    return (
      <div className="settings-tab">
        <div className="setting-group">
          <label className="setting-label">
            {isRegister ? "Create Account" : "Sign In"}
          </label>
          <p className="setting-description">
            Sign in to sync your clipboard across devices.
          </p>

          <form className="auth-form" onSubmit={handleSubmit}>
            <input
              type="email"
              className="setting-input auth-input"
              placeholder="Email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
            <input
              type="password"
              className="setting-input auth-input"
              placeholder="Password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={8}
            />

            {error && <p className="auth-error">{error}</p>}

            <button
              type="submit"
              className="setting-btn auth-btn"
              disabled={loading}
            >
              {loading
                ? "Please wait..."
                : isRegister
                  ? "Create Account"
                  : "Sign In"}
            </button>
          </form>

          <p className="setting-description" style={{ marginTop: 12 }}>
            <span
              className="auth-toggle"
              onClick={() => {
                setIsRegister(!isRegister);
                setError("");
              }}
            >
              {isRegister
                ? "Already have an account? Sign in"
                : "Don't have an account? Create one"}
            </span>
          </p>
        </div>
      </div>
    );
  }

  const statusColor =
    syncState.status === "Connected"
      ? "#4caf50"
      : syncState.status === "Syncing" || syncState.status === "Connecting"
        ? "#ff9800"
        : "#999";

  return (
    <div className="settings-tab">
      <div className="setting-group">
        <label className="setting-label">Account</label>
        <p className="setting-description">
          Signed in as <strong>{syncState.email}</strong>
        </p>
        <div className="sync-status-row">
          <span className="sync-dot" style={{ background: statusColor }} />
          <span className="setting-hint">{syncState.status}</span>
          <button
            className="setting-btn"
            style={{ marginLeft: "auto", fontSize: 12 }}
            onClick={async () => {
              try {
                const msg = await invoke<string>("force_sync");
                console.log("Sync result:", msg);
                loadStatus();
              } catch (err) {
                console.error("Sync failed:", err);
              }
            }}
          >
            Force Sync
          </button>
        </div>
        <div style={{ marginTop: 8 }}>
          <button className="setting-btn" onClick={handleLogout}>
            Sign Out
          </button>
        </div>
      </div>

      <div className="setting-group">
        <label className="setting-label">History Sync</label>
        <p className="setting-description">
          When enabled, clipboard history is synced across your devices.
        </p>
        <div className="setting-row">
          <label className="toggle-label">
            <input
              type="checkbox"
              checked={syncState.history_sync_enabled}
              onChange={async (e) => {
                const enabled = e.target.checked;
                try {
                  await invoke("toggle_history_sync", { enabled });
                  setSyncState((prev) =>
                    prev ? { ...prev, history_sync_enabled: enabled } : prev
                  );
                } catch (err) {
                  console.error("Failed to toggle history sync:", err);
                }
              }}
            />
            Enable history sync
          </label>
        </div>
      </div>

      <div className="setting-group">
        <label className="setting-label">Linked Devices</label>
        {devices.length === 0 ? (
          <p className="setting-empty">No devices linked yet.</p>
        ) : (
          <ul className="app-list">
            {devices.map((device) => (
              <li key={device.id} className="app-list-item">
                <span>
                  {device.name}{" "}
                  <span className="setting-hint">({device.device_type})</span>
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="setting-group">
        <label className="setting-label">Link Device</label>
        <p className="setting-description">
          Share your encryption key with another device using a 6-digit code.
        </p>

        <div style={{ marginTop: 8 }}>
          <button
            className="setting-btn"
            disabled={linkLoading}
            onClick={async () => {
              setLinkLoading(true);
              setLinkError("");
              setLinkCode("");
              try {
                const code = await invoke<string>("generate_link_code");
                setLinkCode(code);
              } catch (err) {
                setLinkError(String(err));
              } finally {
                setLinkLoading(false);
              }
            }}
          >
            {linkLoading ? "Generating..." : "Generate Link Code"}
          </button>

          {linkCode && (
            <div className="link-code-display">
              <span className="link-code">{linkCode}</span>
              <p className="setting-hint">
                Enter this code on your other device within 5 minutes.
              </p>
            </div>
          )}
        </div>

        <div style={{ marginTop: 16 }}>
          <p className="setting-description">
            Or enter a code from another device:
          </p>
          <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
            <input
              type="text"
              className="setting-input auth-input"
              placeholder="000000"
              value={linkCodeInput}
              onChange={(e) => setLinkCodeInput(e.target.value)}
              maxLength={6}
              style={{ width: 100 }}
            />
            <button
              className="setting-btn"
              disabled={linkLoading || linkCodeInput.length !== 6}
              onClick={async () => {
                setLinkLoading(true);
                setLinkError("");
                setLinkSuccess("");
                try {
                  await invoke("enter_link_code", { code: linkCodeInput });
                  setLinkSuccess(
                    "Key imported successfully. Please restart ClipSlot."
                  );
                  setLinkCodeInput("");
                } catch (err) {
                  setLinkError(String(err));
                } finally {
                  setLinkLoading(false);
                }
              }}
            >
              {linkLoading ? "Linking..." : "Link"}
            </button>
          </div>
        </div>

        {linkError && <p className="auth-error">{linkError}</p>}
        {linkSuccess && (
          <p className="setting-description" style={{ color: "#4caf50", marginTop: 8 }}>
            {linkSuccess}
          </p>
        )}
      </div>
    </div>
  );
}
