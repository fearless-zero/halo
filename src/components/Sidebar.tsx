import { useMemo, useState } from "react";
import { useHalo } from "../store";
import { PlusIcon, SearchIcon, SettingsIcon, SparkleIcon } from "./icons";

function relativeDate(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const sameDay = d.toDateString() === now.toDateString();
  if (sameDay) return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}

export function Sidebar({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { notes, currentNote, openNote, startNewRecording, recording } = useHalo();
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return notes;
    return notes.filter(
      (n) => n.title.toLowerCase().includes(q) || n.preview.toLowerCase().includes(q),
    );
  }, [notes, query]);

  const busy = recording.status !== "idle";

  return (
    <aside className="sidebar">
      <div className="sidebar-head">
        <div className="sidebar-brand">
          <SparkleIcon width={18} height={18} />
          <span>Halo</span>
        </div>
        <button className="icon-btn" onClick={onOpenSettings} title="Settings">
          <SettingsIcon width={18} height={18} />
        </button>
      </div>

      <button
        className="btn btn-primary btn-block"
        onClick={() => void startNewRecording()}
        disabled={busy}
      >
        <PlusIcon width={16} height={16} /> New recording
      </button>

      <div className="search-box">
        <SearchIcon width={16} height={16} />
        <input
          placeholder="Search notes"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      <div className="note-list">
        {filtered.length === 0 && <p className="muted empty-hint">No notes yet.</p>}
        {filtered.map((n) => (
          <button
            key={n.id}
            className={`note-item ${currentNote?.id === n.id ? "active" : ""}`}
            onClick={() => void openNote(n.id)}
          >
            <div className="note-item-title">{n.title || "Untitled"}</div>
            <div className="note-item-sub">
              <span>{relativeDate(n.createdAt)}</span>
              {n.durationSecs > 0 && <span> · {Math.round(n.durationSecs / 60)}m</span>}
            </div>
            {n.preview && <div className="note-item-preview">{n.preview}</div>}
          </button>
        ))}
      </div>
    </aside>
  );
}
