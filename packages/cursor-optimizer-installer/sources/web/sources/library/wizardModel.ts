export type InstallerMode = "install" | "uninstall";

export type InstallerInfo = {
  mode: InstallerMode;
  productName: string;
  version: string;
  defaultInstallDirectory: string;
  installedDirectory: string | null;
  installedVersion: string | null;
  hasEmbeddedBinary: boolean;
};

export type InstallForm = {
  installDirectory: string;
  createStartMenuShortcut: boolean;
  createDesktopShortcut: boolean;
};

export type UninstallForm = {
  removeApplicationData: boolean;
};

export const fallbackInfo: InstallerInfo = {
  mode: "install",
  productName: "Cursor Optimizer",
  version: "",
  defaultInstallDirectory: "",
  installedDirectory: null,
  installedVersion: null,
  hasEmbeddedBinary: false,
};

function toRecord(payload: unknown): Record<string, unknown> {
  if (typeof payload === "object" && payload !== null) {
    return payload as Record<string, unknown>;
  }
  return {};
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  return typeof value === "string" ? value : "";
}

function readOptionalString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value !== "" ? value : null;
}

function readBoolean(record: Record<string, unknown>, key: string): boolean {
  return record[key] === true;
}

export function parseInstallerInfo(payload: unknown): InstallerInfo {
  const record = toRecord(payload);
  const mode = readString(record, "mode") === "uninstall" ? "uninstall" : "install";
  return {
    mode,
    productName: readString(record, "productName") || fallbackInfo.productName,
    version: readString(record, "version"),
    defaultInstallDirectory: readString(record, "defaultInstallDirectory"),
    installedDirectory: readOptionalString(record, "installedDirectory"),
    installedVersion: readOptionalString(record, "installedVersion"),
    hasEmbeddedBinary: readBoolean(record, "hasEmbeddedBinary"),
  };
}

export function initialInstallForm(info: InstallerInfo): InstallForm {
  const installedDirectory = info.installedDirectory ?? "";
  return {
    installDirectory: installedDirectory !== "" ? installedDirectory : info.defaultInstallDirectory,
    createStartMenuShortcut: true,
    createDesktopShortcut: true,
  };
}

export function initialUninstallForm(): UninstallForm {
  return { removeApplicationData: true };
}

export function isInstallDirectoryValid(form: InstallForm): boolean {
  return form.installDirectory.trim() !== "";
}

export function buildInstallParams(form: InstallForm): Record<string, unknown> {
  return {
    installDirectory: form.installDirectory.trim(),
    createStartMenuShortcut: form.createStartMenuShortcut,
    createDesktopShortcut: form.createDesktopShortcut,
  };
}

export function buildUninstallParams(form: UninstallForm): Record<string, unknown> {
  return {
    removeApplicationData: form.removeApplicationData,
  };
}

export function readExecutablePath(payload: unknown): string {
  return readString(toRecord(payload), "executablePath");
}
