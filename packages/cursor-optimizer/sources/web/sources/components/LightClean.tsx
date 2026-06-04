import { onMount, Show } from "solid-js";

import { confirmAction } from "../library/confirmStore";
import {
  lightCleanState,
  resetLightCleanAnalysis,
  scanLightClean,
} from "../library/lightCleanStore";
import { operationState, runTrackedOperation } from "../library/operationStore";
import { describeStorageRisk, loadStorageEstimate } from "../library/storageEstimate";
import { loadQuickSummary, summaryState } from "../library/summaryStore";
import SavingsBar from "./SavingsBar";
import ScanProgress from "./ScanProgress";

import type { LightCleanResult } from "../library/types";

export default function LightClean(props: { cursorRunning: boolean }) {
  const summary = summaryState.summary;
  onMount(() => {
    void loadQuickSummary();
  });

  const clean = async () => {
    const estimate = await loadStorageEstimate();
    const confirmed = await confirmAction({
      title: "Run a light clean?",
      message: `This compacts Cursor storage without deleting chat payloads. Make sure Cursor is closed. ${describeStorageRisk(
        estimate,
        estimate?.databaseBytes ?? null,
      )}`,
      confirmLabel: "Clean now",
    });
    if (!confirmed) {
      return;
    }
    const data = await runTrackedOperation<LightCleanResult>({
      action: "lightCleanRun",
      progressTitle: "Cleaning up",
      resultRows: (result) => [
        { label: "Freed", value: result.reclaimedHuman },
        { label: "New size", value: result.afterHuman },
      ],
    });
    if (data !== null) {
      resetLightCleanAnalysis();
      void loadQuickSummary(true);
    }
  };

  return (
    <div class="fade-rise">
      <div class="min-w-0 rounded-2xl border border-line bg-surface p-6 max-[560px]:p-4">
        <h3 class="text-lg font-semibold tracking-tight text-ink max-[560px]:text-base">
          Light clean
        </h3>
        <p class="mt-1.5 text-sm text-muted">
          Compacts Cursor storage without deleting conversation data.
        </p>

        <Show when={!lightCleanState.analysis() && !lightCleanState.scanning()}>
          <button
            type="button"
            onClick={() => {
              void scanLightClean();
            }}
            disabled={operationState.readBlocked()}
            class="mt-5 w-full rounded-full bg-ink px-5 py-3 text-sm font-semibold text-surface transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
          >
            Estimate compaction
          </button>
        </Show>

        <Show when={lightCleanState.scanning()}>
          <div class="mt-5">
            <ScanProgress title="Scanning" progress={lightCleanState.progress()} />
          </div>
        </Show>

        <Show when={lightCleanState.analysis()}>
          {(currentAnalysis) => (
            <>
              <div class="mt-5 min-w-0 rounded-xl border border-line bg-canvas p-5 max-[560px]:p-4">
                <SavingsBar
                  reclaimBytes={currentAnalysis().estimatedReclaimBytes}
                  totalBytes={summary()?.totalBytes ?? 0}
                  reclaimHuman={currentAnalysis().estimatedReclaimHuman}
                  totalHuman={summary()?.totalHuman ?? ""}
                  atLeast
                />
                <div class="mt-4 text-sm text-muted max-[560px]:text-xs">
                  Estimated savings come from storage compaction. Chat payloads are preserved.
                </div>
              </div>

              <button
                type="button"
                onClick={clean}
                disabled={props.cursorRunning || operationState.writeBlocked()}
                class="mt-4 w-full rounded-full bg-accent px-5 py-3 text-sm font-semibold text-surface transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-50"
              >
                {props.cursorRunning
                  ? "Close Cursor first"
                  : operationState.writeBlocked()
                    ? "Task in progress"
                    : "Clean now"}
              </button>
            </>
          )}
        </Show>

        <Show when={lightCleanState.error()}>
          <div class="mt-4 rounded-xl border border-danger/30 bg-danger/5 p-3 text-sm text-danger">
            <i class="bi bi-exclamation-circle" /> {lightCleanState.error()}
          </div>
        </Show>
      </div>
    </div>
  );
}
