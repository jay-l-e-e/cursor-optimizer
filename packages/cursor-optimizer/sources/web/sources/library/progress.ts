export type Progress = {
  active: boolean;
  stage: number;
  stageCount: number;
  label: string;
  done: number | null;
  total: number | null;
  detail: string | null;
  doneBytes: number | null;
  totalBytes: number | null;
};

export const idleProgress: Progress = {
  active: false,
  stage: 0,
  stageCount: 0,
  label: "",
  done: null,
  total: null,
  detail: null,
  doneBytes: null,
  totalBytes: null,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function optionalNumber(value: unknown): number | null {
  return value == null ? null : Number(value);
}

export function parseProgress(text: string): Progress | null {
  if (!text.startsWith("{")) {
    return null;
  }
  try {
    const decoded: unknown = JSON.parse(text);
    if (!isRecord(decoded)) {
      return null;
    }
    if (decoded.kind !== "progress") {
      return null;
    }
    return {
      active: true,
      stage: Number(decoded.stage) || 0,
      stageCount: Number(decoded.stageCount) || 0,
      label: String(decoded.label ?? ""),
      done: optionalNumber(decoded.done),
      total: optionalNumber(decoded.total),
      detail: decoded.detail == null ? null : String(decoded.detail),
      doneBytes: optionalNumber(decoded.doneBytes),
      totalBytes: optionalNumber(decoded.totalBytes),
    };
  } catch (parseError) {
    console.error("failed to parse progress payload", parseError);
    return null;
  }
}
