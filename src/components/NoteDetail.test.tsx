import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("../store", () => ({ useHalo: vi.fn() }));
vi.mock("../ipc", () => ({ api: { exportNote: vi.fn() } }));

import { useHalo } from "../store";
import { api } from "../ipc";
import { NoteDetail } from "./NoteDetail";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const actions = {
  regenerate: vi.fn(),
  updateNoteContent: vi.fn(),
  updateNoteTitle: vi.fn(),
  persistCurrentNote: vi.fn(),
  deleteNote: vi.fn(),
};

const note = {
  id: "n1",
  title: "My Note",
  createdAt: "2026-07-16T10:00:00Z",
  updatedAt: "2026-07-16T10:00:00Z",
  styleId: "meeting",
  content: "## Heading\n- point",
  transcript: { segments: [{ start: 65, end: 70, text: "hello" }], text: "hello world", language: "en" },
  audioPath: null,
  durationSecs: 120,
};

const styles = [
  { id: "meeting", name: "Meeting", description: "", prompt: "", builtin: true },
  { id: "lecture", name: "Lecture", description: "", prompt: "", builtin: true },
];

function setCtx(over: Record<string, unknown> = {}) {
  (useHalo as unknown as Mock).mockReturnValue({
    currentNote: note,
    styles,
    settings: { integrations: [{ id: "notion", enabled: true, options: {} }, { id: "slack", enabled: false, options: {} }] },
    streamBuffer: "streaming text",
    ...actions,
    ...over,
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  (api.exportNote as unknown as Mock).mockResolvedValue({ ok: true, message: "Saved" });
});

describe("NoteDetail", () => {
  it("renders nothing without a current note", () => {
    setCtx({ currentNote: null });
    const { container } = render(<NoteDetail />);
    expect(container.textContent).toBe("");
  });

  it("renders rendered markdown and the header", () => {
    setCtx();
    render(<NoteDetail />);
    expect(screen.getByText("Heading")).toBeTruthy();
    expect((screen.getByPlaceholderText("Untitled note") as HTMLInputElement).value).toBe("My Note");
  });

  it("edits title and content and persists", () => {
    setCtx();
    render(<NoteDetail />);
    const title = screen.getByPlaceholderText("Untitled note");
    fireEvent.change(title, { target: { value: "New" } });
    expect(actions.updateNoteTitle).toHaveBeenCalledWith("New");
    fireEvent.blur(title);
    expect(actions.persistCurrentNote).toHaveBeenCalled();
  });

  it("toggles the editor and edits content", () => {
    setCtx();
    const { container } = render(<NoteDetail />);
    fireEvent.click(screen.getByText("Edit"));
    const editor = container.querySelector(".note-editor") as HTMLTextAreaElement;
    fireEvent.change(editor, { target: { value: "changed" } });
    expect(actions.updateNoteContent).toHaveBeenCalledWith("changed");
    fireEvent.blur(editor);
    expect(actions.persistCurrentNote).toHaveBeenCalled();
    fireEvent.click(screen.getByText("Preview"));
  });

  it("regenerates with the selected style", () => {
    setCtx();
    render(<NoteDetail />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "lecture" } });
    fireEvent.click(screen.getByText("Regenerate"));
    expect(actions.regenerate).toHaveBeenCalledWith("lecture");
  });

  it("shows enabled integration export buttons and exports", async () => {
    setCtx();
    render(<NoteDetail />);
    expect(screen.getByText("Notion")).toBeTruthy();
    expect(screen.queryByText("Slack")).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByText("Copy"));
    });
    expect(api.exportNote).toHaveBeenCalledWith("n1", { kind: "clipboard", format: "markdown" });
    await act(async () => {
      fireEvent.click(screen.getByText("Export .md"));
    });
    await act(async () => {
      fireEvent.click(screen.getByText("Notion"));
    });
    expect(screen.getByText("Saved")).toBeTruthy();
  });

  it("reports a failed export and clears the flash", async () => {
    vi.useFakeTimers();
    (api.exportNote as unknown as Mock).mockResolvedValue({ ok: false, message: "nope" });
    setCtx();
    render(<NoteDetail />);
    await act(async () => {
      fireEvent.click(screen.getByText("Export .md"));
    });
    expect(screen.getByText("Export failed: nope")).toBeTruthy();
    act(() => vi.advanceTimersByTime(2500));
    expect(screen.queryByText("Export failed: nope")).toBeNull();
  });

  it("toggles the transcript with segments and delete", () => {
    setCtx();
    render(<NoteDetail />);
    fireEvent.click(screen.getByText("Show transcript"));
    expect(screen.getByText("hello")).toBeTruthy();
    expect(screen.getByText("01:05")).toBeTruthy();
    fireEvent.click(screen.getByText("Hide transcript"));
    fireEvent.click(screen.getByTitle("Delete"));
    expect(actions.deleteNote).toHaveBeenCalledWith("n1");
  });

  it("renders transcript without segments as plain text", () => {
    setCtx({ currentNote: { ...note, transcript: { segments: [], text: "plain transcript", language: "en" } } });
    render(<NoteDetail />);
    fireEvent.click(screen.getByText("Show transcript"));
    expect(screen.getByText("plain transcript")).toBeTruthy();
  });

  it("streams generated text without edit controls", () => {
    setCtx();
    render(<NoteDetail streaming />);
    expect(screen.getByText("streaming text")).toBeTruthy();
    expect(screen.queryByText("Edit")).toBeNull();
  });
});
