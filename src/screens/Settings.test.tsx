import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("../store", () => ({ useHalo: vi.fn() }));
vi.mock("../ipc", () => ({
  api: { listAudioInputs: vi.fn(), saveNoteStyle: vi.fn(), deleteNoteStyle: vi.fn() },
}));

import { useHalo } from "../store";
import { api } from "../ipc";
import { SettingsPanel } from "./Settings";

afterEach(cleanup);

const saveSettings = vi.fn();
const refreshAll = vi.fn();

const styles = [
  { id: "meeting", name: "Meeting", description: "d", prompt: "p", builtin: true },
  { id: "mine", name: "Mine", description: "c", prompt: "x {transcript}", builtin: false },
];

const settings = {
  setupComplete: true,
  defaultStyleId: "meeting",
  inputDeviceId: null,
  captureSystemAudio: true,
  captureMicrophone: true,
  integrations: [
    { id: "markdown", enabled: true, options: {} },
    { id: "notion", enabled: true, options: {} },
    { id: "slack", enabled: true, options: {} },
    { id: "webhook", enabled: true, options: {} },
    { id: "google-calendar", enabled: true, options: {} },
    { id: "clipboard", enabled: true, options: {} },
  ],
};

function setCtx(over: Record<string, unknown> = {}) {
  (useHalo as unknown as Mock).mockReturnValue({
    settings,
    saveSettings,
    refreshAll,
    models: [{ id: "whisper-base", kind: "whisper", name: "Whisper", sizeBytes: 1, installed: true, license: "MIT" }],
    styles,
    ...over,
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  (api.listAudioInputs as unknown as Mock).mockResolvedValue([{ id: "mic1", name: "Mic One", isDefault: true }]);
  (api.saveNoteStyle as unknown as Mock).mockResolvedValue(styles[1]);
  (api.deleteNoteStyle as unknown as Mock).mockResolvedValue(undefined);
});

describe("SettingsPanel", () => {
  it("renders nothing without settings", () => {
    setCtx({ settings: null });
    const { container } = render(<SettingsPanel onClose={vi.fn()} />);
    expect(container.querySelector(".modal")).toBeNull();
  });

  it("loads devices and edits audio settings", async () => {
    setCtx();
    render(<SettingsPanel onClose={vi.fn()} />);
    await waitFor(() => expect(screen.getByText("Mic One")).toBeTruthy());
    const selects = screen.getAllByRole("combobox");
    fireEvent.change(selects[0], { target: { value: "mic1" } });
    expect(saveSettings).toHaveBeenLastCalledWith(expect.objectContaining({ inputDeviceId: "mic1" }));
    fireEvent.change(selects[0], { target: { value: "" } });
    expect(saveSettings).toHaveBeenLastCalledWith(expect.objectContaining({ inputDeviceId: null }));
    fireEvent.click(screen.getByLabelText(/other participants/));
    fireEvent.click(screen.getByLabelText(/your voice/));
    // default style
    fireEvent.change(selects[1], { target: { value: "mine" } });
    expect(saveSettings).toHaveBeenCalledTimes(5);
  });

  it("closes via the close button", () => {
    const onClose = vi.fn();
    setCtx();
    render(<SettingsPanel onClose={onClose} />);
    fireEvent.click(screen.getByText("Close"));
    expect(onClose).toHaveBeenCalled();
  });

  it("closes when clicking the overlay but not the modal", () => {
    const onClose = vi.fn();
    setCtx();
    const { container } = render(<SettingsPanel onClose={onClose} />);
    fireEvent.click(container.querySelector(".modal") as Element);
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.click(container.querySelector(".modal-overlay") as Element);
    expect(onClose).toHaveBeenCalled();
  });

  it("renders integration fields and edits options", () => {
    setCtx();
    render(<SettingsPanel onClose={vi.fn()} />);
    // markdown folder, notion token+database, slack webhook, webhook url, calendar ics
    fireEvent.change(screen.getByPlaceholderText(/Folder for .md files/), { target: { value: "~/N" } });
    expect(saveSettings).toHaveBeenCalled();
    fireEvent.change(screen.getByPlaceholderText(/Notion integration token/), { target: { value: "t" } });
    fireEvent.change(screen.getByPlaceholderText(/Slack incoming webhook/), { target: { value: "w" } });
    fireEvent.change(screen.getByPlaceholderText(/Secret iCal/), { target: { value: "u" } });
    // toggle an integration off
    fireEvent.click(screen.getByLabelText("Slack"));
    expect(saveSettings).toHaveBeenCalled();
  });

  it("creates, edits, saves and deletes note styles", async () => {
    setCtx();
    render(<SettingsPanel onClose={vi.fn()} />);

    fireEvent.click(screen.getByText("+ New style"));
    fireEvent.change(screen.getByPlaceholderText("Style name"), { target: { value: "Custom" } });
    fireEvent.change(screen.getByPlaceholderText("Short description"), { target: { value: "desc" } });
    fireEvent.change(screen.getByPlaceholderText(/use \{transcript\}/), { target: { value: "prompt {transcript}" } });
    await act(async () => {
      fireEvent.click(screen.getByText("Save"));
    });
    expect(api.saveNoteStyle).toHaveBeenCalled();
    expect(refreshAll).toHaveBeenCalled();

    // Cancel path
    fireEvent.click(screen.getByText("+ New style"));
    fireEvent.click(screen.getByText("Cancel"));

    // Edit an existing builtin (no Delete), then delete the custom one
    fireEvent.click(screen.getAllByText("Edit")[0]);
    await act(async () => {
      fireEvent.click(screen.getByText("Delete"));
    });
    expect(api.deleteNoteStyle).toHaveBeenCalledWith("mine");
  });

  it("shows a selected device and a not-installed model", async () => {
    setCtx({
      settings: { ...settings, inputDeviceId: "mic1" },
      models: [{ id: "qwen3-4b", kind: "llm", name: "Qwen3", sizeBytes: 1, installed: false, license: "Apache-2.0" }],
    });
    render(<SettingsPanel onClose={vi.fn()} />);
    await waitFor(() => expect(screen.getByText(/Not installed/)).toBeTruthy());
  });

  it("survives a device enumeration failure", async () => {
    (api.listAudioInputs as unknown as Mock).mockRejectedValue(new Error("no audio"));
    setCtx();
    render(<SettingsPanel onClose={vi.fn()} />);
    await waitFor(() => expect(screen.getByText("System default")).toBeTruthy());
  });
});
