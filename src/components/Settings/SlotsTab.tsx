import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SlotInfo {
  slot_number: number;
  name: string;
  content: string | null;
  content_preview: string | null;
  updated_at: number;
  is_empty: boolean;
}

export default function SlotsTab() {
  const [slots, setSlots] = useState<SlotInfo[]>([]);
  const [editingSlot, setEditingSlot] = useState<number | null>(null);
  const [editName, setEditName] = useState("");

  const loadSlots = async () => {
    try {
      const result = await invoke<SlotInfo[]>("get_all_slots");
      setSlots(result);
    } catch (e) {
      console.error("Failed to load slots:", e);
    }
  };

  useEffect(() => {
    loadSlots();
  }, []);

  const handleRename = async (slotNumber: number) => {
    const trimmed = editName.trim();
    if (!trimmed) return;
    try {
      await invoke("rename_slot", { slotNumber, name: trimmed });
      setEditingSlot(null);
      await loadSlots();
    } catch (e) {
      console.error("Failed to rename slot:", e);
    }
  };

  const handleClear = async (slotNumber: number) => {
    try {
      await invoke("clear_slot", { slotNumber });
      await loadSlots();
    } catch (e) {
      console.error("Failed to clear slot:", e);
    }
  };

  const truncate = (text: string, maxLen: number) => {
    if (text.length <= maxLen) return text;
    return text.substring(0, maxLen) + "...";
  };

  return (
    <div className="settings-tab">
      <div className="setting-group">
        <label className="setting-label">Permanent Slots</label>
        <p className="setting-description">
          Manage your 10 permanent clipboard slots. Keyboard shortcuts cover
          slots 1-5 (Save: Cmd+Ctrl+1-5, Paste: Cmd+Option+1-5). Slots 6-10
          are available via the UI and sync.
        </p>

        <div className="slots-list">
          {slots.map((slot) => (
            <div key={slot.slot_number} className="slot-card">
              <div className="slot-header">
                {editingSlot === slot.slot_number ? (
                  <input
                    type="text"
                    className="slot-name-input"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onBlur={() => handleRename(slot.slot_number)}
                    onKeyDown={(e) =>
                      e.key === "Enter" && handleRename(slot.slot_number)
                    }
                    autoFocus
                  />
                ) : (
                  <span
                    className="slot-name"
                    onClick={() => {
                      setEditingSlot(slot.slot_number);
                      setEditName(slot.name);
                    }}
                    title="Click to rename"
                  >
                    {slot.name}
                  </span>
                )}
                <div className="slot-actions">
                  {!slot.is_empty && (
                    <button
                      className="slot-clear-btn"
                      onClick={() => handleClear(slot.slot_number)}
                    >
                      Clear
                    </button>
                  )}
                </div>
              </div>
              <div className="slot-preview">
                {slot.is_empty
                  ? "(empty)"
                  : truncate(slot.content_preview || "", 80)}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
