import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("../store", () => ({ useHalo: vi.fn() }));
vi.mock("../ipc", () => ({
  api: { downloadModels: vi.fn() },
  events: { onModelProgress: vi.fn() },
}));

import { useHalo } from "../store";
import { api, events } from "../ipc";
import { Setup } from "./Setup";

afterEach(cleanup);

const saveSettings = vi.fn();
const completeSetup = vi.fn();
const refreshAll = vi.fn();

const styles = [
  { id: "meeting", name: "Meeting", description: "notes", prompt: "", builtin: true },
  { id: "lecture", name: "Lecture", description: "study", prompt: "", builtin: true },
];

const settings = {
  setupComplete: false,
  defaultStyleId: "meeting",
  inputDeviceId: null,
  captureSystemAudio: true,
  captureMicrophone: true,
  integrations: [
    { id: "markdown", enabled: false, options: {} },
    { id: "clipboard", enabled: true, options: {} },
  ],
};

let progressCb: (p: unknown) => void = () => {};

function setCtx(over: Record<string, unknown> = {}) {
  (useHalo as unknown as Mock).mockReturnValue({
    models: [
      { id: "whisper-base", kind: "whisper", name: "Whisper", sizeBytes: 147_000_000, installed: false, license: "MIT" },
      { id: "qwen3-4b", kind: "llm", name: "Qwen3", sizeBytes: 2_500_000_000, installed: false, license: "Apache-2.0" },
    ],
    styles,
    settings,
    saveSettings,
    completeSetup,
    refreshAll,
    ...over,
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  (api.downloadModels as unknown as Mock).mockResolvedValue(undefined);
  (events.onModelProgress as unknown as Mock).mockImplementation((cb: (p: unknown) => void) => {
    progressCb = cb;
    return Promise.resolve(() => {});
  });
});

describe("Setup wizard", () => {
  it("renders nothing until settings load", () => {
    setCtx({ settings: null });
    const { container } = render(<Setup />);
    expect(container.querySelector(".setup-card")).toBeNull();
  });

  it("walks through download, preferences and integrations", async () => {
    setCtx();
    render(<Setup />);

    // Step 0 -> 1
    fireEvent.click(screen.getByText("Get started"));
    expect(screen.getByText("Download the AI models")).toBeTruthy();
    expect(screen.getByText(/GB/)).toBeTruthy(); // fmtSize GB branch
    expect(screen.getByText(/MB/)).toBeTruthy(); // fmtSize MB branch

    // Trigger a download + progress event
    await act(async () => {
      fireEvent.click(screen.getByText("Download models"));
    });
    expect(api.downloadModels).toHaveBeenCalledWith(["whisper-base", "qwen3-4b"]);
    act(() => progressCb({ modelId: "whisper-base", downloadedBytes: 50, totalBytes: 100, done: false, error: null }));
    expect(screen.getByText("50%")).toBeTruthy();
    expect(refreshAll).toHaveBeenCalled();
  });

  it("shows a downloading state while models fetch", async () => {
    setCtx();
    (api.downloadModels as unknown as Mock).mockReturnValue(new Promise(() => {}));
    render(<Setup />);
    fireEvent.click(screen.getByText("Get started"));
    fireEvent.click(screen.getByText("Download models"));
    expect(await screen.findByText("Downloading…")).toBeTruthy();
  });

  it("continues past installed models and finishes setup", () => {
    setCtx({
      models: [
        { id: "whisper-base", kind: "whisper", name: "Whisper", sizeBytes: 147_000_000, installed: true, license: "MIT" },
        { id: "qwen3-4b", kind: "llm", name: "Qwen3", sizeBytes: 2_500_000_000, installed: true, license: "Apache-2.0" },
      ],
    });
    render(<Setup />);
    fireEvent.click(screen.getByText("Get started"));
    expect(screen.getAllByText("Ready").length).toBe(2);
    fireEvent.click(screen.getByText("Continue")); // step 2

    fireEvent.click(screen.getByText("Lecture")); // pick style
    expect(saveSettings).toHaveBeenCalled();
    fireEvent.click(screen.getByLabelText(/Capture system audio/));
    fireEvent.click(screen.getByLabelText(/Capture microphone/));
    fireEvent.click(screen.getByText("Continue")); // step 3

    fireEvent.click(screen.getByText("Markdown export")); // toggle integration
    fireEvent.click(screen.getByText("Finish setup"));
    expect(completeSetup).toHaveBeenCalled();
  });
});
