import { useEffect, useState } from "react";
import { useHalo } from "../store";
import { NoteDetail } from "./NoteDetail";
import { ImportIcon, MicIcon, StopIcon } from "./icons";

function LevelMeter({ rms, peak }: { rms: number; peak: number }) {
  const bars = 24;
  const active = Math.round(rms * bars);
  return (
    <div className="level-meter" aria-label="input level">
      {Array.from({ length: bars }, (_, i) => (
        <span
          key={i}
          className={`level-bar ${i < active ? "on" : ""} ${i >= bars - 3 ? "hot" : ""}`}
          style={{ opacity: i < active ? 1 : 0.18 + peak * 0.2 }}
        />
      ))}
    </div>
  );
}

function elapsedLabel(ms: number): string {
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60).toString().padStart(2, "0");
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function RecordingActive({ startedAt }: { startedAt: number }) {
  const { level, stopRecording, cancelRecording } = useHalo();
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(t);
  }, []);

  return (
    <div className="record-active">
      <div className="pulse-ring">
        <div className="pulse-dot" />
      </div>
      <div className="rec-time">{elapsedLabel(now - startedAt)}</div>
      <LevelMeter rms={level.rms} peak={level.peak} />
      <p className="muted">Listening — capturing your mic and system audio…</p>
      <div className="rec-actions">
        <button className="btn btn-danger btn-lg" onClick={() => void stopRecording()}>
          <StopIcon width={18} height={18} /> Stop & write notes
        </button>
        <button className="btn btn-ghost" onClick={() => void cancelRecording()}>
          Discard
        </button>
      </div>
    </div>
  );
}

function Processing({ label, percent }: { label: string; percent?: number }) {
  return (
    <div className="processing">
      <div className="spinner" />
      <p>{label}</p>
      {percent !== undefined && (
        <div className="progress wide">
          <div className="progress-bar" style={{ width: `${percent}%` }} />
        </div>
      )}
    </div>
  );
}

function EmptyState() {
  const { startNewRecording, importRecordings } = useHalo();
  return (
    <div className="empty-state">
      <div className="empty-icon">
        <MicIcon width={40} height={40} />
      </div>
      <h2>Ready when you are</h2>
      <p className="muted">
        Start a recording before your meeting, lecture or call — or import existing
        recordings (from a voice recorder, USB or SD card). Halo transcribes locally,
        writes your notes, and researches the topics for you.
      </p>
      <div className="empty-actions">
        <button className="btn btn-primary btn-lg" onClick={() => void startNewRecording()}>
          <MicIcon width={18} height={18} /> Start recording
        </button>
        <button className="btn btn-lg" onClick={() => void importRecordings()}>
          <ImportIcon width={18} height={18} /> Import recordings
        </button>
      </div>
    </div>
  );
}

function ImportBanner({ total, done }: { total: number; done: number }) {
  return (
    <div className="import-banner" role="status">
      <div className="spinner" />
      <p>
        Processing recording {Math.min(done + 1, total)} of {total}…
      </p>
      <div className="progress wide">
        <div className="progress-bar" style={{ width: `${(done / total) * 100}%` }} />
      </div>
    </div>
  );
}

export function MainPane() {
  const { recording, currentNote, importing } = useHalo();

  if (recording.status === "recording") {
    return (
      <main className="main-pane centered">
        <RecordingActive startedAt={recording.startedAt} />
      </main>
    );
  }

  if (recording.status === "transcribing") {
    return (
      <main className="main-pane centered">
        {importing && <ImportBanner total={importing.total} done={importing.done} />}
        <Processing label="Transcribing audio…" percent={recording.percent} />
      </main>
    );
  }

  if (recording.status === "generating") {
    return (
      <main className="main-pane">
        <NoteDetail streaming />
      </main>
    );
  }

  if (recording.status === "researching") {
    return (
      <main className="main-pane centered">
        <Processing label="Researching topics online…" />
      </main>
    );
  }

  if (currentNote) {
    return (
      <main className="main-pane">
        <NoteDetail />
      </main>
    );
  }

  return (
    <main className="main-pane centered">
      {importing && <ImportBanner total={importing.total} done={importing.done} />}
      <EmptyState />
    </main>
  );
}
