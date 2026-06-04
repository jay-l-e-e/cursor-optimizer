import { Show } from "solid-js";

import { operationState } from "../library/operationStore";
import { parseProgress } from "../library/progress";
import TruncatedText from "./TruncatedText";

export default function StatusBar() {
  const active = () =>
    operationState.activityProgress() !== null ||
    operationState.status().activeWrite !== null ||
    operationState.status().activeReads.length > 0;

  const resolvedProgress = () => {
    const foreground = operationState.activityProgress();
    if (foreground !== null) {
      return foreground;
    }
    const write = operationState.status().activeWrite;
    if (write) {
      return parseProgress(write.progress);
    }
    const reads = operationState.status().activeReads;
    if (reads.length > 0) {
      return parseProgress(reads[0].progress);
    }
    return null;
  };

  const percent = () => {
    const progress = resolvedProgress();
    if (progress && progress.total != null && progress.total > 0 && progress.done != null) {
      return Math.min(100, Math.round((progress.done / progress.total) * 100));
    }
    return null;
  };

  const label = () => {
    const progress = resolvedProgress();
    if (progress?.label) {
      return progress.label;
    }
    if (operationState.activityProgress() !== null) {
      return operationState.activityTitle() || "Working";
    }
    const reads = operationState.status().activeReads;
    if (reads.length > 0) {
      return reads[0].title || "Working";
    }
    const write = operationState.status().activeWrite;
    if (write) {
      return write.title || "Working";
    }
    return "Working";
  };

  return (
    <Show when={active()}>
      <div class="status-bar-appear relative shrink-0 border-t border-line bg-surface">
        <div class="absolute inset-x-0 top-0 h-px overflow-hidden bg-subtle">
          <Show
            when={percent() !== null}
            fallback={
              <div class="h-full w-full">
                <div class="h-full animate-[shimmer_1.5s_ease-in-out_infinite] bg-accent/60" />
              </div>
            }
          >
            <div
              class="h-full bg-accent transition-[width] duration-150 ease-out"
              style={{ width: `${percent()}%` }}
            />
          </Show>
        </div>
        <div class="flex items-center px-3 py-[3px]">
          <span
            class="flex min-w-0 items-center gap-1.5 text-[10px] text-muted"
            style={{ "line-height": "14px" }}
          >
            <i
              class="bi bi-arrow-repeat animate-spin text-[9px] text-accent"
              style={{ "line-height": "14px" }}
            />
            <TruncatedText text={label()} placement="above" />
            <Show when={percent() !== null}>
              <span class="shrink-0 tabular-nums text-faint">{percent()}%</span>
            </Show>
          </span>
        </div>
      </div>
    </Show>
  );
}
