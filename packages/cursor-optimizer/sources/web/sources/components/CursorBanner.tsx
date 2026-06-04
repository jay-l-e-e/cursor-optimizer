import { createSignal, Show } from "solid-js";

import { callBackend } from "../library/ipc";
import { refreshLiveStatus } from "../library/liveStatusStore";

import type { InitializeInfo } from "../library/types";

export default function CursorBanner(props: {
  running: boolean;
  onChanged: (info: InitializeInfo) => void;
}) {
  const [quitting, setQuitting] = createSignal(false);

  const forceQuit = async () => {
    setQuitting(true);
    try {
      const updated = await callBackend<InitializeInfo>("forceQuitCursor");
      props.onChanged(updated);
      await refreshLiveStatus();
    } finally {
      setQuitting(false);
    }
  };

  return (
    <Show when={props.running}>
      <div class="fade-rise mb-4 flex items-center justify-between gap-4 rounded-xl border border-accent/30 bg-accent-soft px-4 py-3 max-[560px]:flex-col max-[560px]:items-stretch">
        <div class="flex min-w-0 items-center gap-2.5 text-sm text-ink">
          <i class="bi bi-exclamation-triangle shrink-0 text-accent" />
          <span>Cursor is running. Close it to enable cleanup.</span>
        </div>
        <button
          type="button"
          onClick={forceQuit}
          disabled={quitting()}
          class="shrink-0 rounded-full bg-accent px-4 py-1.5 text-sm font-semibold text-surface transition hover:bg-accent-strong disabled:opacity-60"
        >
          {quitting() ? "Closing..." : "Close Cursor"}
        </button>
      </div>
    </Show>
  );
}
