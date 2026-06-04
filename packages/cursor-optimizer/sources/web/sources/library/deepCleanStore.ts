import { createSignal } from "solid-js";

import { cancelRequest, startRequest } from "./ipc";
import { operationState, trackActivity } from "./operationStore";
import { idleProgress, type Progress } from "./progress";

import type { DeepCleanAnalysis } from "./types";

const [analysis, setAnalysis] = createSignal<DeepCleanAnalysis | null>(null);
const [progress, setProgress] = createSignal<Progress>(idleProgress);
const [analyzing, setAnalyzing] = createSignal(false);
const [error, setError] = createSignal("");
const [selectedDays, setSelectedDays] = createSignal(30);
const [analysisDays, setAnalysisDays] = createSignal<number | null>(null);

let activeId: number | null = null;
let analyzedDays: number | null = null;

export const deepCleanState = {
  analysis,
  progress,
  analyzing,
  error,
  selectedDays,
  setSelectedDays,
  analysisDays,
};

export function resetDeepCleanResult(): void {
  setAnalysis(null);
  setAnalysisDays(null);
  analyzedDays = null;
}

export async function analyzeDeepClean(days: number): Promise<void> {
  if (analyzedDays === days && analyzing()) {
    return;
  }
  if (operationState.readBlocked()) {
    setError("Another task is already running.");
    return;
  }
  if (activeId !== null) {
    cancelRequest(activeId);
  }
  analyzedDays = days;
  setError("");
  setProgress(idleProgress);
  setAnalyzing(true);
  const activity = trackActivity("Calculating savings");
  const request = startRequest<DeepCleanAnalysis>("deepCleanAnalyze", { days }, (next) => {
    setProgress(next);
    activity.report(next);
  });
  activeId = request.id;
  try {
    const result = await request.promise;
    if (activeId === request.id) {
      setAnalysis(result);
      setAnalysisDays(days);
    }
  } catch (caught) {
    if (activeId === request.id) {
      const message = caught instanceof Error ? caught.message : String(caught);
      if (message !== "cancelled") {
        setError(message);
      }
      analyzedDays = null;
      setAnalysisDays(null);
    }
  } finally {
    activity.finish();
    if (activeId === request.id) {
      setAnalyzing(false);
      activeId = null;
    }
  }
}
