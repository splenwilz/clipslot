import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
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

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  item: ClipboardItem | null;
}

function App() {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [count, setCount] = useState(0);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    item: null,
  });

  const listRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);

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

  useEffect(() => {
    setSelectedIndex(-1);
  }, [searchQuery]);

  useEffect(() => {
    const unlisten = listen("clipboard-changed", () => {
      loadHistory();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadHistory]);

  // Close context menu on click outside
  useEffect(() => {
    const handleClick = () => setContextMenu((prev) => ({ ...prev, visible: false }));
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, []);

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
      await loadHistory();
    } catch (e) {
      console.error("Failed to delete:", e);
    }
  };

  const handleClear = async () => {
    try {
      await invoke<number>("clear_history");
      await loadHistory();
    } catch (e) {
      console.error("Failed to clear:", e);
    }
  };

  const handleSaveToSlot = async (itemId: string, slotNumber: number) => {
    try {
      await invoke("save_item_to_slot", { itemId, slotNumber });
      setContextMenu((prev) => ({ ...prev, visible: false }));
    } catch (e) {
      console.error("Failed to save to slot:", e);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, item: ClipboardItem) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ visible: true, x: e.clientX, y: e.clientY, item });
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (contextMenu.visible) {
      if (e.key === "Escape") {
        setContextMenu((prev) => ({ ...prev, visible: false }));
      }
      return;
    }

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) => {
          const next = Math.min(prev + 1, items.length - 1);
          itemRefs.current[next]?.scrollIntoView({ block: "nearest" });
          return next;
        });
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => {
          const next = Math.max(prev - 1, 0);
          itemRefs.current[next]?.scrollIntoView({ block: "nearest" });
          return next;
        });
        break;
      case "Enter":
        if (selectedIndex >= 0 && selectedIndex < items.length) {
          handleCopy(items[selectedIndex]);
        }
        break;
      case "Escape":
        getCurrentWebviewWindow().close();
        break;
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
    <div className="history-container" onKeyDown={handleKeyDown} tabIndex={0}>
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

      <div className="history-list" ref={listRef}>
        {items.length === 0 ? (
          <div className="empty-state">
            {searchQuery ? "No matching items" : "No clipboard history yet"}
          </div>
        ) : (
          items.map((item, index) => (
            <div
              key={item.id}
              ref={(el) => { itemRefs.current[index] = el; }}
              className={`history-item${copiedId === item.id ? " copied" : ""}${
                selectedIndex === index ? " selected" : ""
              }`}
              onClick={() => {
                setSelectedIndex(index);
                handleCopy(item);
              }}
              onContextMenu={(e) => handleContextMenu(e, item)}
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

      {/* Context Menu */}
      {contextMenu.visible && contextMenu.item && (
        <div
          className="context-menu"
          style={{ top: contextMenu.y, left: contextMenu.x }}
          onClick={(e) => e.stopPropagation()}
        >
          <div
            className="context-menu-item"
            onClick={() => {
              handleCopy(contextMenu.item!);
              setContextMenu((prev) => ({ ...prev, visible: false }));
            }}
          >
            Copy
          </div>
          <div className="context-menu-separator" />
          {[1, 2, 3, 4, 5].map((n) => (
            <div
              key={n}
              className="context-menu-item"
              onClick={() => handleSaveToSlot(contextMenu.item!.id, n)}
            >
              Save to Slot {n}
            </div>
          ))}
          <div className="context-menu-separator" />
          <div
            className="context-menu-item danger"
            onClick={() => {
              handleDelete(contextMenu.item!.id);
              setContextMenu((prev) => ({ ...prev, visible: false }));
            }}
          >
            Delete
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
