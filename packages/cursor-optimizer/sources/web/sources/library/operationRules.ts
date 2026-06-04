export type OperationBlockStatus = {
  recoveryPending: boolean;
  activeWrite: unknown | null;
  activeReads: readonly unknown[];
};

export function operationBlocked(
  access: "read" | "write",
  currentStatus: OperationBlockStatus,
  localBusy: boolean,
): boolean {
  if (localBusy || currentStatus.recoveryPending || currentStatus.activeWrite !== null) {
    return true;
  }
  return access === "write" && currentStatus.activeReads.length > 0;
}
