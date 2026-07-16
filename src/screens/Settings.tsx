import { useEffect, useState } from "react";
import { useHalo } from "../store";
import { api } from "../ipc";
import { integrationLabel } from "../labels";
import type { AudioDevice, IntegrationConfig, NoteStyle } from "../types";

interface Field {
  key: string;
  placeholder: string;
  help?: string;
}

const INTEGRATION_FIELDS: Record<string, Field[]> = {
  markdown: [{ key: "folder", placeholder: "Folder for .md files (e.g. ~/Notes/Halo)" }],
  obsidian: [{ key: "folder", placeholder: "Path to your Obsidian vault (e.g. ~/Obsidian/Notes)" }],
  notion: [
    { key: "token", placeholder: "Notion integration token (secret_…)" },
    { key: "database", placeholder: "Notion database ID" },
  ],
  slack: [{ key: "webhook", placeholder: "Slack incoming webhook URL" }],
  webhook: [{ key: "url", placeholder: "POST notes as JSON to this URL" }],
  "google-calendar": [
    { key: "ics", placeholder: "Secret iCal (.ics) URL", help: "Google Calendar → Settings → your calendar → Secret address in iCal format" },
  ],
  "apple-calendar": [
    { key: "ics", placeholder: "Public iCal (.ics) URL", help: "Calendar app → right-click calendar → Share → Public Calendar" },
  ],
  "microsoft-calendar": [
    { key: "ics", placeholder: "Published iCal (.ics) URL", help: "Outlook → Settings → Calendar → Shared calendars → Publish → ICS link" },
  ],
};

function IntegrationEditor({
  cfg,
  onChange,
}: {
  cfg: IntegrationConfig;
  onChange: (c: IntegrationConfig) => void;
}) {
  const setOption = (key: string, value: string) =>
    onChange({ ...cfg, options: { ...cfg.options, [key]: value } });

  const fields = INTEGRATION_FIELDS[cfg.id] ?? [];

  return (
    <div className="setting-block">
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={cfg.enabled}
          onChange={(e) => onChange({ ...cfg, enabled: e.target.checked })}
        />
        <span>{integrationLabel(cfg.id)}</span>
      </label>
      {cfg.enabled &&
        fields.map((f) => (
          <div key={f.key} className="setting-block">
            <input
              className="text-input"
              placeholder={f.placeholder}
              value={cfg.options[f.key] ?? ""}
              onChange={(e) => setOption(f.key, e.target.value)}
            />
            {f.help && <span className="field-label">{f.help}</span>}
          </div>
        ))}
    </div>
  );
}

function StyleEditor() {
  const { styles, refreshAll } = useHalo();
  const [draft, setDraft] = useState<NoteStyle | null>(null);

  const newStyle = () =>
    setDraft({
      id: `custom-${Date.now()}`,
      name: "New style",
      description: "",
      prompt: "Summarise the following transcript into clear notes.\n\n{transcript}",
      builtin: false,
    });

  const save = async () => {
    /* v8 ignore next -- Save only renders when a draft exists */
    if (!draft) return;
    await api.saveNoteStyle(draft);
    setDraft(null);
    await refreshAll();
  };

  const remove = async (id: string) => {
    await api.deleteNoteStyle(id);
    await refreshAll();
  };

  return (
    <div>
      <div className="style-list">
        {styles.map((s) => (
          <div key={s.id} className="style-list-row">
            <div>
              <strong>{s.name}</strong>
              <span className="muted"> — {s.description}</span>
            </div>
            <div className="row-actions">
              <button className="btn btn-sm" onClick={() => setDraft(s)}>Edit</button>
              {!s.builtin && (
                <button className="btn btn-sm btn-ghost" onClick={() => void remove(s.id)}>
                  Delete
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
      <button className="btn btn-sm" onClick={newStyle}>+ New style</button>

      {draft && (
        <div className="style-draft">
          <input
            className="text-input"
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            placeholder="Style name"
          />
          <input
            className="text-input"
            value={draft.description}
            onChange={(e) => setDraft({ ...draft, description: e.target.value })}
            placeholder="Short description"
          />
          <textarea
            className="prompt-input"
            value={draft.prompt}
            onChange={(e) => setDraft({ ...draft, prompt: e.target.value })}
            placeholder="Prompt — use {transcript} where the transcript should go"
          />
          <div className="row-actions">
            <button className="btn btn-primary btn-sm" onClick={() => void save()}>Save</button>
            <button className="btn btn-ghost btn-sm" onClick={() => setDraft(null)}>Cancel</button>
          </div>
        </div>
      )}
    </div>
  );
}

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { settings, saveSettings, models, styles } = useHalo();
  const [devices, setDevices] = useState<AudioDevice[]>([]);

  useEffect(() => {
    void api.listAudioInputs().then(setDevices).catch(() => setDevices([]));
  }, []);

  if (!settings) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <h2>Settings</h2>
          <button className="btn btn-ghost btn-sm" onClick={onClose}>Close</button>
        </div>

        <section className="settings-section">
          <h3>Audio</h3>
          <label className="field-label">Microphone</label>
          <select
            className="text-input"
            value={settings.inputDeviceId ?? ""}
            onChange={(e) => saveSettings({ ...settings, inputDeviceId: e.target.value || null })}
          >
            <option value="">System default</option>
            {devices.map((d) => (
              <option key={d.id} value={d.id}>{d.name}</option>
            ))}
          </select>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={settings.captureSystemAudio}
              onChange={(e) => saveSettings({ ...settings, captureSystemAudio: e.target.checked })}
            />
            Capture system audio (other participants)
          </label>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={settings.captureMicrophone}
              onChange={(e) => saveSettings({ ...settings, captureMicrophone: e.target.checked })}
            />
            Capture microphone (your voice)
          </label>
        </section>

        <section className="settings-section">
          <h3>Default note style</h3>
          <select
            className="text-input"
            value={settings.defaultStyleId}
            onChange={(e) => saveSettings({ ...settings, defaultStyleId: e.target.value })}
          >
            {styles.map((s) => (
              <option key={s.id} value={s.id}>{s.name}</option>
            ))}
          </select>
        </section>

        <section className="settings-section">
          <h3>Note styles</h3>
          <StyleEditor />
        </section>

        <section className="settings-section">
          <h3>Integrations</h3>
          {settings.integrations.map((cfg) => (
            <IntegrationEditor
              key={cfg.id}
              cfg={cfg}
              onChange={(next) =>
                saveSettings({
                  ...settings,
                  integrations: settings.integrations.map((c) => (c.id === next.id ? next : c)),
                })
              }
            />
          ))}
        </section>

        <section className="settings-section">
          <h3>Models</h3>
          {models.map((m) => (
            <div key={m.id} className="model-list-row">
              <span>{m.name}</span>
              <span className="muted">{m.installed ? "Ready" : "Not installed"} · {m.license}</span>
            </div>
          ))}
        </section>
      </div>
    </div>
  );
}
