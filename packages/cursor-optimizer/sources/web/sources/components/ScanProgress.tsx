import { Show } from "solid-js";

import { formatNumber } from "../library/format";

import type { Progress } from "../library/progress";

export default function ScanProgress(props: { title: string; progress: Progress }) {
  const percent = () => {
    const { done, total } = props.progress;
    if (total != null && total > 0 && done != null) {
      return Math.min(100, Math.round((done / total) * 100));
    }
    return null;
  };

  return (
    <div class="fade-rise min-w-0 rounded-xl border border-line bg-surface p-4 max-[560px]:p-3">
      <div class="flex items-center justify-between gap-3 text-sm font-semibold text-ink">
        <span class="flex min-w-0 items-center gap-2">
          <i class="bi bi-arrow-repeat inline-block animate-spin" /> {props.title}
        </span>
        <Show when={props.progress.stageCount > 0}>
          <span class="shrink-0 text-xs font-normal text-muted">
            Step {props.progress.stage} of {props.progress.stageCount}
          </span>
        </Show>
      </div>

      <Show
        when={percent() !== null}
        fallback={
          <div class="scan-track mt-3">
            <div class="scan-bar" />
          </div>
        }
      >
        <div class="mt-3 h-1.5 w-full overflow-hidden rounded-full bg-subtle">
          <div class="bar-grow h-full rounded-full bg-accent" style={{ width: `${percent()}%` }} />
        </div>
      </Show>

      <div class="mt-2 flex items-center justify-between gap-3 text-xs text-muted max-[560px]:flex-wrap">
        <span class="min-w-0 truncate">{props.progress.label || "Working..."}</span>
        <Show when={props.progress.total != null || props.progress.done != null}>
          <span class="shrink-0">
            <Show
              when={props.progress.total != null}
              fallback={formatNumber(props.progress.done ?? 0)}
            >
              {formatNumber(props.progress.done ?? 0)} / {formatNumber(props.progress.total ?? 0)}
            </Show>
          </span>
        </Show>
      </div>
      <Show when={props.progress.detail}>
        <p class="mt-1 text-xs leading-5 text-faint">{props.progress.detail}</p>
      </Show>
      <Show when={props.progress.totalBytes != null && props.progress.totalBytes > 0}>
        <div class="mt-1 text-xs text-faint">
          {Math.min(
            100,
            Math.round(((props.progress.doneBytes ?? 0) / (props.progress.totalBytes ?? 1)) * 100),
          )}
          % processed
        </div>
      </Show>
    </div>
  );
}
