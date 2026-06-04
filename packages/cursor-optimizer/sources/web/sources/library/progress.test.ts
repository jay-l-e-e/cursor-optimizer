import { describe, expect, it } from "vitest";

import { parseProgress } from "./progress";

describe("parseProgress", () => {
  it("returns null for plain status text", () => {
    expect(parseProgress("scanning chat data")).toBeNull();
  });

  it("returns null for non-progress json", () => {
    expect(parseProgress(JSON.stringify({ kind: "other" }))).toBeNull();
  });

  it("parses a structured progress payload", () => {
    const result = parseProgress(
      JSON.stringify({
        kind: "progress",
        stage: 2,
        stageCount: 4,
        label: "Scanning",
        done: 100,
        total: 400,
      }),
    );
    expect(result).toEqual({
      active: true,
      stage: 2,
      stageCount: 4,
      label: "Scanning",
      done: 100,
      total: 400,
      detail: null,
      doneBytes: null,
      totalBytes: null,
    });
  });

  it("defaults missing counts to null", () => {
    const result = parseProgress(
      JSON.stringify({ kind: "progress", stage: 1, stageCount: 1, label: "x" }),
    );
    expect(result?.done).toBeNull();
    expect(result?.total).toBeNull();
  });

  it("parses vacuum detail fields", () => {
    const result = parseProgress(
      JSON.stringify({
        kind: "progress",
        stage: 1,
        stageCount: 1,
        label: "Compacting",
        detail: "SQLite is rewriting the database file.",
        doneBytes: 100,
        totalBytes: 200,
      }),
    );
    expect(result?.detail).toBe("SQLite is rewriting the database file.");
    expect(result?.doneBytes).toBe(100);
    expect(result?.totalBytes).toBe(200);
  });
});
