import { createEffect, createMemo, For, on, onMount, Show } from "solid-js";

import { confirmAction } from "../library/confirmStore";
import { formatNumber, friendlyPrefix, percentOf } from "../library/format";
import { operationState, runTrackedOperation } from "../library/operationStore";
import { loadOverview, overviewState } from "../library/overviewStore";
import { describeStorageRisk, loadStorageEstimate } from "../library/storageEstimate";
import ScanProgress from "./ScanProgress";
import TruncatedText from "./TruncatedText";

import type { VacuumResult } from "../library/types";

const SEGMENT_COLORS = ["bg-peach", "bg-mint", "bg-sky", "bg-lavender", "bg-gold"];

function Tile(props: { label: string; value: string; hint?: string }) {
  return (
    <div class="min-w-0 rounded-xl border border-line bg-surface p-4 max-[560px]:p-3">
      <div class="text-xs text-muted">{props.label}</div>
      <div class="mt-1">
        <TruncatedText
          text={props.value}
          class="text-2xl font-bold tracking-tight text-ink max-[560px]:text-xl"
          placement="above"
        />
      </div>
      <Show when={props.hint}>
        <div class="mt-0.5 text-xs text-faint">{props.hint}</div>
      </Show>
    </div>
  );
}

function ReclaimableTile(props: { cursorRunning: boolean }) {
  const reclaimableBytes = () => overviewState.details()?.storage.reclaimableBytes ?? 0;
  const reclaimableLabel = () => {
    const human = overviewState.details()?.storage.reclaimableHuman;
    if (human == null) {
      return "—";
    }
    return reclaimableBytes() > 0 ? `at least ${human}` : human;
  };

  const compact = async () => {
    const estimate = await loadStorageEstimate();
    const confirmed = await confirmAction({
      title: "Compact storage?",
      message: `This reorganizes storage to free unused space. Make sure Cursor is closed. ${describeStorageRisk(
        estimate,
        estimate?.databaseBytes ?? null,
      )}`,
      confirmLabel: "Compact now",
    });
    if (!confirmed) {
      return;
    }
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
      await loadOverview(true);
    }
  };

  return (
    <div class="min-w-0 rounded-xl border border-line bg-surface p-4 max-[560px]:p-3">
      <div class="text-xs text-muted">Free space</div>
      <div class="mt-1">
        <TruncatedText
          text={reclaimableLabel()}
          class="text-2xl font-bold tracking-tight text-ink max-[560px]:text-xl"
          placement="above"
        />
      </div>
      <div class="mt-0.5 text-xs text-faint">recoverable by compacting, often more</div>
      <button
        type="button"
        onClick={() => void compact()}
        disabled={props.cursorRunning || operationState.writeBlocked() || reclaimableBytes() === 0}
        class="mt-3 w-full rounded-full bg-accent px-3 py-1.5 text-xs font-semibold text-surface transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-50"
      >
        <Show
          when={props.cursorRunning}
          fallback={operationState.writeBlocked() ? "Task in progress" : "Compact now"}
        >
          Close Cursor first
        </Show>
      </button>
    </div>
  );
}

export default function Overview(props: { refreshToken: number; cursorRunning: boolean }) {
  onMount(() => void loadOverview(false));
  createEffect(
    on(
      () => props.refreshToken,
      (token) => {
        if (token > 0) {
          void loadOverview(true);
        }
      },
      { defer: true },
    ),
  );

  const segments = createMemo(() => {
    const data = overviewState.details();
    if (!data) {
      return [];
    }
    const top = data.keyPrefixes.slice(0, 6);
    const total = top.reduce((sum, entry) => sum + entry.bytes, 0);
    return top.map((entry) => ({
      label: friendlyPrefix(entry.prefix),
      human: entry.human,
      rows: entry.rowCount,
      share: percentOf(entry.bytes, total),
    }));
  });

  return (
    <div class="fade-rise space-y-4">
      <div class="grid grid-cols-1 gap-4 min-[560px]:grid-cols-3 max-[560px]:gap-3">
        <Tile label="Total storage" value={overviewState.summary()?.totalHuman ?? "..."} />
        <Tile
          label="Database"
          value={overviewState.summary()?.databaseHuman ?? "..."}
          hint="Cursor storage file"
        />
        <ReclaimableTile cursorRunning={props.cursorRunning} />
      </div>

      <div class="min-w-0 rounded-xl border border-line bg-surface p-5 max-[560px]:p-4">
        <div class="mb-4 flex items-center justify-between gap-3">
          <h3 class="text-sm font-semibold text-ink">Storage breakdown</h3>
          <Show when={overviewState.details()}>
            {(data) => (
              <span class="shrink-0 text-xs text-muted">
                {formatNumber(data().agentBlobs.count)} stored items
              </span>
            )}
          </Show>
        </div>

        <Show when={overviewState.error()}>
          <div class="rounded-xl border border-danger/30 bg-danger/5 p-3 text-sm text-danger">
            <i class="bi bi-exclamation-circle" /> {overviewState.error()}
          </div>
        </Show>

        <Show
          when={!overviewState.error() && (overviewState.loading() || !overviewState.details())}
        >
          <ScanProgress title="Analyzing" progress={overviewState.progress()} />
        </Show>

        <Show when={!overviewState.loading() && overviewState.details()}>
          <div class="flex h-3.5 w-full overflow-hidden rounded-full bg-subtle">
            <For each={segments()}>
              {(segment, index) => (
                <div
                  class={`${SEGMENT_COLORS[index() % SEGMENT_COLORS.length]} bar-grow h-full`}
                  style={{ width: `${segment.share}%` }}
                />
              )}
            </For>
          </div>
          <div class="mt-4 grid grid-cols-1 gap-x-10 gap-y-1.5 min-[700px]:grid-cols-2">
            <For each={segments()}>
              {(segment, index) => (
                <div class="flex min-w-0 items-center justify-between gap-4 py-1 text-sm max-[560px]:items-start">
                  <span class="flex min-w-0 items-center gap-2 text-ink">
                    <span
                      class={`${SEGMENT_COLORS[index() % SEGMENT_COLORS.length]} inline-block h-2.5 w-2.5 shrink-0 rounded-full`}
                    />
                    <TruncatedText text={segment.label} placement="above" />
                  </span>
                  <span class="shrink-0 text-right text-muted">
                    {segment.human} · {formatNumber(segment.rows)} items
                  </span>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
}
