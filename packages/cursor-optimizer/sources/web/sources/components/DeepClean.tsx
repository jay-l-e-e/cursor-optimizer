import { For, onMount, Show } from "solid-js";

import { confirmAction } from "../library/confirmStore";
import { analyzeDeepClean, deepCleanState, resetDeepCleanResult } from "../library/deepCleanStore";
import { formatNumber } from "../library/format";
import { operationState, runTrackedOperation } from "../library/operationStore";
import { describeStorageRisk, loadStorageEstimate } from "../library/storageEstimate";
import { loadQuickSummary, summaryState } from "../library/summaryStore";
import { offerPreBackup } from "../library/toolsStore";
import ReclaimBreakdown from "./ReclaimBreakdown";
import SavingsBar from "./SavingsBar";
import ScanProgress from "./ScanProgress";

import type { DeepCleanResult, VacuumResult } from "../library/types";

const PRESETS = [30, 60, 90, 180];

export default function DeepClean(props: { cursorRunning: boolean }) {
  const summary = summaryState.summary;
  onMount(() => {
    void loadQuickSummary();
  });

  const days = deepCleanState.selectedDays;
  const setDays = deepCleanState.setSelectedDays;

  const currentAnalysis = () =>
    deepCleanState.analysisDays() === days() ? deepCleanState.analysis() : null;
  const matchingEntries = () => currentAnalysis()?.matchingEntries;
  const compactionOnly = () =>
    matchingEntries() === 0 && (currentAnalysis()?.compactionReclaimBytes ?? 0) > 0;
  const nothingToDo = () =>
    currentAnalysis() !== null &&
    matchingEntries() === 0 &&
    (currentAnalysis()?.compactionReclaimBytes ?? 0) === 0;

  const analyze = () => {
    void analyzeDeepClean(days());
  };

  const remove = async () => {
    const chosen = days();
    const isCompactionOnly = compactionOnly();
    if (!isCompactionOnly) {
      const shouldContinue = await offerPreBackup();
      if (!shouldContinue) {
        return;
      }
    }
    const estimate = await loadStorageEstimate();
    const confirmed = await confirmAction(
      isCompactionOnly
        ? {
            title: "Compact storage?",
            message: `No old conversations to delete, but storage can be compacted to free space. ${describeStorageRisk(
              estimate,
              estimate?.databaseBytes ?? null,
            )}`,
            confirmLabel: "Compact now",
          }
        : {
            title: "Delete old conversations?",
            message: `This permanently deletes conversations older than ${chosen} days, then compacts storage. This cannot be undone. ${describeStorageRisk(
              estimate,
              estimate?.databaseBytes ?? null,
            )}`,
            confirmLabel: "Delete permanently",
            danger: true,
          },
    );
    if (!confirmed) {
      return;
    }
    if (isCompactionOnly) {
      const data = await runTrackedOperation<VacuumResult>({
        action: "toolVacuum",
        progressTitle: "Compacting",
        resultRows: (result) => [
          { label: "Before", value: result.beforeHuman },
          { label: "After", value: result.afterHuman },
          { label: "Freed", value: result.reclaimedHuman },
        ],
      });
      if (data !== null) {
        resetDeepCleanResult();
        void loadQuickSummary(true);
      }
    } else {
      const data = await runTrackedOperation<DeepCleanResult>({
        action: "deepCleanRun",
        params: { days: chosen },
        progressTitle: "Deleting",
        resultRows: (result) => [
          { label: "Deleted conversations", value: formatNumber(result.deletedConversationRows) },
          { label: "Freed", value: result.reclaimedHuman },
          { label: "New size", value: result.afterHuman },
        ],
      });
      if (data !== null) {
        resetDeepCleanResult();
        void loadQuickSummary(true);
      }
    }
  };

  return (
    <div class="fade-rise">
      <div class="min-w-0 rounded-2xl border border-line bg-surface p-6 max-[560px]:p-4">
        <h3 class="text-lg font-semibold tracking-tight text-ink max-[560px]:text-base">
          Deep clean
        </h3>
        <p class="mt-1.5 text-sm text-muted">
          Permanently deletes conversations older than your chosen window. Recent work stays.
        </p>

        <div class="mt-5 flex flex-wrap items-center gap-2">
          <For each={PRESETS}>
            {(preset) => (
              <button
                type="button"
                onClick={() => {
                  setDays(preset);
                }}
                class="rounded-full border px-4 py-1.5 text-sm font-medium transition max-[560px]:px-3 max-[560px]:text-xs"
                classList={{
                  "border-accent bg-accent-soft text-accent": days() === preset,
                  "border-line text-muted hover:text-ink": days() !== preset,
                }}
              >
                {preset} days
              </button>
            )}
          </For>
          <div class="flex items-center gap-2 rounded-full border border-line px-3 py-1 max-[560px]:text-xs">
            <input
              type="number"
              min="1"
              value={days()}
              onInput={(event) => {
                const next = Number(event.currentTarget.value);
                if (next >= 1) {
                  setDays(next);
                }
              }}
              class="w-16 bg-transparent text-sm outline-none"
            />
            <span class="text-xs text-faint">days</span>
          </div>
        </div>

        <button
          type="button"
          onClick={analyze}
          disabled={deepCleanState.analyzing() || operationState.readBlocked()}
          class="mt-4 w-full rounded-full bg-ink px-5 py-3 text-sm font-semibold text-surface transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Show when={deepCleanState.analyzing()} fallback="Analyze">
            Analyzing...
          </Show>
        </button>

        <Show when={deepCleanState.analyzing()}>
          <div class="mt-5">
            <ScanProgress title="Calculating savings" progress={deepCleanState.progress()} />
          </div>
        </Show>

        <Show
          when={
            !deepCleanState.analyzing() &&
            deepCleanState.analysis() !== null &&
            deepCleanState.analysisDays() !== days()
          }
        >
          <div class="mt-4 rounded-xl border border-line bg-canvas p-4 text-sm text-muted max-[560px]:text-xs">
            Filter changed. Re-analyze to update the estimate.
          </div>
        </Show>

        <Show when={!deepCleanState.analyzing() && currentAnalysis()}>
          {(analysis) => (
            <div class="mt-5 min-w-0 rounded-xl border border-line bg-canvas p-5 max-[560px]:p-4">
              <SavingsBar
                reclaimBytes={analysis().totalReclaimBytes}
                totalBytes={summary()?.totalBytes ?? 0}
                reclaimHuman={analysis().totalReclaimHuman}
                totalHuman={summary()?.totalHuman ?? ""}
                atLeast
              />
              <ReclaimBreakdown
                dataLabel="old conversations"
                dataHuman={analysis().estimatedHuman}
                compactionHuman={analysis().compactionReclaimHuman}
              />
              <div class="mt-4 text-sm text-muted max-[560px]:text-xs">
                {formatNumber(analysis().matchingEntries)} conversations older than {days()} days.
              </div>
            </div>
          )}
        </Show>

        <Show when={deepCleanState.error()}>
          <div class="mt-4 rounded-xl border border-danger/30 bg-danger/5 p-3 text-sm text-danger">
            <i class="bi bi-exclamation-circle" /> {deepCleanState.error()}
          </div>
        </Show>

        <button
          type="button"
          onClick={remove}
          disabled={
            props.cursorRunning ||
            operationState.writeBlocked() ||
            matchingEntries() === undefined ||
            nothingToDo()
          }
          class="mt-4 w-full rounded-full bg-accent px-5 py-3 text-sm font-semibold text-surface transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Show
            when={props.cursorRunning}
            fallback={
              <Show
                when={operationState.writeBlocked()}
                fallback={
                  <Show
                    when={nothingToDo()}
                    fallback={compactionOnly() ? "Compact now" : "Delete permanently"}
                  >
                    Nothing to clean
                  </Show>
                }
              >
                Task in progress
              </Show>
            }
          >
            Close Cursor first
          </Show>
        </button>
      </div>
    </div>
  );
}
