import { useEffect, useMemo, useState } from "react";
import { useHalo } from "../store";
import { api } from "../ipc";
import { renderMarkdown } from "../markdown";
import type { ExportTarget } from "../types";
import { CopyIcon, DownloadIcon, SearchIcon, SparkleIcon, TrashIcon } from "./icons";

export function NoteDetail({ streaming = false }: { streaming?: boolean }) {
  const {
    currentNote,
    styles,
    settings,
    streamBuffer,
    regenerate,
    researchCurrentNote,
    updateNoteContent,
    updateNoteTitle,
    persistCurrentNote,
    deleteNote,
  } = useHalo();

  const [editing, setEditing] = useState(false);
  const [styleId, setStyleId] = useState<string>("");
  const [showTranscript, setShowTranscript] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);

  useEffect(() => {
    if (currentNote) setStyleId(currentNote.styleId);
  }, [currentNote?.id]);

  const content = streaming ? streamBuffer : currentNote?.content ?? "";
  const rendered = useMemo(() => renderMarkdown(content), [content]);

  if (!currentNote) return null;

  const notify = (msg: string) => {
    setFlash(msg);
    setTimeout(() => setFlash(null), 2500);
  };

  const doExport = async (target: ExportTarget) => {
    const res = await api.exportNote(currentNote.id, target);
    notify(res.ok ? res.message : `Export failed: ${res.message}`);
  };

  const exportOptions: Array<{ id: string; label: string; target: ExportTarget }> = [
    { id: "obsidian", label: "Obsidian", target: { kind: "obsidian" } },
    { id: "notion", label: "Notion", target: { kind: "notion" } },
    { id: "slack", label: "Slack", target: { kind: "slack" } },
    { id: "webhook", label: "Webhook", target: { kind: "webhook" } },
  ];
  const extraExports = exportOptions.filter((e) =>
    settings?.integrations.some((c) => c.id === e.id && c.enabled),
  );

  return (
    <div className="note-detail">
      <div className="note-header">
        <input
          className="note-title-input"
          value={currentNote.title}
          placeholder="Untitled note"
          onChange={(e) => updateNoteTitle(e.target.value)}
          onBlur={() => void persistCurrentNote()}
        />
        <div className="note-header-actions">
          <button className="icon-btn" title="Delete" onClick={() => void deleteNote(currentNote.id)}>
            <TrashIcon width={18} height={18} />
          </button>
        </div>
      </div>

      <div className="note-sub muted">
        {new Date(currentNote.createdAt).toLocaleString()}
        {currentNote.durationSecs > 0 && ` · ${Math.round(currentNote.durationSecs / 60)} min`}
      </div>

      <div className="note-toolbar">
        <div className="style-picker">
          <SparkleIcon width={16} height={16} />
          <select value={styleId} onChange={(e) => setStyleId(e.target.value)} disabled={streaming}>
            {styles.map((s) => (
              <option key={s.id} value={s.id}>{s.name}</option>
            ))}
          </select>
          <button
            className="btn btn-sm"
            disabled={streaming}
            onClick={() => void regenerate(styleId)}
          >
            Regenerate
          </button>
          {settings?.webResearch && (
            <button
              className="btn btn-sm"
              disabled={streaming}
              title="Research this note's topics online"
              onClick={() => void researchCurrentNote()}
            >
              <SearchIcon width={15} height={15} /> Research
            </button>
          )}
        </div>
        <div className="note-export">
          <button className="btn btn-sm" onClick={() => void doExport({ kind: "clipboard", format: "markdown" })}>
            <CopyIcon width={15} height={15} /> Copy
          </button>
          <button className="btn btn-sm" onClick={() => void doExport({ kind: "markdown" })}>
            <DownloadIcon width={15} height={15} /> Export .md
          </button>
          {extraExports.map((e) => (
            <button key={e.id} className="btn btn-sm" onClick={() => void doExport(e.target)}>
              {e.label}
            </button>
          ))}
          {!streaming && (
            <button className="btn btn-sm" onClick={() => setEditing((v) => !v)}>
              {editing ? "Preview" : "Edit"}
            </button>
          )}
        </div>
      </div>

      {flash && <div className="flash">{flash}</div>}

      {editing && !streaming ? (
        <textarea
          className="note-editor"
          value={content}
          onChange={(e) => updateNoteContent(e.target.value)}
          onBlur={() => void persistCurrentNote()}
        />
      ) : (
        <div className="note-content">
          <div className="markdown" dangerouslySetInnerHTML={{ __html: rendered }} />
          {streaming && <span className="cursor-blink">▍</span>}
        </div>
      )}

      {!streaming && currentNote.research.length > 0 && (
        <div className="sources-block">
          <h3 className="sources-head">
            <SearchIcon width={15} height={15} /> Sources
          </h3>
          <ul className="sources-list">
            {currentNote.research.map((r, i) => (
              <li key={i}>
                {r.url ? (
                  <a href={r.url} target="_blank" rel="noreferrer noopener">
                    {r.title}
                  </a>
                ) : (
                  <span>{r.title}</span>
                )}
                <span className="muted"> · {r.source}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {currentNote.transcript && currentNote.transcript.text && (
        <div className="transcript-block">
          <button className="transcript-toggle" onClick={() => setShowTranscript((v) => !v)}>
            {showTranscript ? "Hide" : "Show"} transcript
          </button>
          {showTranscript && (
            <div className="transcript-text">
              {currentNote.transcript.segments.length > 0
                ? currentNote.transcript.segments.map((s, i) => (
                    <p key={i}>
                      <span className="ts">{formatTs(s.start)}</span> {s.text}
                    </p>
                  ))
                : <p>{currentNote.transcript.text}</p>}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function formatTs(sec: number): string {
  const m = Math.floor(sec / 60).toString().padStart(2, "0");
  const s = Math.floor(sec % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}
