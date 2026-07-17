import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("./ipc", () => ({ inTauri: vi.fn() }));

import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { inTauri } from "./ipc";
import { checkForUpdate } from "./updater";

const m = (fn: unknown) => fn as Mock;

beforeEach(() => vi.clearAllMocks());

describe("checkForUpdate", () => {
  it("returns null outside the Tauri shell without hitting the plugin", async () => {
    m(inTauri).mockReturnValue(false);
    expect(await checkForUpdate()).toBeNull();
    expect(check).not.toHaveBeenCalled();
  });

  it("returns null when the app is up to date", async () => {
    m(inTauri).mockReturnValue(true);
    m(check).mockResolvedValue(null);
    expect(await checkForUpdate()).toBeNull();
  });

  it("wraps an available update and installs then relaunches", async () => {
    m(inTauri).mockReturnValue(true);
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    m(check).mockResolvedValue({ version: "0.3.0", downloadAndInstall });
    m(relaunch).mockResolvedValue(undefined);

    const pending = await checkForUpdate();
    expect(pending?.version).toBe("0.3.0");
    await pending!.install();
    expect(downloadAndInstall).toHaveBeenCalled();
    expect(relaunch).toHaveBeenCalled();
  });
});
