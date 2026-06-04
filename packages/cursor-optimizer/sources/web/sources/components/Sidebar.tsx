import { createSignal, For, Show } from "solid-js";

import { callBackend } from "../library/ipc";
import TruncatedText from "./TruncatedText";

import type { InitializeInfo } from "../library/types";

type TabDefinition = { id: string; icon: string; label: string };

const TABS: TabDefinition[] = [
  { id: "overview", icon: "bi-grid-1x2", label: "Overview" },
  { id: "light-clean", icon: "bi-stars", label: "Light clean" },
  { id: "deep-clean", icon: "bi-calendar-range", label: "Deep clean" },
  { id: "flush-database", icon: "bi-database-down", label: "Flush database" },
  { id: "tools", icon: "bi-tools", label: "Tools" },
];

function databaseFileName(path: string | null | undefined): string {
  if (!path) {
    return "Not found";
  }
  const parts = path.split(/[\\/]/);
  return parts.at(-1) ?? path;
}

function copyDatabasePath(path: string | null | undefined): void {
  if (!path) {
    return;
  }
  void navigator.clipboard.writeText(path);
}

function openDatabaseDirectory(): void {
  void callBackend("openDatabaseDirectory");
}

function IconButton(props: {
  label: string;
  icon: string;
  feedbackLabel?: string;
  onClick: () => void;
}) {
  const [feedback, setFeedback] = createSignal(false);

  const handleClick = () => {
    props.onClick();
    if (props.feedbackLabel) {
      setFeedback(true);
      window.setTimeout(() => setFeedback(false), 1500);
    }
  };

  return (
    <span class="group relative">
      <button
        type="button"
        onClick={handleClick}
        class="grid h-6 w-6 place-items-center rounded-full text-faint transition hover:bg-subtle hover:text-ink"
        aria-label={props.label}
      >
        <i class={`bi ${feedback() ? "bi-check-lg text-ok" : props.icon}`} />
      </button>
      <span class="pointer-events-none absolute bottom-full right-0 z-10 mb-1 whitespace-nowrap rounded-lg border border-line bg-ink px-2 py-1 text-[11px] font-medium text-surface opacity-0 shadow-sm transition group-hover:opacity-100">
        {feedback() ? props.feedbackLabel : props.label}
      </span>
    </span>
  );
}

export default function Sidebar(props: {
  active: string;
  onSelect: (id: string) => void;
  info: InitializeInfo | null;
  cursorRunning: boolean;
}) {
  return (
    <aside class="flex w-60 shrink-0 flex-col border-r border-line bg-surface p-3 max-[760px]:w-16 max-[760px]:p-2">
      <nav class="flex flex-col gap-0.5">
        <For each={TABS}>
          {(tab) => (
            <button
              type="button"
              onClick={() => props.onSelect(tab.id)}
              class="flex items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition max-[760px]:justify-center max-[760px]:px-0"
              classList={{
                "bg-subtle font-semibold text-ink": props.active === tab.id,
                "text-muted hover:bg-subtle/60 hover:text-ink": props.active !== tab.id,
              }}
            >
              <i class={`bi ${tab.icon}`} />
              <span class="truncate max-[760px]:hidden" title={tab.label}>
                {tab.label}
              </span>
            </button>
          )}
        </For>
      </nav>

      <Show when={props.info?.version}>
        <div class="mb-2 mt-auto text-center text-[10px] text-faint max-[760px]:hidden">
          Version {props.info?.version}
        </div>
      </Show>

      <div class="rounded-2xl border border-line bg-canvas p-3 max-[760px]:hidden">
        <div class="flex items-start gap-3">
          <span
            class="mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-full"
            classList={{
              "bg-danger/10 text-danger": props.cursorRunning === true,
              "bg-ok/10 text-ok":
                props.cursorRunning === false && props.info?.databaseExists === true,
              "bg-subtle text-faint": props.info?.databaseExists !== true,
            }}
          >
            <i
              class="bi"
              classList={{
                "bi-exclamation-circle": props.cursorRunning === true,
                "bi-check-circle":
                  props.cursorRunning === false && props.info?.databaseExists === true,
                "bi-database-x": props.info?.databaseExists !== true,
              }}
            />
          </span>
          <div class="min-w-0">
            <div class="text-xs font-semibold text-ink">
              <Show when={props.info} fallback="Connecting">
                {props.cursorRunning
                  ? "Cursor is running"
                  : props.info?.databaseExists
                    ? "Ready to optimize"
                    : "Database not found"}
              </Show>
            </div>
            <p class="mt-1 text-[11px] leading-4 text-muted">
              <Show when={props.cursorRunning} fallback="All features available.">
                Close Cursor to enable cleanup.
              </Show>
            </p>
          </div>
        </div>
        <Show when={props.info?.databasePath}>
          <div class="mt-3 rounded-xl border border-line bg-surface px-3 py-2">
            <div class="flex items-center gap-2 text-[11px] text-muted">
              <i class="bi bi-database" />
              <TruncatedText
                text={databaseFileName(props.info?.databasePath)}
                class="font-medium text-ink"
                placement="above"
              />
              <span class="ml-auto flex items-center gap-1">
                <IconButton
                  label="Copy path"
                  feedbackLabel="Copied!"
                  icon="bi-copy"
                  onClick={() => copyDatabasePath(props.info?.databasePath)}
                />
                <IconButton
                  label="Show in Explorer"
                  icon="bi-folder2-open"
                  onClick={openDatabaseDirectory}
                />
              </span>
            </div>
            <div class="mt-1">
              <TruncatedText
                text={props.info?.databasePath ?? ""}
                class="text-[10px] text-faint"
                placement="above"
              />
            </div>
          </div>
        </Show>
      </div>
    </aside>
  );
}
