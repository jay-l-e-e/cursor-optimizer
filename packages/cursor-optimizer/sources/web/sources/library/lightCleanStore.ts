import { createSignal } from "solid-js";

import { startRequest } from "./ipc";
import { operationState, trackActivity } from "./operationStore";
import { idleProgress, type Progress } from "./progress";

import type { LightCleanAnalysis } from "./types";

const [analysis, setAnalysis] = createSignal<LightCleanAnalysis | null>(null);
const [progress, setProgress] = createSignal<Progress>(idleProgress);
const [scanning, setScanning] = createSignal(false);
const [error, setError] = createSignal("");

export const lightCleanState = {
  analysis,
  progress,
  scanning,
  error,
};

export function resetLightCleanAnalysis(): void {
  setAnalysis(null);
}

export async function scanLightClean(): Promise<void> {
  if (analysis() !== null || scanning()) {
    return;
  }
  if (operationState.readBlocked()) {
    setError("Another task is already running.");
    return;
  }
  setError("");
  setProgress(idleProgress);
  setScanning(true);
  const activity = trackActivity("Scanning");
  try {
    setAnalysis(
      await startRequest<LightCleanAnalysis>("lightCleanAnalyze", {}, (next) => {
        setProgress(next);
        activity.report(next);
      }).promise,
    );
  } catch (caught) {
    setError(caught instanceof Error ? caught.message : String(caught));
  } finally {
    activity.finish();
    setScanning(false);
  }
}
