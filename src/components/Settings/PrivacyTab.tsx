import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function PrivacyTab() {
  const [excludedApps, setExcludedApps] = useState<string[]>([]);
  const [newApp, setNewApp] = useState("");

  useEffect(() => {
    invoke<Record<string, string>>("get_settings").then((settings) => {
      if (settings.excluded_apps) {
        try {
          setExcludedApps(JSON.parse(settings.excluded_apps));
        } catch {
          setExcludedApps([]);
        }
      }
    });
  }, []);

  const saveExcludedApps = async (apps: string[]) => {
    const prev = excludedApps;
    setExcludedApps(apps);
    try {
      await invoke("update_setting", {
        key: "excluded_apps",
        value: JSON.stringify(apps),
      });
    } catch (e) {
      console.error("Failed to save excluded apps:", e);
      setExcludedApps(prev);
    }
  };

  const handleAdd = () => {
    const trimmed = newApp.trim();
    if (trimmed && !excludedApps.includes(trimmed)) {
      saveExcludedApps([...excludedApps, trimmed]);
      setNewApp("");
    }
  };

  const handleRemove = (app: string) => {
    saveExcludedApps(excludedApps.filter((a) => a !== app));
  };

  return (
    <div className="settings-tab">
      <div className="setting-group">
        <label className="setting-label">Excluded Applications</label>
        <p className="setting-description">
          ClipSlot will not capture clipboard content copied from these apps.
          Enter the application bundle identifier (e.g., com.1password.app).
        </p>

        <div className="setting-row">
          <input
            type="text"
            className="setting-input"
            placeholder="com.example.app"
            value={newApp}
            onChange={(e) => setNewApp(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAdd()}
          />
          <button className="setting-btn" onClick={handleAdd}>
            Add
          </button>
        </div>

        {excludedApps.length > 0 ? (
          <ul className="app-list">
            {excludedApps.map((app) => (
              <li key={app} className="app-list-item">
                <span>{app}</span>
                <button
                  className="remove-btn"
                  onClick={() => handleRemove(app)}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="setting-empty">No excluded apps.</p>
        )}
      </div>
    </div>
  );
}
