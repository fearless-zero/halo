import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("../store", () => ({ useHalo: vi.fn() }));
vi.mock("./NoteDetail", () => ({
  NoteDetail: ({ streaming }: { streaming?: boolean }) => <div>NOTE-DETAIL:{streaming ? "stream" : "static"}</div>,
}));

import { useHalo } from "../store";
import { MainPane } from "./MainPane";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const stopRecording = vi.fn();
const cancelRecording = vi.fn();
const startNewRecording = vi.fn();
const importRecordings = vi.fn();

function setCtx(over: Record<string, unknown>) {
  (useHalo as unknown as Mock).mockReturnValue({
    recording: { status: "idle" },
    currentNote: null,
    level: { rms: 0.9, peak: 0.5 },
    importing: null,
    stopRecording,
    cancelRecording,
    startNewRecording,
    importRecordings,
    ...over,
  });
}

describe("MainPane", () => {
  it("shows the empty state and starts recording", () => {
    setCtx({});
    render(<MainPane />);
    expect(screen.getByText("Ready when you are")).toBeTruthy();
    fireEvent.click(screen.getByText("Start recording"));
    expect(startNewRecording).toHaveBeenCalled();
    fireEvent.click(screen.getByText("Import recordings"));
    expect(importRecordings).toHaveBeenCalled();
  });

  it("shows the import progress banner in the empty state", () => {
    setCtx({ importing: { total: 3, done: 1 } });
    render(<MainPane />);
    expect(screen.getByText("Processing recording 2 of 3…")).toBeTruthy();
  });

  it("shows the import banner alongside transcribing progress", () => {
    setCtx({
      recording: { status: "transcribing", noteId: "n", percent: 40 },
      importing: { total: 2, done: 0 },
    });
    render(<MainPane />);
    expect(screen.getByText("Processing recording 1 of 2…")).toBeTruthy();
    expect(screen.getByText("Transcribing audio…")).toBeTruthy();
  });

  it("shows the researching state", () => {
    setCtx({ recording: { status: "researching", noteId: "n" } });
    render(<MainPane />);
    expect(screen.getByText("Researching topics online…")).toBeTruthy();
  });

  it("renders the active recording view with controls and a ticking timer", () => {
    vi.useFakeTimers();
    setCtx({ recording: { status: "recording", startedAt: Date.now(), noteId: "n" } });
    render(<MainPane />);
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(screen.getByText(/Listening/)).toBeTruthy();
    fireEvent.click(screen.getByText("Stop & write notes"));
    expect(stopRecording).toHaveBeenCalled();
    fireEvent.click(screen.getByText("Discard"));
    expect(cancelRecording).toHaveBeenCalled();
  });

  it("shows transcribing progress", () => {
    setCtx({ recording: { status: "transcribing", noteId: "n", percent: 40 } });
    render(<MainPane />);
    expect(screen.getByText("Transcribing audio…")).toBeTruthy();
  });

  it("shows streaming note detail while generating", () => {
    setCtx({ recording: { status: "generating", noteId: "n" } });
    render(<MainPane />);
    expect(screen.getByText("NOTE-DETAIL:stream")).toBeTruthy();
  });

  it("shows the current note when idle", () => {
    setCtx({ currentNote: { id: "n" } });
    render(<MainPane />);
    expect(screen.getByText("NOTE-DETAIL:static")).toBeTruthy();
  });
});
