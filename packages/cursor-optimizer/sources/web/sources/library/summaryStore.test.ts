import { beforeEach, describe, expect, it, vi } from "vitest";

import type { QuickSummary } from "./types";

const { callBackendMock, blockedState } = vi.hoisted(() => ({
  callBackendMock: vi.fn(),
  blockedState: { value: false },
}));

vi.mock("./ipc", () => ({
  callBackend: () => callBackendMock(),
}));

vi.mock("./operationStore", () => ({
  operationState: { readBlocked: () => blockedState.value },
}));

import { loadQuickSummary, summaryState } from "./summaryStore";

const sample: QuickSummary = {
  databaseBytes: 10,
  databaseHuman: "10 B",
  writeAheadLogBytes: 0,
  writeAheadLogHuman: "0 B",
  totalBytes: 10,
  totalHuman: "10 B",
};

describe("loadQuickSummary", () => {
  beforeEach(() => {
    callBackendMock.mockReset();
    blockedState.value = false;
  });

  it("does not fetch while reads are blocked", async () => {
    blockedState.value = true;
    await loadQuickSummary(true);
    expect(callBackendMock).not.toHaveBeenCalled();
    expect(summaryState.summary()).toBeNull();
  });

  it("fetches once and serves the cached summary afterward", async () => {
    callBackendMock.mockResolvedValue(sample);
    await loadQuickSummary();
    expect(summaryState.summary()).toEqual(sample);
    await loadQuickSummary();
    expect(callBackendMock).toHaveBeenCalledTimes(1);
  });

  it("refetches when forced", async () => {
    callBackendMock.mockResolvedValue(sample);
    await loadQuickSummary(true);
    expect(callBackendMock).toHaveBeenCalledTimes(1);
  });
});
