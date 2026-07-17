import { useState } from "react";
import { HaloProvider, useHalo } from "./store";
import { Setup } from "./screens/Setup";
import { Sidebar } from "./components/Sidebar";
import { MainPane } from "./components/MainPane";
import { SettingsPanel } from "./screens/Settings";
import "./App.css";

function UpdateBanner() {
  const { update, installUpdate, dismissUpdate, installingUpdate } = useHalo();
  if (!update) return null;
  return (
    <div className="update-banner">
      <span>Halo {update.version} is available.</span>
      <div className="update-actions">
        <button
          className="btn btn-sm btn-primary"
          disabled={installingUpdate}
          onClick={() => void installUpdate()}
        >
          {installingUpdate ? "Installing…" : "Install & restart"}
        </button>
        <button className="btn btn-sm btn-ghost" disabled={installingUpdate} onClick={dismissUpdate}>
          Later
        </button>
      </div>
    </div>
  );
}

function Shell() {
  const { view, error, clearError } = useHalo();
  const [showSettings, setShowSettings] = useState(false);

  if (view === "loading") {
    return (
      <div className="center-screen">
        <div className="spinner" />
        <p>Loading Halo…</p>
      </div>
    );
  }

  if (view === "setup") {
    return <Setup />;
  }

  return (
    <div className="layout">
      <UpdateBanner />
      <Sidebar onOpenSettings={() => setShowSettings(true)} />
      <MainPane />
      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
      {error && (
        <div className="toast toast-error" onClick={clearError}>
          <span>{error}</span>
          <button className="toast-close">Dismiss</button>
        </div>
      )}
    </div>
  );
}

export default function App() {
  return (
    <HaloProvider>
      <Shell />
    </HaloProvider>
  );
}
