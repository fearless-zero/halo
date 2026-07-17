import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

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
    researchNote: vi.fn(),
    importAudio: vi.fn(),
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

import { open as openDialog } from "@tauri-apps/plugin-dialog";
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
  webResearch: true,
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
  research: [],
};

let ctx!: ReturnType<typeof useHalo>;
function Capture() {
  ctx = useHalo();
  return null;
}

let audioCb: (p: unknown) => void = () => {};
let transCb: (p: unknown) => void = () => {};
let tokenCb: (p: unknown) => void = () => {};

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
  m(api.researchNote).mockImplementation((id: string) =>
    Promise.resolve({ ...baseNote, id, content: "# Notes", research: [
      { title: "Topic", summary: "s", url: "https://en.wikipedia.org/wiki/Topic", source: "Wikipedia" },
    ] }),
  );
  m(api.importAudio).mockResolvedValue([
    { ...baseNote, id: "imp1", title: "Class 1" },
    { ...baseNote, id: "imp2", title: "Class 2" },
  ]);
  m(openDialog).mockResolvedValue(["/recordings/class1.m4a", "/recordings/class2.m4a"]);
  m(api.saveNote).mockImplementation((note: Note) => Promise.resolve(note));
  m(api.deleteNote).mockResolvedValue(undefined);
  m(api.cancelRecording).mockResolvedValue(undefined);
  m(events.onAudioLevel).mockImplementation((cb: (p: unknown) => void) => {
    audioCb = cb;
    return Promise.resolve(() => {});
  });
  m(events.onTranscribeProgress).mockImplementation((cb: (p: unknown) => void) => {
    transCb = cb;
    return Promise.resolve(() => {});
  });
  m(events.onNotesToken).mockImplementation((cb: (p: unknown) => void) => {
    tokenCb = cb;
    return Promise.resolve(() => {});
  });
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

describe("web research", () => {
  it("researches after generating when webResearch is on", async () => {
    await mount();
    await act(async () => {
      await ctx.startNewRecording();
    });
    await act(async () => {
      await ctx.stopRecording();
    });
    expect(api.researchNote).toHaveBeenCalled();
    expect(ctx.currentNote?.research).toHaveLength(1);
  });

  it("skips research when webResearch is off", async () => {
    m(api.getSettings).mockResolvedValue({ ...baseSettings, webResearch: false });
    await mount();
    await act(async () => {
      await ctx.openNote("r");
    });
    await act(async () => {
      await ctx.regenerate("meeting");
    });
    expect(api.researchNote).not.toHaveBeenCalled();
  });

  it("keeps the generated note when research fails mid-processing", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("r");
    });
    m(api.researchNote).mockRejectedValueOnce(new Error("offline"));
    await act(async () => {
      await ctx.regenerate("meeting");
    });
    // Research failure is swallowed; the generated note stays.
    expect(ctx.currentNote?.content).toBe("# Notes");
    expect(ctx.error).toBeNull();
  });

  it("skips research during processing when settings are unavailable", async () => {
    m(api.getSettings).mockRejectedValue(new Error("no settings"));
    render(
      <HaloProvider>
        <Capture />
      </HaloProvider>,
    );
    await waitFor(() => expect(ctx.error).toBe("no settings"));
    await act(async () => {
      await ctx.openNote("x");
    });
    await act(async () => {
      await ctx.regenerate("meeting");
    });
    expect(api.generateNotes).toHaveBeenCalled();
    expect(api.researchNote).not.toHaveBeenCalled();
  });

  it("researches the current note on demand", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("manual");
    });
    await act(async () => {
      await ctx.researchCurrentNote();
    });
    expect(api.researchNote).toHaveBeenCalledWith("manual");
    expect(ctx.currentNote?.research).toHaveLength(1);
    expect(ctx.recording.status).toBe("idle");
  });

  it("no-ops manual research with no current note and captures errors", async () => {
    await mount();
    await act(async () => {
      await ctx.researchCurrentNote();
    });
    expect(api.researchNote).not.toHaveBeenCalled();

    await act(async () => {
      await ctx.openNote("m2");
    });
    m(api.researchNote).mockRejectedValueOnce(new Error("net down"));
    await act(async () => {
      await ctx.researchCurrentNote();
    });
    expect(ctx.error).toBe("net down");
    expect(ctx.recording.status).toBe("idle");
  });
});

describe("import recordings", () => {
  it("imports selected files and processes each", async () => {
    await mount();
    await act(async () => {
      await ctx.importRecordings();
    });
    expect(api.importAudio).toHaveBeenCalledWith([
      "/recordings/class1.m4a",
      "/recordings/class2.m4a",
    ]);
    expect(m(api.transcribe).mock.calls.length).toBe(2);
    expect(m(api.generateNotes).mock.calls.length).toBe(2);
    expect(ctx.importing).toBeNull();
  });

  it("wraps a single-file selection into a list", async () => {
    m(openDialog).mockResolvedValueOnce("/recordings/only.wav");
    m(api.importAudio).mockResolvedValueOnce([{ ...baseNote, id: "one" }]);
    await mount();
    await act(async () => {
      await ctx.importRecordings();
    });
    expect(api.importAudio).toHaveBeenCalledWith(["/recordings/only.wav"]);
  });

  it("does nothing when the dialog is cancelled", async () => {
    m(openDialog).mockResolvedValueOnce(null);
    await mount();
    await act(async () => {
      await ctx.importRecordings();
    });
    expect(api.importAudio).not.toHaveBeenCalled();
    expect(ctx.importing).toBeNull();
  });

  it("captures import errors", async () => {
    m(api.importAudio).mockRejectedValueOnce(new Error("bad file"));
    await mount();
    await act(async () => {
      await ctx.importRecordings();
    });
    expect(ctx.error).toBe("bad file");
    expect(ctx.importing).toBeNull();
  });

  it("guards import when settings failed to load", async () => {
    m(api.getSettings).mockRejectedValue(new Error("no settings"));
    render(
      <HaloProvider>
        <Capture />
      </HaloProvider>,
    );
    await waitFor(() => expect(ctx.error).toBe("no settings"));
    await act(async () => {
      await ctx.importRecordings();
    });
    expect(openDialog).not.toHaveBeenCalled();
  });
});

