import { createSignal } from "solid-js";

import { startRequest } from "./ipc";
import { operationState, trackActivity } from "./operationStore";
import { idleProgress, type Progress } from "./progress";

import type { OverviewResult, QuickSummary } from "./types";

const CACHE_KEY = "cursor-optimizer:overview-cache";

type OverviewCache = {
  fingerprint: string;
  summary: QuickSummary;
  details: OverviewResult;
};

function summaryFingerprint(quickSummary: QuickSummary): string {
  return `${quickSummary.databaseBytes}:${quickSummary.totalBytes}`;
}

function readCache(): OverviewCache | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) {
      return null;
    }
    return JSON.parse(raw) as OverviewCache;
  } catch {
    return null;
  }
}

function writeCache(quickSummary: QuickSummary, overviewResult: OverviewResult): void {
  try {
    const entry: OverviewCache = {
      fingerprint: summaryFingerprint(quickSummary),
      summary: quickSummary,
      details: overviewResult,
    };
    localStorage.setItem(CACHE_KEY, JSON.stringify(entry));
  } catch {
    /* storage full or unavailable */
  }
}

const [summary, setSummary] = createSignal<QuickSummary | null>(null);
const [details, setDetails] = createSignal<OverviewResult | null>(null);
const [progress, setProgress] = createSignal<Progress>(idleProgress);
const [loading, setLoading] = createSignal(false);
const [error, setError] = createSignal("");

let loadedFingerprint: string | null = null;

export const overviewState = { summary, details, progress, loading, error };

export async function loadOverview(force: boolean): Promise<void> {
  if (loading()) {
    return;
  }
  if (operationState.readBlocked()) {
    return;
  }

  let quickSummary: QuickSummary | null = null;
  try {
    quickSummary = await startRequest<QuickSummary>("quickSummary").promise;
    setSummary(quickSummary);
  } catch {
    setSummary(null);
  }
  const fingerprint = quickSummary ? summaryFingerprint(quickSummary) : null;

  if (!force && fingerprint !== null) {
    if (loadedFingerprint === fingerprint && details() !== null) {
      return;
    }
    const cached = readCache();
    if (cached && cached.fingerprint === fingerprint) {
      setSummary(cached.summary);
      setDetails(cached.details);
      loadedFingerprint = fingerprint;
      return;
    }
  }

  setError("");
  setProgress(idleProgress);
  setLoading(true);
  const activity = trackActivity("Analyzing");
  const request = startRequest<OverviewResult>("overview", {}, (next) => {
    setProgress(next);
    activity.report(next);
  });
  try {
    const result = await request.promise;
    setDetails(result);
    loadedFingerprint = fingerprint;
    if (quickSummary) {
      writeCache(quickSummary, result);
    }
  } catch (caught) {
    setError(caught instanceof Error ? caught.message : String(caught));
  } finally {
    activity.finish();
    setLoading(false);
  }
}

export function invalidateOverviewCache(): void {
  loadedFingerprint = null;
  try {
    localStorage.removeItem(CACHE_KEY);
  } catch {
    /* ignore */
  }
}
