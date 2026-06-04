import { describe, expect, it } from "vitest";

import { formatNumber, friendlyPrefix, percentOf } from "./format";

describe("formatNumber", () => {
  it("groups thousands", () => {
    expect(formatNumber(1234567)).toBe("1,234,567");
  });
});

describe("friendlyPrefix", () => {
  it("maps known prefixes", () => {
    expect(friendlyPrefix("composerData")).toBe("Chat sessions");
  });

  it("humanizes unknown camelCase", () => {
    expect(friendlyPrefix("codeBlockDiff")).toBe("Code block diff");
  });

  it("labels empty input as Other", () => {
    expect(friendlyPrefix("")).toBe("Other");
  });
});

describe("percentOf", () => {
  it("computes a rounded percentage", () => {
    expect(percentOf(1, 4)).toBe(25);
  });

  it("guards against a zero whole", () => {
    expect(percentOf(5, 0)).toBe(0);
  });

  it("caps the result at 100", () => {
    expect(percentOf(10, 5)).toBe(100);
  });
});
