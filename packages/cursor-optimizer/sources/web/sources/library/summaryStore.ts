import { createSignal } from "solid-js";

import { callBackend } from "./ipc";
import { operationState } from "./operationStore";

import type { QuickSummary } from "./types";

const [summary, setSummary] = createSignal<QuickSummary | null>(null);

export const summaryState = {
  summary,
};

export async function loadQuickSummary(force = false): Promise<void> {
  if (!force && summary() !== null) {
    return;
  }
  if (operationState.readBlocked()) {
    return;
  }
  try {
    setSummary(await callBackend<QuickSummary>("quickSummary"));
  } catch (caught) {
    void caught;
  }
}
