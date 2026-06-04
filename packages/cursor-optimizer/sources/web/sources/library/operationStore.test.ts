import { describe, expect, it } from "vitest";

import { type OperationBlockStatus, operationBlocked } from "./operationRules";

const idleStatus: OperationBlockStatus = {
  recoveryPending: false,
  activeWrite: null,
  activeReads: [],
};

const readSnapshot = {
  requestId: 1,
  action: "overview",
  title: "Analyzing database",
  access: "read",
  startedMillis: 1,
  progress: "",
  elapsedSeconds: 0,
};

const writeSnapshot = {
  requestId: 2,
  action: "toolVacuum",
  title: "Compacting database",
  access: "write",
  startedMillis: 1,
  progress: "",
  elapsedSeconds: 0,
};

describe("operationBlocked", () => {
  it("allows reads and writes while idle", () => {
    expect(operationBlocked("read", idleStatus, false)).toBe(false);
    expect(operationBlocked("write", idleStatus, false)).toBe(false);
  });

  it("blocks writes while reads are active", () => {
    const status = { ...idleStatus, activeReads: [readSnapshot] };
    expect(operationBlocked("read", status, false)).toBe(false);
    expect(operationBlocked("write", status, false)).toBe(true);
  });

  it("blocks all database work during writes and recovery", () => {
    const writing = { ...idleStatus, activeWrite: writeSnapshot };
    const recovering = { ...idleStatus, recoveryPending: true };
    expect(operationBlocked("read", writing, false)).toBe(true);
    expect(operationBlocked("write", writing, false)).toBe(true);
    expect(operationBlocked("read", recovering, false)).toBe(true);
    expect(operationBlocked("write", recovering, false)).toBe(true);
  });
});
