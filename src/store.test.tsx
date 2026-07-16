import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("./ipc", () => ({
  api: {
    getAppStatus: vi.fn(),
    getSettings: vi.fn(),
    getModels: vi.fn(),
    getNoteStyles: vi.fn(),
    listNotes: vi.fn(),
    updateSettings: vi.fn(),
    getNote: vi.fn(),
    createNote: vi.fn(),
    startRecording: vi.fn(),
    stopRecording: vi.fn(),
    cancelRecording: vi.fn(),
    transcribe: vi.fn(),
    generateNotes: vi.fn(),
    saveNote: vi.fn(),
    deleteNote: vi.fn(),
    suggestedNoteTitle: vi.fn(),
  },
  events: {
    onAudioLevel: vi.fn(),
    onTranscribeProgress: vi.fn(),
    onNotesToken: vi.fn(),
  },
}));

import { api, events } from "./ipc";
import { HaloProvider, useHalo } from "./store";
import type { Note, Settings } from "./types";

const m = (fn: unknown) => fn as Mock;
const lastArg = (fn: unknown, i = 0): unknown => {
  const calls = (fn as Mock).mock.calls;
  return calls[calls.length - 1]?.[i];
};

const baseSettings: Settings = {
  setupComplete: true,
  defaultStyleId: "meeting",
  inputDeviceId: null,
  captureSystemAudio: true,
  captureMicrophone: true,
  integrations: [],
};

const baseNote: Note = {
  id: "n1",
  title: "New recording",
  createdAt: "2026-07-16T10:00:00Z",
  updatedAt: "2026-07-16T10:00:00Z",
  styleId: "meeting",
  content: "",
  transcript: null,
  audioPath: null,
  durationSecs: 0,
};

let ctx!: ReturnType<typeof useHalo>;
function Capture() {
  ctx = useHalo();
  return null;
}

let audioCb: (p: unknown) => void = () => {};

beforeEach(() => {
  vi.clearAllMocks();
  m(api.getAppStatus).mockResolvedValue({ setupComplete: true, modelsReady: true, version: "0.1" });
  m(api.getSettings).mockResolvedValue(baseSettings);
  m(api.getModels).mockResolvedValue([]);
  m(api.getNoteStyles).mockResolvedValue([
    { id: "meeting", name: "Meeting", description: "", prompt: "", builtin: true },
    { id: "lecture", name: "Lecture", description: "", prompt: "", builtin: true },
  ]);
  m(api.listNotes).mockResolvedValue([]);
  m(api.updateSettings).mockImplementation((s: Settings) => Promise.resolve(s));
  m(api.getNote).mockImplementation((id: string) => Promise.resolve({ ...baseNote, id }));
  m(api.createNote).mockImplementation((title: string) => Promise.resolve({ ...baseNote, title }));
  m(api.suggestedNoteTitle).mockResolvedValue("Standup");
  m(api.startRecording).mockResolvedValue(undefined);
  m(api.stopRecording).mockResolvedValue(42);
  m(api.transcribe).mockResolvedValue({ segments: [], text: "hello", language: "en" });
  m(api.generateNotes).mockImplementation((id: string, styleId: string) =>
    Promise.resolve({ ...baseNote, id, styleId, content: "# Notes" }),
  );
  m(api.saveNote).mockImplementation((note: Note) => Promise.resolve(note));
  m(api.deleteNote).mockResolvedValue(undefined);
  m(api.cancelRecording).mockResolvedValue(undefined);
  m(events.onAudioLevel).mockImplementation((cb: (p: unknown) => void) => {
    audioCb = cb;
    return Promise.resolve(() => {});
  });
  m(events.onTranscribeProgress).mockResolvedValue(() => {});
  m(events.onNotesToken).mockResolvedValue(() => {});
});

afterEach(cleanup);

async function mount() {
  render(
    <HaloProvider>
      <Capture />
    </HaloProvider>,
  );
  await waitFor(() => expect(ctx.view).not.toBe("loading"));
}

