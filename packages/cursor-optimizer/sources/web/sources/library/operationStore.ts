import { createSignal } from "solid-js";

import { callBackend, startRequest } from "./ipc";
import { operationBlocked } from "./operationRules";
import { idleProgress, type Progress, parseProgress } from "./progress";

import type { StatRow } from "./types";

const OVERVIEW_CACHE_KEY = "cursor-optimizer:overview-cache";

export type OperationSnapshot = {
  requestId: number;
  action: string;
  title: string;
  access: "neutral" | "read" | "write";
  startedMillis: number;
  progress: string;
  elapsedSeconds: number;
};

export type OperationJournal = {
  action?: string;
  title?: string;
  progress?: string;
  updatedMillis?: number;
  databaseBytes?: number;
  writeAheadLogBytes?: number;
};

export type OperationStatus = {
  recoveryPending: boolean;
  closeBlocked: boolean;
  activeWrite: OperationSnapshot | null;
  activeReads: OperationSnapshot[];
  journal: OperationJournal | null;
};

type OperationRun<ResultType> = {
  action: string;
  access?: "read" | "write";
  params?: Record<string, unknown>;
  progressTitle: string;
  resultRows: (result: ResultType) => StatRow[];
};

const [status, setStatus] = createSignal<OperationStatus>({
  recoveryPending: false,
  closeBlocked: false,
  activeWrite: null,
  activeReads: [],
  journal: null,
});
const [progress, setProgress] = createSignal<Progress>(idleProgress);
const [progressTitle, setProgressTitle] = createSignal("");
const [rows, setRows] = createSignal<StatRow[] | null>(null);
const [error, setError] = createSignal("");
const [busy, setBusy] = createSignal(false);
const [elapsedSeconds, setElapsedSeconds] = createSignal(0);
const [currentAccess, setCurrentAccess] = createSignal<"read" | "write">("write");
const [activityProgress, setActivityProgress] = createSignal<Progress | null>(null);
const [activityTitle, setActivityTitle] = createSignal("");

let timer: ReturnType<typeof setInterval> | undefined;
let polling: ReturnType<typeof setInterval> | undefined;
let lastWriteCompletedAt = 0;

const WRITE_COOLDOWN_MILLIS = 5000;

export function recentlyCompletedWrite(): boolean {
  return Date.now() - lastWriteCompletedAt < WRITE_COOLDOWN_MILLIS;
}

export const operationState = {
  status,
  progress,
  progressTitle,
  rows,
  error,
  busy,
  elapsedSeconds,
  activityProgress,
  activityTitle,
  writeBlocked: () => operationBlocked("write", status(), busy()),
  readBlocked: () => operationBlocked("read", status(), busy()),
  closeBlocked: () => (busy() && currentAccess() === "write") || status().closeBlocked,
};

export type ActivityTracker = {
  report: (next: Progress) => void;
  finish: () => void;
};

let activityToken = 0;

export function trackActivity(title: string): ActivityTracker {
  activityToken += 1;
  const token = activityToken;
  setActivityTitle(title);
  setActivityProgress(idleProgress);
  let lastProgress: Progress = idleProgress;
  return {
    report: (next: Progress) => {
      if (token !== activityToken) {
        return;
      }
      lastProgress = next;
      setActivityProgress(next);
    },
    finish: () => {
      if (token !== activityToken) {
        return;
      }
      if (lastProgress.total != null && lastProgress.total > 0 && lastProgress.done != null) {
        setActivityProgress({ ...lastProgress, done: lastProgress.total });
        window.setTimeout(() => {
          if (token === activityToken) {
            setActivityProgress(null);
          }
        }, 320);
      } else {
        setActivityProgress(null);
      }
    },
  };
}

export async function refreshOperationStatus(): Promise<void> {
  try {
    const nextStatus = await callBackend<OperationStatus>("operationStatus");
    setStatus(nextStatus);
    const activeWrite = nextStatus.activeWrite;
    if (!busy() && activeWrite) {
      setProgressTitle(activeWrite.title);
      setElapsedSeconds(activeWrite.elapsedSeconds);
      const parsed = parseProgress(activeWrite.progress);
      setProgress(parsed ?? { ...idleProgress, active: true, label: activeWrite.title });
    }
    if (!busy() && !activeWrite) {
      setElapsedSeconds(0);
    }
  } catch (caught) {
    setError(caught instanceof Error ? caught.message : String(caught));
  }
}

export function startOperationStatusPolling(): void {
  if (polling) {
    return;
  }
  void refreshOperationStatus();
  polling = setInterval(() => void refreshOperationStatus(), 1000);
}

export async function runTrackedOperation<ResultType>(
  operation: OperationRun<ResultType>,
): Promise<ResultType | null> {
  const access = operation.access ?? "write";
  if (operationBlocked(access, status(), busy())) {
    setError("Another task is already running.");
    return null;
  }
  setBusy(true);
  setCurrentAccess(access);
  setProgressTitle(operation.progressTitle);
  setProgress(idleProgress);
  setRows(null);
  setError("");
  setElapsedSeconds(0);
  if (timer) {
    clearInterval(timer);
  }
  timer = setInterval(() => setElapsedSeconds(elapsedSeconds() + 1), 1000);
  const activity = trackActivity(operation.progressTitle);
  const request = startRequest<ResultType>(operation.action, operation.params ?? {}, (next) => {
    setProgress(next);
    activity.report(next);
  });
  void refreshOperationStatus();
  try {
    const result = await request.promise;
    setRows(operation.resultRows(result));
    if (access === "write") {
      try {
        localStorage.removeItem(OVERVIEW_CACHE_KEY);
      } catch {
        /* ignore */
      }
    }
    return result;
  } catch (caught) {
    setError(caught instanceof Error ? caught.message : String(caught));
    return null;
  } finally {
    activity.finish();
    if (timer) {
      clearInterval(timer);
      timer = undefined;
    }
    if (access === "write") {
      lastWriteCompletedAt = Date.now();
    }
    setBusy(false);
    await refreshOperationStatus();
  }
}

export function clearResult(): void {
  setRows(null);
  setError("");
}

export async function recoverOperations(): Promise<void> {
  await runTrackedOperation({
    action: "recoverOperations",
    progressTitle: "Recovering",
    resultRows: () => [{ label: "Recovery", value: "Complete" }],
  });
}
