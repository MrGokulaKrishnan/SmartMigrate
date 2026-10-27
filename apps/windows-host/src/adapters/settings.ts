// TODO: wire to Rust command get_app_settings
// TODO: wire to Rust command save_app_settings

export interface AppSettings {
  startWithWindows: boolean;
  minimizeToTray: boolean;
  defaultQuality: "auto" | "60" | "120";
  liveSessionIndicatorLocked: boolean; // Always true per security policy
  updateChannel: "stable" | "beta";
}

const STORAGE_KEY = "sm_host_settings";

const DEFAULT_SETTINGS: AppSettings = {
  startWithWindows: false,
  minimizeToTray: true,
  defaultQuality: "auto",
  liveSessionIndicatorLocked: true,
  updateChannel: "stable",
};

export async function getAppSettings(): Promise<AppSettings> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(raw), liveSessionIndicatorLocked: true };
    }
  } catch (_) {}
  return DEFAULT_SETTINGS;
}

export async function saveAppSettings(settings: Partial<AppSettings>): Promise<AppSettings> {
  const current = await getAppSettings();
  const updated: AppSettings = { ...current, ...settings, liveSessionIndicatorLocked: true };
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch (_) {}
  return updated;
}
