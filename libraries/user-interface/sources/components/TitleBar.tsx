import { createSignal, onCleanup, onMount, Show } from "solid-js";

import iconUrl from "../../../../assets/icon.svg";
import { callBackend } from "../ipc";

function sendWindowAction(action: string): void {
  void callBackend(action);
}

function RestoreIcon() {
  return (
    <span class="relative block h-3.5 w-3.5">
      <span class="absolute left-0 top-1 h-2.5 w-2.5 rounded-[3px] border border-current" />
      <span class="absolute left-1 top-0 h-2.5 w-2.5 rounded-[3px] border border-current" />
    </span>
  );
}

type WindowState = {
  maximized: boolean;
};

export default function TitleBar(props: {
  title?: string;
  closeBlocked?: boolean;
  onCloseBlocked?: () => void;
  showMaximize?: boolean;
}) {
  const [maximized, setMaximized] = createSignal(false);
  let dragRegion: HTMLDivElement | undefined;

  const refreshWindowState = async () => {
    try {
      const state = await callBackend<WindowState>("windowState");
      setMaximized(state.maximized);
    } catch {
      setMaximized(false);
    }
  };

  const toggleMaximize = async () => {
    try {
      const state = await callBackend<WindowState>("windowToggleMaximize");
      setMaximized(state.maximized);
    } catch {
      setMaximized(false);
    }
  };

  onMount(() => {
    if (props.showMaximize) {
      void refreshWindowState();
    }
    const currentDragRegion = dragRegion;
    if (!currentDragRegion) {
      return;
    }
    const handleMouseDown = (event: MouseEvent) => {
      if (event.button !== 0) {
        return;
      }
      if (event.target instanceof Element && event.target.closest(".window-drag-excluded")) {
        return;
      }
      if (props.showMaximize && event.detail === 2) {
        void toggleMaximize();
      } else {
        sendWindowAction("windowDrag");
      }
    };
    currentDragRegion.addEventListener("mousedown", handleMouseDown);
    const handleWindowStateChanged = () => {
      window.setTimeout(() => void refreshWindowState(), 50);
      window.setTimeout(() => void refreshWindowState(), 250);
    };
    if (props.showMaximize) {
      window.addEventListener("resize", handleWindowStateChanged);
      window.addEventListener("focus", handleWindowStateChanged);
    }
    onCleanup(() => {
      currentDragRegion.removeEventListener("mousedown", handleMouseDown);
      if (props.showMaximize) {
        window.removeEventListener("resize", handleWindowStateChanged);
        window.removeEventListener("focus", handleWindowStateChanged);
      }
    });
  });

  return (
    <div
      ref={dragRegion}
      class="window-drag-region flex h-10 shrink-0 items-center justify-between border-b border-line bg-surface/95 pl-3"
    >
      <div class="flex min-w-0 items-center gap-2">
        <img src={iconUrl} alt="Cursor Optimizer" class="h-5 w-5 shrink-0 rounded-[5px]" />
        <span class="truncate text-[13px] font-semibold tracking-tight text-ink">
          {props.title ?? "Cursor Optimizer"}
        </span>
      </div>
      <div class="window-drag-excluded flex h-full">
        <button
          type="button"
          onClick={() => sendWindowAction("windowMinimize")}
          class="grid h-full w-11 place-items-center text-muted transition hover:bg-subtle hover:text-ink"
          aria-label="Minimize window"
        >
          <i class="bi bi-dash-lg" />
        </button>
        <Show when={props.showMaximize}>
          <button
            type="button"
            onClick={() => void toggleMaximize()}
            class="grid h-full w-11 place-items-center text-muted transition hover:bg-subtle hover:text-ink"
            aria-label={maximized() ? "Restore window" : "Maximize window"}
          >
            <Show when={maximized()} fallback={<i class="bi bi-square" />}>
              <RestoreIcon />
            </Show>
          </button>
        </Show>
        <button
          type="button"
          onClick={() => {
            if (props.closeBlocked && props.onCloseBlocked) {
              props.onCloseBlocked();
            } else {
              sendWindowAction("windowClose");
            }
          }}
          class="grid h-full w-11 place-items-center text-muted transition hover:bg-danger hover:text-surface"
          aria-label="Close window"
        >
          <i class="bi bi-x-lg" />
        </button>
      </div>
    </div>
  );
}
