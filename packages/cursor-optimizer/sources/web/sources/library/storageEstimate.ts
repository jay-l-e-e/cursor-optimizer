import { callBackend } from "./ipc";

import type { StorageEstimate } from "./types";

export async function loadStorageEstimate(): Promise<StorageEstimate | null> {
  try {
    return await callBackend<StorageEstimate>("storageEstimate");
  } catch {
    return null;
  }
}

export function describeStorageRisk(
  estimate: StorageEstimate | null,
  requiredBytes: number | null,
): string {
  const available = estimate?.availableHuman ?? "unknown";
  const required =
    requiredBytes === null ? "a similar amount" : (estimate?.databaseHuman ?? "a similar amount");
  const warning =
    estimate?.availableBytes != null &&
    requiredBytes != null &&
    estimate.availableBytes < requiredBytes
      ? " Free disk space may be insufficient."
      : "";
  return `Disk space: ${available} available, ~${required} needed temporarily.${warning}`;
}
