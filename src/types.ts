// Shared types describing the Tauri IPC contract between the React frontend
// and the Rust backend. Every interface here has a matching serde struct in
// `src-tauri/src`.

export type NoteStyleId = string;

export interface NoteStyle {
  id: NoteStyleId;
  name: string;
  description: string;
  /** Prompt template. `{transcript}` is replaced with the raw transcript. */
  prompt: string;
  builtin: boolean;
}

export type ModelKind = "whisper" | "llm";

export interface ModelInfo {
  id: string;
  kind: ModelKind;
  name: string;
  /** Approximate download size in bytes. */
  sizeBytes: number;
  /** Whether the model file is present on disk and ready to use. */
  installed: boolean;
  /** License short name, e.g. "Apache-2.0" or "Llama Community". */
  license: string;
}

export interface ModelDownloadProgress {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number;
  done: boolean;
  error: string | null;
}

export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export type IntegrationId =
  | "markdown"
  | "obsidian"
  | "clipboard"
  | "notion"
  | "slack"
  | "webhook"
  | "google-calendar"
  | "apple-calendar"
  | "microsoft-calendar";

export interface CalendarEvent {
  title: string;
  start: string;
  end: string;
  provider: string;
}

export interface IntegrationConfig {
  id: IntegrationId;
  enabled: boolean;
  /** Free-form settings, e.g. { folder: "/path" } or { token: "..." }. */
  options: Record<string, string>;
}

export interface Settings {
  setupComplete: boolean;
  defaultStyleId: NoteStyleId;
  /** Selected microphone. null = system default input. */
  inputDeviceId: string | null;
  /** Capture the computer's own audio output (other meeting participants). */
  captureSystemAudio: boolean;
  /** Capture the microphone (the user's own voice). */
  captureMicrophone: boolean;
  integrations: IntegrationConfig[];
}

export interface TranscriptSegment {
  start: number;
  end: number;
  text: string;
}

export interface Transcript {
  segments: TranscriptSegment[];
  text: string;
  language: string;
}

export interface Note {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  styleId: NoteStyleId;
  /** AI-generated markdown notes. */
  content: string;
  /** Raw transcript, may be empty while a recording is still processing. */
  transcript: Transcript | null;
  /** Path to the recorded audio on disk, if retained. */
  audioPath: string | null;
  durationSecs: number;
}

export interface NoteSummary {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  preview: string;
  durationSecs: number;
}

export type RecordingState =
  | { status: "idle" }
  | { status: "recording"; startedAt: number; noteId: string }
  | { status: "transcribing"; noteId: string; percent: number }
  | { status: "generating"; noteId: string };

export interface AudioLevel {
  rms: number;
  peak: number;
}

export interface TranscribeProgress {
  noteId: string;
  percent: number;
  partialText: string | null;
}

export interface NotesToken {
  noteId: string;
  text: string;
}

export interface AppStatus {
  setupComplete: boolean;
  modelsReady: boolean;
  version: string;
}

export type ExportTarget =
  | { kind: "markdown" }
  | { kind: "obsidian" }
  | { kind: "clipboard"; format: "markdown" | "plain" }
  | { kind: "notion" }
  | { kind: "slack" }
  | { kind: "webhook" };

export interface ExportResult {
  ok: boolean;
  /** Where the note ended up, e.g. a file path or URL. */
  location: string | null;
  message: string;
}
