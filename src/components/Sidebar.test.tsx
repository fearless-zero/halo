import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("../store", () => ({ useHalo: vi.fn() }));
import { useHalo } from "../store";
import { Sidebar } from "./Sidebar";

afterEach(cleanup);

const openNote = vi.fn();
const startNewRecording = vi.fn();

function setCtx(over: Record<string, unknown> = {}) {
  (useHalo as unknown as Mock).mockReturnValue({
    notes: [],
    currentNote: null,
    openNote,
    startNewRecording,
    recording: { status: "idle" },
    ...over,
  });
}

const today = new Date().toISOString();
const notes = [
  { id: "a", title: "Alpha", createdAt: today, updatedAt: today, preview: "first note", durationSecs: 120 },
  { id: "b", title: "Beta", createdAt: "2024-01-02T09:00:00Z", updatedAt: "2024-01-02T09:00:00Z", preview: "second", durationSecs: 0 },
];

describe("Sidebar", () => {
  it("shows an empty hint with no notes", () => {
    setCtx();
    render(<Sidebar onOpenSettings={vi.fn()} />);
    expect(screen.getByText("No notes yet.")).toBeTruthy();
  });

  it("lists notes and opens one on click", () => {
    setCtx({ notes, currentNote: { id: "a" } });
    render(<Sidebar onOpenSettings={vi.fn()} />);
    expect(screen.getByText("Alpha")).toBeTruthy();
    expect(screen.getByText("Beta")).toBeTruthy();
    fireEvent.click(screen.getByText("Beta"));
    expect(openNote).toHaveBeenCalledWith("b");
  });

  it("filters notes by search query", () => {
    setCtx({ notes });
    render(<Sidebar onOpenSettings={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText("Search notes"), { target: { value: "alpha" } });
    expect(screen.getByText("Alpha")).toBeTruthy();
    expect(screen.queryByText("Beta")).toBeNull();
  });

  it("opens settings and starts a recording", () => {
    const onOpenSettings = vi.fn();
    setCtx();
    render(<Sidebar onOpenSettings={onOpenSettings} />);
    fireEvent.click(screen.getByTitle("Settings"));
    expect(onOpenSettings).toHaveBeenCalled();
    fireEvent.click(screen.getByText("New recording"));
    expect(startNewRecording).toHaveBeenCalled();
  });

  it("disables the record button while busy", () => {
    setCtx({ recording: { status: "recording" } });
    render(<Sidebar onOpenSettings={vi.fn()} />);
    expect((screen.getByText("New recording").closest("button") as HTMLButtonElement).disabled).toBe(true);
  });

  it("renders untitled fallback", () => {
    setCtx({ notes: [{ id: "c", title: "", createdAt: today, updatedAt: today, preview: "", durationSecs: 0 }] });
    render(<Sidebar onOpenSettings={vi.fn()} />);
    expect(screen.getByText("Untitled")).toBeTruthy();
  });
});
