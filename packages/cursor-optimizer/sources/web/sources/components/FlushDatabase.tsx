import { confirmAction } from "../library/confirmStore";
import { operationState, runTrackedOperation } from "../library/operationStore";
import { describeStorageRisk, loadStorageEstimate } from "../library/storageEstimate";
import { loadQuickSummary } from "../library/summaryStore";
import { offerPreBackup } from "../library/toolsStore";

import type { FlushDatabaseResult } from "../library/types";

export default function FlushDatabase(props: { cursorRunning: boolean }) {
  const flush = async () => {
    const shouldContinue = await offerPreBackup();
    if (!shouldContinue) {
      return;
    }
    const estimate = await loadStorageEstimate();
    const confirmed = await confirmAction({
      title: "Flush database?",
      message: `This removes Cursor's cached chat content and diff history. Existing chats may no longer open or continue in Cursor. Use this only when you accept losing access to old chat contents. ${describeStorageRisk(
        estimate,
        estimate?.databaseBytes ?? null,
      )}`,
      confirmLabel: "Flush database",
      danger: true,
    });
    if (!confirmed) {
      return;
    }
    const data = await runTrackedOperation<FlushDatabaseResult>({
      action: "toolFlushDatabase",
      progressTitle: "Flushing database",
      resultRows: (result) => [
        { label: "Database before", value: result.beforeDatabaseHuman },
        { label: "Database after", value: result.afterDatabaseHuman },
        { label: "Log before", value: result.beforeWriteAheadLogHuman },
        { label: "Log after", value: result.afterWriteAheadLogHuman },
        { label: "Freed", value: result.reclaimedHuman },
      ],
    });
    if (data !== null) {
      void loadQuickSummary(true);
    }
  };

  return (
    <div class="fade-rise">
      <div class="min-w-0 rounded-2xl border border-line bg-surface p-6 max-[560px]:p-4">
        <h3 class="text-lg font-semibold tracking-tight text-ink max-[560px]:text-base">
          Flush database
        </h3>
        <p class="mt-1.5 text-sm text-muted">
          Removes local Cursor chat payload rows to reclaim space, then compacts storage.
        </p>
        <div class="mt-5 flex gap-3 rounded-xl border border-gold/40 bg-gold/10 p-4 text-sm leading-6 text-ink">
          <i class="bi bi-exclamation-triangle mt-0.5 text-accent" />
          <span>
            Existing chats may become impossible to open or continue. Close Cursor before running
            this.
          </span>
        </div>
        <button
          type="button"
          onClick={flush}
          disabled={props.cursorRunning || operationState.writeBlocked()}
          class="mt-4 w-full rounded-full bg-accent px-5 py-3 text-sm font-semibold text-surface transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-50"
        >
          {props.cursorRunning
            ? "Close Cursor first"
            : operationState.writeBlocked()
              ? "Task in progress"
              : "Flush database"}
        </button>
      </div>
    </div>
  );
}
