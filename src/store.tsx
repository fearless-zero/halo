import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { api, events } from "./ipc";
import type {
  AppStatus,
  AudioLevel,
  ModelInfo,
  Note,
  NoteStyle,
  NoteSummary,
  RecordingState,
  Settings,
} from "./types";

export type View = "loading" | "setup" | "home";

interface HaloState {
  status: AppStatus | null;
  settings: Settings | null;
  models: ModelInfo[];
  styles: NoteStyle[];
  notes: NoteSummary[];
  currentNote: Note | null;
  view: View;
  recording: RecordingState;
  level: AudioLevel;
  /** Tokens accumulated while the model streams notes for the active note. */
  streamBuffer: string;
  error: string | null;
}

interface HaloActions {
  refreshAll: () => Promise<void>;
  saveSettings: (s: Settings) => Promise<void>;
  completeSetup: () => Promise<void>;
  openNote: (id: string) => Promise<void>;
  closeNote: () => void;
  startNewRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  cancelRecording: () => Promise<void>;
  regenerate: (styleId: string) => Promise<void>;
  updateNoteContent: (content: string) => void;
  updateNoteTitle: (title: string) => void;
  persistCurrentNote: () => Promise<void>;
  deleteNote: (id: string) => Promise<void>;
  clearError: () => void;
}

type HaloContextValue = HaloState & HaloActions;

const HaloContext = createContext<HaloContextValue | null>(null);

export function useHalo(): HaloContextValue {
  const ctx = useContext(HaloContext);
  if (!ctx) throw new Error("useHalo must be used within HaloProvider");
  return ctx;
}

export function HaloProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [styles, setStyles] = useState<NoteStyle[]>([]);
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [currentNote, setCurrentNote] = useState<Note | null>(null);
  const [view, setView] = useState<View>("loading");
  const [recording, setRecording] = useState<RecordingState>({ status: "idle" });
  const [level, setLevel] = useState<AudioLevel>({ rms: 0, peak: 0 });
  const [streamBuffer, setStreamBuffer] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Track the note currently being recorded/processed so event handlers that
  // fire outside React's render can attribute streamed data correctly.
  const activeNoteId = useRef<string | null>(null);

  const fail = (e: unknown) => setError(e instanceof Error ? e.message : String(e));

  const refreshNotes = async () => setNotes(await api.listNotes());

  const refreshAll = async () => {
    try {
      const [st, se, md, sy] = await Promise.all([
        api.getAppStatus(),
        api.getSettings(),
        api.getModels(),
        api.getNoteStyles(),
      ]);
      setStatus(st);
      setSettings(se);
      setModels(md);
      setStyles(sy);
      await refreshNotes();
      setView(se.setupComplete && st.modelsReady ? "home" : "setup");
    } catch (e) {
      fail(e);
    }
  };

  useEffect(() => {
    void refreshAll();
    const unsubs: Array<Promise<() => void>> = [
      events.onAudioLevel(setLevel),
      events.onTranscribeProgress((p) => {
        if (p.noteId === activeNoteId.current) {
          setRecording({ status: "transcribing", noteId: p.noteId, percent: p.percent });
        }
      }),
      events.onNotesToken((t) => {
        if (t.noteId === activeNoteId.current) {
          setStreamBuffer((b) => b + t.text);
        }
      }),
    ];
    return () => {
      for (const u of unsubs) void u.then((fn) => fn());
    };
  }, []);

  const saveSettings = async (s: Settings) => {
    try {
      setSettings(await api.updateSettings(s));
    } catch (e) {
      fail(e);
    }
  };

  const completeSetup = async () => {
    if (!settings) return;
    await saveSettings({ ...settings, setupComplete: true });
    await refreshAll();
  };

  const openNote = async (id: string) => {
    try {
      setStreamBuffer("");
      setCurrentNote(await api.getNote(id));
    } catch (e) {
      fail(e);
    }
  };

  const closeNote = () => {
    setCurrentNote(null);
    setStreamBuffer("");
  };

  const runProcessing = async (noteId: string, styleId: string) => {
    activeNoteId.current = noteId;
    try {
      setRecording({ status: "transcribing", noteId, percent: 0 });
      await api.transcribe(noteId);
      setRecording({ status: "generating", noteId });
      setStreamBuffer("");
      const note = await api.generateNotes(noteId, styleId);
      setCurrentNote(note);
      setStreamBuffer("");
      await refreshNotes();
    } catch (e) {
      fail(e);
    } finally {
      setRecording({ status: "idle" });
      activeNoteId.current = null;
    }
  };

  const startNewRecording = async () => {
    if (!settings) return;
    try {
      const note = await api.createNote("New recording");
      setCurrentNote(note);
      activeNoteId.current = note.id;
      await api.startRecording(note.id, settings.inputDeviceId);
      setRecording({ status: "recording", startedAt: Date.now(), noteId: note.id });
      await refreshNotes();
    } catch (e) {
      fail(e);
    }
  };

  const stopRecording = async () => {
    if (recording.status !== "recording" || !settings) return;
    const noteId = recording.noteId;
    try {
      await api.stopRecording();
      await runProcessing(noteId, settings.defaultStyleId);
    } catch (e) {
      fail(e);
    }
  };

  const cancelRecording = async () => {
    try {
      await api.cancelRecording();
    } catch (e) {
      fail(e);
    } finally {
      setRecording({ status: "idle" });
      activeNoteId.current = null;
    }
  };

  const regenerate = async (styleId: string) => {
    if (!currentNote) return;
    await runProcessing(currentNote.id, styleId);
  };

  const updateNoteContent = (content: string) =>
    setCurrentNote((n) => (n ? { ...n, content } : n));

  const updateNoteTitle = (title: string) =>
    setCurrentNote((n) => (n ? { ...n, title } : n));

  const persistCurrentNote = async () => {
    if (!currentNote) return;
    try {
      const saved = await api.saveNote(currentNote);
      setCurrentNote(saved);
      await refreshNotes();
    } catch (e) {
      fail(e);
    }
  };

  const deleteNote = async (id: string) => {
    try {
      await api.deleteNote(id);
      if (currentNote?.id === id) closeNote();
      await refreshNotes();
    } catch (e) {
      fail(e);
    }
  };

  const value = useMemo<HaloContextValue>(
    () => ({
      status,
      settings,
      models,
      styles,
      notes,
      currentNote,
      view,
      recording,
      level,
      streamBuffer,
      error,
      refreshAll,
      saveSettings,
      completeSetup,
      openNote,
      closeNote,
      startNewRecording,
      stopRecording,
      cancelRecording,
      regenerate,
      updateNoteContent,
      updateNoteTitle,
      persistCurrentNote,
      deleteNote,
      clearError: () => setError(null),
    }),
    [status, settings, models, styles, notes, currentNote, view, recording, level, streamBuffer, error],
  );

  return <HaloContext.Provider value={value}>{children}</HaloContext.Provider>;
}
