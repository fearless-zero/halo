import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppStatus,
  AudioDevice,
  AudioLevel,
  CalendarEvent,
  ExportResult,
  ExportTarget,
  ModelDownloadProgress,
  ModelInfo,
  Note,
  NoteStyle,
  NoteSummary,
  NotesToken,
  Settings,
  TranscribeProgress,
  Transcript,
} from "./types";

// Thin, typed wrappers over the Tauri command surface. Keeping every `invoke`
// call in one place means the string command names live in exactly one spot.

export const api = {
  getAppStatus: () => invoke<AppStatus>("get_app_status"),

  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) =>
    invoke<Settings>("update_settings", { settings }),

  listAudioInputs: () => invoke<AudioDevice[]>("list_audio_inputs"),

  getModels: () => invoke<ModelInfo[]>("get_models"),
  downloadModels: (modelIds: string[]) =>
    invoke<void>("download_models", { modelIds }),

  getNoteStyles: () => invoke<NoteStyle[]>("get_note_styles"),
  saveNoteStyle: (style: NoteStyle) =>
    invoke<NoteStyle>("save_note_style", { style }),
  deleteNoteStyle: (id: string) => invoke<void>("delete_note_style", { id }),

  startRecording: (noteId: string, deviceId: string | null) =>
    invoke<void>("start_recording", { noteId, deviceId }),
  stopRecording: () => invoke<number>("stop_recording"),
  cancelRecording: () => invoke<void>("cancel_recording"),

  transcribe: (noteId: string) => invoke<Transcript>("transcribe", { noteId }),
  generateNotes: (noteId: string, styleId: string) =>
    invoke<Note>("generate_notes", { noteId, styleId }),

  listNotes: () => invoke<NoteSummary[]>("list_notes"),
  getNote: (id: string) => invoke<Note>("get_note", { id }),
  createNote: (title: string) => invoke<Note>("create_note", { title }),
  saveNote: (note: Note) => invoke<Note>("save_note", { note }),
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),

  exportNote: (id: string, target: ExportTarget) =>
    invoke<ExportResult>("export_note", { id, target }),

  getCalendarEvents: () => invoke<CalendarEvent[]>("get_calendar_events"),
  suggestedNoteTitle: () => invoke<string>("suggested_note_title"),
};

// Event subscriptions. Each returns a promise resolving to an unlisten fn.

export const events = {
  onModelProgress: (cb: (p: ModelDownloadProgress) => void): Promise<UnlistenFn> =>
    listen<ModelDownloadProgress>("model-download-progress", (e) => cb(e.payload)),

  onAudioLevel: (cb: (l: AudioLevel) => void): Promise<UnlistenFn> =>
    listen<AudioLevel>("recording-level", (e) => cb(e.payload)),

  onTranscribeProgress: (cb: (p: TranscribeProgress) => void): Promise<UnlistenFn> =>
    listen<TranscribeProgress>("transcribe-progress", (e) => cb(e.payload)),

  onNotesToken: (cb: (t: NotesToken) => void): Promise<UnlistenFn> =>
    listen<NotesToken>("notes-token", (e) => cb(e.payload)),

  onNotesDone: (cb: (noteId: string) => void): Promise<UnlistenFn> =>
    listen<string>("notes-done", (e) => cb(e.payload)),
};

/** Detect whether we are running inside the Tauri shell (vs. a plain browser). */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
