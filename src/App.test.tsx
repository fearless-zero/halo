import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("./store", () => ({
  HaloProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useHalo: vi.fn(),
}));
vi.mock("./screens/Setup", () => ({ Setup: () => <div>SETUP</div> }));
vi.mock("./components/Sidebar", () => ({
  Sidebar: ({ onOpenSettings }: { onOpenSettings: () => void }) => (
    <button onClick={onOpenSettings}>open-settings</button>
  ),
}));
vi.mock("./components/MainPane", () => ({ MainPane: () => <div>MAIN</div> }));
vi.mock("./screens/Settings", () => ({
  SettingsPanel: ({ onClose }: { onClose: () => void }) => <button onClick={onClose}>close-settings</button>,
}));

import App from "./App";
import { useHalo } from "./store";

afterEach(cleanup);

const clearError = vi.fn();
function setCtx(over: Record<string, unknown>) {
  (useHalo as unknown as Mock).mockReturnValue({ view: "home", error: null, clearError, ...over });
}

describe("App shell", () => {
  it("shows a loading state", () => {
    setCtx({ view: "loading" });
    render(<App />);
    expect(screen.getByText("Loading Halo…")).toBeTruthy();
  });

  it("shows the setup screen", () => {
    setCtx({ view: "setup" });
    render(<App />);
    expect(screen.getByText("SETUP")).toBeTruthy();
  });

  it("renders the home layout and toggles settings", () => {
    setCtx({ view: "home" });
    render(<App />);
    expect(screen.getByText("MAIN")).toBeTruthy();
    fireEvent.click(screen.getByText("open-settings"));
    expect(screen.getByText("close-settings")).toBeTruthy();
    fireEvent.click(screen.getByText("close-settings"));
    expect(screen.queryByText("close-settings")).toBeNull();
  });

  it("renders an error toast that clears on click", () => {
    setCtx({ view: "home", error: "something broke" });
    render(<App />);
    expect(screen.getByText("something broke")).toBeTruthy();
    fireEvent.click(screen.getByText("something broke"));
    expect(clearError).toHaveBeenCalled();
  });
});
