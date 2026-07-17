import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { inTauri } from "./ipc";

export interface PendingUpdate {
  version: string;
  /** Download + install the update, then relaunch the app. */
  install: () => Promise<void>;
}

/** Check GitHub Releases for a newer signed build. Returns null when running
 * outside the Tauri shell or when the app is already up to date. */
export async function checkForUpdate(): Promise<PendingUpdate | null> {
  if (!inTauri()) return null;
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    install: async () => {
      await update.downloadAndInstall();
      await relaunch();
    },
  };
}