describe("live events", () => {
  it("updates the input level from the audio event", async () => {
    await mount();
    act(() => audioCb({ rms: 0.4, peak: 0.9 }));
    expect(ctx.level).toEqual({ rms: 0.4, peak: 0.9 });
  });

  it("routes transcribe/token events only for the active note", async () => {
    await mount();
    await act(async () => {
      await ctx.startNewRecording();
    });
    // activeNoteId is now the created note ("n1").
    act(() => transCb({ noteId: "n1", percent: 33 }));
    expect(ctx.recording).toMatchObject({ status: "transcribing", percent: 33 });
    act(() => tokenCb({ noteId: "n1", text: "abc" }));
    expect(ctx.streamBuffer).toBe("abc");
    // Events for another note are ignored.
    act(() => tokenCb({ noteId: "other", text: "zzz" }));
    act(() => transCb({ noteId: "other", percent: 99 }));
    expect(ctx.streamBuffer).toBe("abc");
  });
});

describe("guards and error paths", () => {
  it("no-ops actions with no current note", async () => {
    await mount();
    ctx.closeNote();
    act(() => {
      ctx.updateNoteContent("x");
      ctx.updateNoteTitle("y");
    });
    await act(async () => {
      await ctx.persistCurrentNote();
      await ctx.regenerate("meeting");
    });
    expect(api.saveNote).not.toHaveBeenCalled();
    expect(api.generateNotes).not.toHaveBeenCalled();
  });

  it("no-ops stopRecording when idle", async () => {
    await mount();
    await act(async () => {
      await ctx.stopRecording();
    });
    expect(api.stopRecording).not.toHaveBeenCalled();
  });

  it("captures errors from openNote and cancelRecording", async () => {
    await mount();
    m(api.getNote).mockRejectedValueOnce(new Error("no note"));
    await act(async () => {
      await ctx.openNote("bad");
    });
    expect(ctx.error).toBe("no note");

    m(api.cancelRecording).mockRejectedValueOnce(new Error("cancel failed"));
    await act(async () => {
      await ctx.cancelRecording();
    });
    expect(ctx.error).toBe("cancel failed");
    expect(ctx.recording.status).toBe("idle");
  });

  it("captures errors from processing, recording, and persistence", async () => {
    await mount();

    await act(async () => {
      await ctx.openNote("r");
    });
    m(api.transcribe).mockRejectedValueOnce(new Error("t-fail"));
    await act(async () => {
      await ctx.regenerate("meeting");
    });
    expect(ctx.error).toBe("t-fail");
    expect(ctx.recording.status).toBe("idle");

    m(api.createNote).mockRejectedValueOnce(new Error("c-fail"));
    await act(async () => {
      await ctx.startNewRecording();
    });
    expect(ctx.error).toBe("c-fail");

    await act(async () => {
      await ctx.openNote("p");
    });
    m(api.saveNote).mockRejectedValueOnce(new Error("s-fail"));
    await act(async () => {
      await ctx.persistCurrentNote();
    });
    expect(ctx.error).toBe("s-fail");
  });

  it("throws when useHalo is used outside the provider", () => {
    const Bad = () => {
      useHalo();
      return null;
    };
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Bad />)).toThrow("must be used within");
    spy.mockRestore();
  });

  it("leaves a different open note untouched when deleting another", async () => {
    await mount();
    await act(async () => {
      await ctx.openNote("keep");
    });
    await act(async () => {
      await ctx.deleteNote("other");
    });
    expect(ctx.currentNote?.id).toBe("keep");
  });

  it("captures a stopRecording failure", async () => {
    await mount();
    await act(async () => {
      await ctx.startNewRecording();
    });
    m(api.stopRecording).mockRejectedValueOnce(new Error("stop-fail"));
    await act(async () => {
      await ctx.stopRecording();
    });
    expect(ctx.error).toBe("stop-fail");
  });

  it("guards settings-dependent actions when settings failed to load", async () => {
    m(api.getSettings).mockRejectedValue(new Error("no settings"));
    render(
      <HaloProvider>
        <Capture />
      </HaloProvider>,
    );
    await waitFor(() => expect(ctx.error).toBe("no settings"));
    await act(async () => {
      await ctx.completeSetup();
      await ctx.startNewRecording();
    });
    expect(api.updateSettings).not.toHaveBeenCalled();
    expect(api.createNote).not.toHaveBeenCalled();
  });
});
