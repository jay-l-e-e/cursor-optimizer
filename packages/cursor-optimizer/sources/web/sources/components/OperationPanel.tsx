import { createSignal, For, Show } from "solid-js";

import { callBackend } from "../library/ipc";
import { clearResult, operationState, recoverOperations } from "../library/operationStore";
import ScanProgress from "./ScanProgress";
import TruncatedText from "./TruncatedText";

function pathFileName(fullPath: string): string {
  const parts = fullPath.split(/[\\/]/);
  return parts.at(-1) ?? fullPath;
}

function CopyButton(props: { text: string }) {
  const [copied, setCopied] = createSignal(false);
  const copy = () => {
    void navigator.clipboard.writeText(props.text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };
  return (
    <button
      type="button"
      onClick={copy}
      class="grid h-6 w-6 shrink-0 place-items-center rounded-full text-faint transition hover:bg-subtle hover:text-ink"
      aria-label="Copy path"
    >
      <i class={`bi ${copied() ? "bi-check-lg text-ok" : "bi-copy"}`} />
    </button>
  );
}

export default function OperationPanel(props: { cursorRunning: boolean }) {
  const recoveryTitle = () => operationState.status().journal?.title ?? "Recovery needed";
  const running = () => operationState.busy() || operationState.status().activeWrite !== null;
  const recoveryNeeded = () => operationState.status().recoveryPending && !running();
  const hasResult = () => !running() && operationState.rows() !== null;
  const hasError = () => operationState.error() !== "";
  const resultOnly = () => !running() && !recoveryNeeded() && (hasResult() || hasError());
  const visible = () => running() || recoveryNeeded() || hasResult() || hasError();
  const hasPathRows = () => (operationState.rows() ?? []).some((row) => row.kind === "path");
  const textRows = () => (operationState.rows() ?? []).filter((row) => row.kind !== "path");
  const pathRows = () => (operationState.rows() ?? []).filter((row) => row.kind === "path");

  return (
    <Show when={visible()}>
      <Show
        when={!resultOnly()}
        fallback={
          <div class="mb-4 min-w-0">
            <Show when={hasError()}>
              <div class="fade-rise flex items-start gap-2 rounded-xl border border-danger/30 bg-danger/5 p-3 text-sm text-danger">
                <i class="bi bi-exclamation-circle shrink-0 mt-0.5" />
                <span class="min-w-0 flex-1">{operationState.error()}</span>
                <button
                  type="button"
                  onClick={clearResult}
                  class="shrink-0 grid h-5 w-5 place-items-center rounded text-danger/60 transition hover:text-danger"
                  aria-label="Dismiss"
                >
                  <i class="bi bi-x-lg text-xs" />
                </button>
              </div>
            </Show>
            <Show when={hasResult() && !hasError()}>
              <Show
                when={hasPathRows()}
                fallback={
                  <div class="fade-rise flex items-center gap-2 rounded-xl border border-ok/30 bg-ok/5 px-3 py-2.5 text-sm">
                    <i class="bi bi-check-circle shrink-0 text-ok" />
                    <span class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
                      <For each={operationState.rows() ?? []}>
                        {(row) => (
                          <span class="flex items-center gap-1.5">
                            <span class="text-muted">{row.label}</span>
                            <span class="font-semibold text-ink">{row.value}</span>
                          </span>
                        )}
                      </For>
                    </span>
                    <button
                      type="button"
                      onClick={clearResult}
                      class="ml-auto shrink-0 grid h-5 w-5 place-items-center rounded text-ok/60 transition hover:text-ok"
                      aria-label="Dismiss"
                    >
                      <i class="bi bi-x-lg text-xs" />
                    </button>
                  </div>
                }
              >
                <div class="fade-rise rounded-xl border border-ok/30 bg-ok/5 p-4">
                  <div class="flex items-center justify-between gap-2">
                    <span class="flex items-center gap-2 text-sm">
                      <i class="bi bi-check-circle shrink-0 text-ok" />
                      <span class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
                        <For each={textRows()}>
                          {(row) => (
                            <span class="flex items-center gap-1.5">
                              <span class="text-muted">{row.label}</span>
                              <span class="font-semibold text-ink">{row.value}</span>
                            </span>
                          )}
                        </For>
                      </span>
                    </span>
                    <button
                      type="button"
                      onClick={clearResult}
                      class="shrink-0 grid h-5 w-5 place-items-center rounded text-ok/60 transition hover:text-ok"
                      aria-label="Dismiss"
                    >
                      <i class="bi bi-x-lg text-xs" />
                    </button>
                  </div>
                  <For each={pathRows()}>
                    {(row) => (
                      <div class="mt-3 rounded-xl border border-line bg-surface px-3 py-2">
                        <div class="flex items-center gap-2 text-[11px] text-muted">
                          <i class="bi bi-file-earmark-zip" />
                          <span class="min-w-0 font-medium text-ink">
                            <TruncatedText text={pathFileName(row.value)} placement="above" />
                          </span>
                          <span class="ml-auto flex shrink-0 items-center gap-0.5">
                            <CopyButton text={row.value} />
                            <button
                              type="button"
                              onClick={() => void callBackend("revealPath", { path: row.value })}
                              class="grid h-6 w-6 shrink-0 place-items-center rounded-full text-faint transition hover:bg-subtle hover:text-ink"
                              aria-label="Show in folder"
                            >
                              <i class="bi bi-folder2-open" />
                            </button>
                          </span>
                        </div>
                        <div class="mt-1">
                          <TruncatedText
                            text={row.value}
                            class="text-[10px] text-faint"
                            placement="above"
                          />
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </Show>
          </div>
        }
      >
        <div class="mb-4 min-w-0 rounded-2xl border border-line bg-surface p-4 shadow-sm max-[560px]:p-3">
          <Show when={running()}>
            <ScanProgress
              title={operationState.progressTitle() || "Working"}
              progress={operationState.progress()}
            />
            <p class="mt-2 text-xs text-muted">
              Running for {operationState.elapsedSeconds()}s. Keep this window open.
            </p>
          </Show>
          <Show when={recoveryNeeded()}>
            <div>
              <div class="flex min-w-0 items-center gap-2 text-sm font-semibold text-ink">
                <i class="bi bi-exclamation-triangle shrink-0 text-accent" />
                <span class="truncate">{recoveryTitle()}</span>
              </div>
              <p class="mt-1 text-sm text-muted">
                A previous task was interrupted and needs to finish before continuing.
              </p>
              <button
                type="button"
                onClick={() => void recoverOperations()}
                disabled={props.cursorRunning}
                class="mt-3 rounded-full bg-accent px-4 py-2 text-sm font-semibold text-surface transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-50"
              >
                <Show when={props.cursorRunning} fallback="Continue recovery">
                  Close Cursor first
                </Show>
              </button>
            </div>
          </Show>
          <Show when={hasError()}>
            <div
              class="rounded-xl border border-danger/30 bg-danger/5 p-3 text-sm text-danger"
              classList={{ "mt-3": running() || recoveryNeeded() }}
            >
              <i class="bi bi-exclamation-circle" /> {operationState.error()}
            </div>
          </Show>
          <Show when={hasResult()}>
            <div
              class="fade-rise flex items-center gap-2 rounded-xl border border-ok/30 bg-ok/5 px-3 py-2.5 text-sm"
              classList={{
                "mt-3": recoveryNeeded() || hasError(),
              }}
            >
              <i class="bi bi-check-circle shrink-0 text-ok" />
              <span class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
                <For each={operationState.rows() ?? []}>
                  {(row) => (
                    <span class="flex items-center gap-1.5">
                      <span class="text-muted">{row.label}</span>
                      <span class="font-semibold text-ink">{row.value}</span>
                    </span>
                  )}
                </For>
              </span>
            </div>
          </Show>
        </div>
      </Show>
    </Show>
  );
}
