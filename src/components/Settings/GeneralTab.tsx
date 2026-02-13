import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function GeneralTab() {
  const [historyLimit, setHistoryLimit] = useState(500);
  const [autoClearOnQuit, setAutoClearOnQuit] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<Record<string, string>>("get_settings").then((settings) => {
      if (settings.history_limit) {
        setHistoryLimit(parseInt(settings.history_limit, 10) || 500);
      }
      if (settings.auto_clear_on_quit) {
        setAutoClearOnQuit(settings.auto_clear_on_quit === "true");
      }
    });
  }, []);

  const saveSetting = async (key: string, value: string) => {
    try {
      await invoke("update_setting", { key, value });
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
    } catch (e) {
      console.error("Failed to save setting:", e);
    }
  };

  return (
    <div className="settings-tab">
      <div className="setting-group">
        <label className="setting-label">History Limit</label>
        <p className="setting-description">
          Maximum number of clipboard items to keep in history.
        </p>
        <div className="setting-row">
          <input
            type="number"
            className="setting-input number-input"
            value={historyLimit}
            min={10}
            max={10000}
            onChange={(e) => setHistoryLimit(parseInt(e.target.value, 10) || 500)}
            onBlur={() => saveSetting("history_limit", historyLimit.toString())}
          />
          <span className="setting-hint">items</span>
        </div>
      </div>

      <div className="setting-group">
        <label className="setting-label">
          <input
            type="checkbox"
            checked={autoClearOnQuit}
            onChange={(e) => {
              setAutoClearOnQuit(e.target.checked);
              saveSetting("auto_clear_on_quit", e.target.checked.toString());
            }}
          />
          Clear history on quit
        </label>
        <p className="setting-description">
          Automatically delete all clipboard history when ClipSlot exits.
        </p>
      </div>

      {saved && <div className="save-indicator">Settings saved</div>}
    </div>
  );
}
