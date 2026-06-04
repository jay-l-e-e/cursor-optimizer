import { describe, expect, it } from "vitest";

import {
  buildInstallParams,
  buildUninstallParams,
  fallbackInfo,
  initialInstallForm,
  initialUninstallForm,
  isInstallDirectoryValid,
  parseInstallerInfo,
  readExecutablePath,
} from "./wizardModel";

describe("parseInstallerInfo", () => {
  it("reads a full install payload", () => {
    const info = parseInstallerInfo({
      mode: "install",
      productName: "Cursor Optimizer",
      version: "0.1.0",
      defaultInstallDirectory: "C:/Program Files/CursorOptimizer",
      installedDirectory: null,
      hasEmbeddedBinary: true,
    });
    expect(info.mode).toBe("install");
    expect(info.version).toBe("0.1.0");
    expect(info.defaultInstallDirectory).toBe("C:/Program Files/CursorOptimizer");
    expect(info.installedDirectory).toBeNull();
    expect(info.hasEmbeddedBinary).toBe(true);
  });

  it("falls back to safe defaults for malformed payloads", () => {
    const info = parseInstallerInfo(null);
    expect(info.mode).toBe("install");
    expect(info.productName).toBe(fallbackInfo.productName);
    expect(info.hasEmbeddedBinary).toBe(false);
  });

  it("recognizes uninstall mode and an existing directory", () => {
    const info = parseInstallerInfo({
      mode: "uninstall",
      installedDirectory: "C:/Program Files/CursorOptimizer",
    });
    expect(info.mode).toBe("uninstall");
    expect(info.installedDirectory).toBe("C:/Program Files/CursorOptimizer");
  });
});

describe("install form", () => {
  it("prefers an existing installation directory when present", () => {
    const form = initialInstallForm({
      ...fallbackInfo,
      defaultInstallDirectory: "C:/Program Files/CursorOptimizer",
      installedDirectory: "D:/Apps/CursorOptimizer",
    });
    expect(form.installDirectory).toBe("D:/Apps/CursorOptimizer");
    expect(form.createStartMenuShortcut).toBe(true);
    expect(form.createDesktopShortcut).toBe(true);
  });

  it("uses the default directory when nothing is installed", () => {
    const form = initialInstallForm({
      ...fallbackInfo,
      defaultInstallDirectory: "C:/Program Files/CursorOptimizer",
    });
    expect(form.installDirectory).toBe("C:/Program Files/CursorOptimizer");
  });

  it("rejects an empty directory", () => {
    expect(
      isInstallDirectoryValid({
        installDirectory: "   ",
        createStartMenuShortcut: true,
        createDesktopShortcut: true,
      }),
    ).toBe(false);
  });

  it("trims the directory when building parameters", () => {
    const params = buildInstallParams({
      installDirectory: "  C:/Apps/CursorOptimizer  ",
      createStartMenuShortcut: false,
      createDesktopShortcut: true,
    });
    expect(params).toEqual({
      installDirectory: "C:/Apps/CursorOptimizer",
      createStartMenuShortcut: false,
      createDesktopShortcut: true,
    });
  });
});

describe("uninstall form", () => {
  it("defaults to removing the application data", () => {
    expect(initialUninstallForm()).toEqual({ removeApplicationData: true });
  });

  it("builds uninstall parameters", () => {
    expect(buildUninstallParams({ removeApplicationData: false })).toEqual({
      removeApplicationData: false,
    });
  });
});

describe("readExecutablePath", () => {
  it("reads the executable path from an install result", () => {
    expect(
      readExecutablePath({ executablePath: "C:/Apps/CursorOptimizer/cursor-optimizer.exe" }),
    ).toBe("C:/Apps/CursorOptimizer/cursor-optimizer.exe");
  });

  it("returns an empty string for malformed results", () => {
    expect(readExecutablePath(undefined)).toBe("");
  });
});