describe("initial load", () => {
  it("goes to home when setup complete and models ready", async () => {
    await mount();
    expect(ctx.view).toBe("home");
    expect(ctx.settings?.setupComplete).toBe(true);
    expect(ctx.styles).toHaveLength(2);
  });

  it("goes to setup when not complete", async () => {
    m(api.getSettings).mockResolvedValue({ ...baseSettings, setupComplete: false });
    await mount();
    expect(ctx.view).toBe("setup");
  });

  it("surfaces load errors", async () => {
    m(api.getSettings).mockRejectedValue(new Error("boom load"));
    render(
      <HaloProvider>
        <Capture />
      </HaloProvider>,
    );
    await waitFor(() => expect(ctx.error).toBe("boom load"));
  });
});

describe("settings", () => {
  it("saveSettings persists and updates state", async () => {
    await mount();
    await act(async () => {
      await ctx.saveSettings({ ...baseSettings, defaultStyleId: "lecture" });
    });
    expect(api.updateSettings).toHaveBeenCalled();
    expect(ctx.settings?.defaultStyleId).toBe("lecture");
  });

  it("completeSetup flips the flag and refreshes", async () => {
    await mount();
    m(api.getAppStatus).mockClear();
    await act(async () => {
      await ctx.completeSetup();
    });
    expect(lastArg(api.updateSettings, 0)).toMatchObject({ setupComplete: true });
    expect(api.getAppStatus).toHaveBeenCalled();
  });

  it("captures and clears errors", async () => {
    await mount();
    m(api.updateSettings).mockRejectedValueOnce(new Error("save failed"));
    await act(async () => {
      await ctx.saveSettings(baseSettings);
    });
    expect(ctx.error).toBe("save failed");
    act(() => ctx.clearError());
    expect(ctx.error).toBeNull();
  });
});

describe("notes", () => {
  it("opens and closes a note", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("abc");
    });
    expect(ctx.currentNote?.id).toBe("abc");
    act(() => ctx.closeNote());
    expect(ctx.currentNote).toBeNull();
  });

  it("edits and persists a note", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("edit1");
    });
    act(() => {
      ctx.updateNoteTitle("My title");
      ctx.updateNoteContent("Body text");
    });
    await act(async () => {
      await ctx.persistCurrentNote();
    });
    expect(lastArg(api.saveNote, 0)).toMatchObject({ title: "My title", content: "Body text" });
  });

  it("deletes the current note and closes it", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("del1");
    });
    await act(async () => {
      await ctx.deleteNote("del1");
    });
    expect(api.deleteNote).toHaveBeenCalledWith("del1");
    expect(ctx.currentNote).toBeNull();
  });

  it("stringifies non-Error failures", async () => {
    await mount();
    m(api.deleteNote).mockRejectedValueOnce("kaboom");
    await act(async () => {
      await ctx.deleteNote("x");
    });
    expect(ctx.error).toBe("kaboom");
  });
});

describe("recording flow", () => {
  it("records, transcribes and generates notes", async () => {
    await mount();
    await act(async () => {
      await ctx.startNewRecording();
    });
    expect(api.suggestedNoteTitle).toHaveBeenCalled();
    expect(api.startRecording).toHaveBeenCalled();
    expect(ctx.recording.status).toBe("recording");

    await act(async () => {
      await ctx.stopRecording();
    });
    expect(api.transcribe).toHaveBeenCalled();
    expect(api.generateNotes).toHaveBeenCalled();
    expect(ctx.recording.status).toBe("idle");
    expect(ctx.currentNote?.content).toBe("# Notes");
  });

  it("cancels a recording", async () => {
    await mount();
    await act(async () => {
      await ctx.startNewRecording();
    });
    await act(async () => {
      await ctx.cancelRecording();
    });
    expect(api.cancelRecording).toHaveBeenCalled();
    expect(ctx.recording.status).toBe("idle");
  });

  it("regenerates notes for the current note", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("regen1");
    });
    await act(async () => {
      await ctx.regenerate("lecture");
    });
    expect(lastArg(api.generateNotes, 1)).toBe("lecture");
  });

  it("falls back to a default title when the calendar lookup fails", async () => {
    await mount();
    m(api.suggestedNoteTitle).mockRejectedValueOnce(new Error("no calendar"));
    await act(async () => {
      await ctx.startNewRecording();
    });
    expect(lastArg(api.createNote, 0)).toBe("New recording");
  });
});

describe("live events", () => {
  it("updates the input level from the audio event", async () => {
    await mount();
    act(() => audioCb({ rms: 0.4, peak: 0.9 }));
    expect(ctx.level).toEqual({ rms: 0.4, peak: 0.9 });
  });
});
