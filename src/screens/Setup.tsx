import { useEffect, useState } from "react";
import { useHalo } from "../store";
import { api, events } from "../ipc";
import type { ModelDownloadProgress } from "../types";
import { CheckIcon, DownloadIcon, SparkleIcon } from "../components/icons";
import { integrationLabel } from "../labels";

function fmtSize(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${Math.round(mb)} MB`;
}

export function Setup() {
  const { models, styles, settings, saveSettings, completeSetup, refreshAll } = useHalo();
  const [step, setStep] = useState(0);
  const [progress, setProgress] = useState<Record<string, ModelDownloadProgress>>({});
  const [downloading, setDownloading] = useState(false);

  useEffect(() => {
    const unlisten = events.onModelProgress((p) => {
      setProgress((prev) => ({ ...prev, [p.modelId]: p }));
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const allInstalled = models.length > 0 && models.every((m) => m.installed);

  const startDownload = async () => {
    setDownloading(true);
    try {
      await api.downloadModels(models.filter((m) => !m.installed).map((m) => m.id));
      await refreshAll();
    } finally {
      setDownloading(false);
    }
  };

  if (!settings) return null;

  return (
    <div className="setup">
      <div className="setup-card">
        <div className="setup-brand">
          <SparkleIcon width={28} height={28} />
          <h1>Halo</h1>
        </div>

        {step === 0 && (
          <div className="setup-step">
            <h2>Private AI notes, on your machine</h2>
            <p className="muted">
              Halo listens to your meetings, lectures and conversations, transcribes them
              locally, and writes clean notes for you. Nothing leaves your computer — the AI
              runs entirely offline once set up.
            </p>
            <ul className="setup-points">
              <li>Captures both your microphone and the other side of the call</li>
              <li>Transcription and note-writing run 100% on-device</li>
              <li>Fully open-source models — no accounts, no API keys, no cloud</li>
            </ul>
            <button className="btn btn-primary btn-lg" onClick={() => setStep(1)}>
              Get started
            </button>
          </div>
        )}

        {step === 1 && (
          <div className="setup-step">
            <h2>Download the AI models</h2>
            <p className="muted">
              One-time download. After this, Halo works completely offline.
            </p>
            <div className="model-list">
              {models.map((m) => {
                const p = progress[m.id];
                const pct = p && p.totalBytes > 0
                  ? Math.round((p.downloadedBytes / p.totalBytes) * 100)
                  : m.installed ? 100 : 0;
                return (
                  <div key={m.id} className="model-row">
                    <div className="model-meta">
                      <strong>{m.name}</strong>
                      <span className="muted">
                        {m.kind === "llm" ? "Note writer" : "Transcription"} · {fmtSize(m.sizeBytes)} · {m.license}
                      </span>
                    </div>
                    <div className="model-status">
                      {m.installed ? (
                        <span className="badge badge-ok"><CheckIcon width={14} height={14} /> Ready</span>
                      ) : (
                        <div className="progress">
                          <div className="progress-bar" style={{ width: `${pct}%` }} />
                          <span className="progress-label">{pct}%</span>
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
            {allInstalled ? (
              <button className="btn btn-primary btn-lg" onClick={() => setStep(2)}>
                Continue
              </button>
            ) : (
              <button className="btn btn-primary btn-lg" onClick={startDownload} disabled={downloading}>
                <DownloadIcon width={18} height={18} />
                {downloading ? "Downloading…" : "Download models"}
              </button>
            )}
          </div>
        )}

        {step === 2 && (
          <div className="setup-step">
            <h2>How should Halo write your notes?</h2>
            <p className="muted">Pick a default style. You can change it per note later.</p>
            <div className="style-grid">
              {styles.map((s) => (
                <button
                  key={s.id}
                  className={`style-card ${settings.defaultStyleId === s.id ? "selected" : ""}`}
                  onClick={() => saveSettings({ ...settings, defaultStyleId: s.id })}
                >
                  <strong>{s.name}</strong>
                  <span className="muted">{s.description}</span>
                </button>
              ))}
            </div>
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={settings.captureSystemAudio}
                onChange={(e) => saveSettings({ ...settings, captureSystemAudio: e.target.checked })}
              />
              Capture system audio (hear the other people on the call)
            </label>
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={settings.captureMicrophone}
                onChange={(e) => saveSettings({ ...settings, captureMicrophone: e.target.checked })}
              />
              Capture microphone (your voice)
            </label>
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={settings.webResearch}
                onChange={(e) => saveSettings({ ...settings, webResearch: e.target.checked })}
              />
              Enrich notes with web research when online (uses Wikipedia)
            </label>
            <button className="btn btn-primary btn-lg" onClick={() => setStep(3)}>
              Continue
            </button>
          </div>
        )}

        {step === 3 && (
          <div className="setup-step">
            <h2>Where should notes go?</h2>
            <p className="muted">Turn on integrations now or later in Settings.</p>
            <div className="integration-list">
              {settings.integrations.map((cfg) => (
                <label key={cfg.id} className="checkbox-row">
                  <input
                    type="checkbox"
                    checked={cfg.enabled}
                    onChange={(e) =>
                      saveSettings({
                        ...settings,
                        integrations: settings.integrations.map((c) =>
                          c.id === cfg.id ? { ...c, enabled: e.target.checked } : c,
                        ),
                      })
                    }
                  />
                  <span>{integrationLabel(cfg.id)}</span>
                </label>
              ))}
            </div>
            <button className="btn btn-primary btn-lg" onClick={() => void completeSetup()}>
              Finish setup
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
