import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

const params = new URLSearchParams(window.location.search);
const page = params.get("page");

// Lazy-load settings to avoid bundling it in every window
const SettingsWindow = React.lazy(
  () => import("./components/Settings/SettingsWindow")
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {page === "settings" ? (
      <React.Suspense fallback={<div>Loading...</div>}>
        <SettingsWindow />
      </React.Suspense>
    ) : (
      <App />
    )}
  </React.StrictMode>
);
