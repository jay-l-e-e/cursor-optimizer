import { For, Show } from "solid-js";

import { operationState } from "../library/operationStore";
import { executeTool, miniTools } from "../library/toolsStore";
import TruncatedText from "./TruncatedText";

export default function Tools(props: { cursorRunning: boolean; walPresent: boolean }) {
  return (
    <div class="fade-rise">
      <div class="grid grid-cols-1 gap-3 min-[700px]:grid-cols-2">
        <For each={miniTools}>
          {(tool) => {
            const nothingToFlush = () => tool.requiresWriteAheadLog === true && !props.walPresent;
            const blocked = () =>
              nothingToFlush() ||
              (tool.writes && props.cursorRunning) ||
              (tool.writes ? operationState.writeBlocked() : operationState.readBlocked());
            const blockedReason = () => {
              if (nothingToFlush()) {
                return "No pending writes to flush";
              }
              if (tool.writes && props.cursorRunning) {
                return "Close Cursor first";
              }
              return "Task in progress";
            };
            return (
              <button
                type="button"
                onClick={() => void executeTool(tool)}
                disabled={blocked()}
                class="min-w-0 rounded-xl border border-line bg-surface p-4 text-left transition hover:border-ink/20 disabled:cursor-not-allowed disabled:opacity-50 max-[560px]:p-3"
              >
                <div class="flex min-w-0 items-center gap-2 text-sm font-semibold text-ink">
                  <i class={`bi ${tool.icon} shrink-0 text-muted`} />
                  <TruncatedText text={tool.title} />
                </div>
                <p class="mt-1 text-xs text-muted">{tool.description}</p>
                <Show when={blocked()}>
                  <p class="mt-1 text-xs text-accent">{blockedReason()}</p>
                </Show>
              </button>
            );
          }}
        </For>
      </div>
    </div>
  );
}
