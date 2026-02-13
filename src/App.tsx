import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface ClipboardItem {
  id: string;
  content: string;
  content_hash: string;
  content_type: string;
  source_app: string | null;
  device_id: string;
  created_at: number;
  is_promoted: boolean;
}

function App() {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [count, setCount] = useState(0);

  const loadHistory = useCallback(async () => {
    try {
      if (searchQuery.trim()) {
        const results = await invoke<ClipboardItem[]>("search_history", {
          query: searchQuery,
        });
        setItems(results);
      } else {
        const results = await invoke<ClipboardItem[]>(
          "get_clipboard_history",
          { limit: 100, offset: 0 }
        );
        setItems(results);
      }
      const c = await invoke<number>("get_history_count");
      setCount(c);
    } catch (e) {
      console.error("Failed to load history:", e);
    }
  }, [searchQuery]);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  // Listen for new clipboard items and refresh
  useEffect(() => {
    const unlisten = listen("clipboard-changed", () => {
      loadHistory();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadHistory]);

  const handleCopy = async (item: ClipboardItem) => {
    try {
      await invoke("copy_to_clipboard", { text: item.content });
      setCopiedId(item.id);
      setTimeout(() => setCopiedId(null), 1500);
    } catch (e) {
      console.error("Failed to copy:", e);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_history_item", { id });
      loadHistory();
    } catch (e) {
      console.error("Failed to delete:", e);
    }
  };

  const handleClear = async () => {
    try {
      await invoke("clear_history");
      loadHistory();
    } catch (e) {
      console.error("Failed to clear:", e);
    }
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return date.toLocaleDateString();
  };

  const truncate = (text: string, maxLen: number) => {
    if (text.length <= maxLen) return text;
    return text.substring(0, maxLen) + "...";
  };

  return (
    <div className="history-container">
      <div className="history-header">
        <h2>Clipboard History</h2>
        <span className="count">{count} items</span>
      </div>

      <div className="search-bar">
        <input
          type="text"
          placeholder="Search clipboard history..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          autoFocus
        />
      </div>

      <div className="history-list">
        {items.length === 0 ? (
          <div className="empty-state">
            {searchQuery ? "No matching items" : "No clipboard history yet"}
          </div>
        ) : (
          items.map((item) => (
            <div
              key={item.id}
              className={`history-item ${copiedId === item.id ? "copied" : ""}`}
              onClick={() => handleCopy(item)}
            >
              <div className="item-content">
                {truncate(item.content, 120)}
              </div>
              <div className="item-meta">
                <span className="item-time">{formatTime(item.created_at)}</span>
                <span className="item-size">
                  {item.content.length} chars
                </span>
                {copiedId === item.id && (
                  <span className="copied-badge">Copied!</span>
                )}
                <button
                  className="delete-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(item.id);
                  }}
                  title="Delete"
                >
                  x
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {items.length > 0 && (
        <div className="history-footer">
          <button className="clear-btn" onClick={handleClear}>
            Clear History
          </button>
        </div>
      )}
    </div>
  );
}

export default App;
