import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { api, events, inTauri } from "./ipc";
import type { Note, NoteStyle, Settings } from "./types";

const invokeMock = invoke as unknown as Mock;
const listenMock = listen as unknown as Mock;

const settings = { setupComplete: true } as unknown as Settings;
const style = { id: "s" } as unknown as NoteStyle;
const note = { id: "n" } as unknown as Note;

beforeEach(() => {
  invokeMock.mockReset().mockResolvedValue("ok");
  listenMock.mockReset().mockResolvedValue(() => {});
});

describe("api command wrappers", () => {
  it("call invoke with the right command and args", async () => {
    const calls: Array<[Promise<unknown>, string, unknown]> = [
      [api.getAppStatus(), "get_app_status", undefined],
      [api.getSettings(), "get_settings", undefined],
      [api.updateSettings(settings), "update_settings", { settings }],
      [api.listAudioInputs(), "list_audio_inputs", undefined],
      [api.getModels(), "get_models", undefined],
      [api.downloadModels(["a", "b"]), "download_models", { modelIds: ["a", "b"] }],
      [api.getNoteStyles(), "get_note_styles", undefined],
      [api.saveNoteStyle(style), "save_note_style", { style }],
      [api.deleteNoteStyle("s"), "delete_note_style", { id: "s" }],
      [api.startRecording("n", "dev"), "start_recording", { noteId: "n", deviceId: "dev" }],
      [api.stopRecording(), "stop_recording", undefined],
      [api.cancelRecording(), "cancel_recording", undefined],
      [api.transcribe("n"), "transcribe", { noteId: "n" }],
      [api.generateNotes("n", "s"), "generate_notes", { noteId: "n", styleId: "s" }],
      [api.listNotes(), "list_notes", undefined],
      [api.getNote("n"), "get_note", { id: "n" }],
      [api.createNote("Title"), "create_note", { title: "Title" }],
      [api.saveNote(note), "save_note", { note }],
      [api.deleteNote("n"), "delete_note", { id: "n" }],
      [api.exportNote("n", { kind: "markdown" }), "export_note", { id: "n", target: { kind: "markdown" } }],
      [api.getCalendarEvents(), "get_calendar_events", undefined],
      [api.suggestedNoteTitle(), "suggested_note_title", undefined],
    ];
    await Promise.all(calls.map(([p]) => p));
    calls.forEach(([, cmd, args], i) => {
      expect(invokeMock.mock.calls[i][0]).toBe(cmd);
      expect(invokeMock.mock.calls[i][1]).toEqual(args);
    });
    expect(invokeMock).toHaveBeenCalledTimes(calls.length);
  });
});

describe("event subscriptions", () => {
  const subs: Array<[string, (cb: (p: never) => void) => Promise<unknown>, unknown]> = [
    ["model-download-progress", (cb) => events.onModelProgress(cb as never), { modelId: "m" }],
    ["recording-level", (cb) => events.onAudioLevel(cb as never), { rms: 1, peak: 1 }],
    ["transcribe-progress", (cb) => events.onTranscribeProgress(cb as never), { noteId: "n", percent: 1 }],
    ["notes-token", (cb) => events.onNotesToken(cb as never), { noteId: "n", text: "x" }],
    ["notes-done", (cb) => events.onNotesDone(cb as never), "n"],
  ];

  it("register the right channel and forward the payload", async () => {
    for (const [channel, subscribe, payload] of subs) {
      listenMock.mockReset().mockResolvedValue(() => {});
      const cb = vi.fn();
      await subscribe(cb as never);
      expect(listenMock.mock.lastCall?.[0]).toBe(channel);
      const handler = listenMock.mock.lastCall?.[1] as (e: { payload: unknown }) => void;
      handler({ payload });
      expect(cb).toHaveBeenCalledWith(payload);
    }
  });
});

describe("inTauri", () => {
  it("detects the Tauri shell via globals", () => {
    expect(inTauri()).toBe(false);
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(inTauri()).toBe(true);
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
});
