import { useState } from "react";
import AccountTab from "./AccountTab";
import GeneralTab from "./GeneralTab";
import PrivacyTab from "./PrivacyTab";
import SlotsTab from "./SlotsTab";
import "./settings.css";

const TABS = [
  { id: "general", label: "General" },
  { id: "privacy", label: "Privacy" },
  { id: "slots", label: "Slots" },
  { id: "account", label: "Account" },
] as const;

type TabId = (typeof TABS)[number]["id"];

export default function SettingsWindow() {
  const [activeTab, setActiveTab] = useState<TabId>("general");

  return (
    <div className="settings-container">
      <div className="settings-header">
        <h2>Settings</h2>
      </div>

      <div className="tab-bar">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            className={`tab-button${activeTab === tab.id ? " active" : ""}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="tab-content">
        {activeTab === "general" && <GeneralTab />}
        {activeTab === "privacy" && <PrivacyTab />}
        {activeTab === "slots" && <SlotsTab />}
        {activeTab === "account" && <AccountTab />}
      </div>
    </div>
  );
}
