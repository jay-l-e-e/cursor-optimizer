import { createEffect, createSignal, Match, onMount, Show, Switch } from "solid-js";

import ConfirmDialog from "./components/ConfirmDialog";
import CursorBanner from "./components/CursorBanner";
import DeepClean from "./components/DeepClean";
import FlushDatabase from "./components/FlushDatabase";
import LightClean from "./components/LightClean";
import OperationPanel from "./components/OperationPanel";
import Overview from "./components/Overview";
import Sidebar from "./components/Sidebar";
import StatusBar from "./components/StatusBar";
import TitleBar from "./components/TitleBar";
import Tools from "./components/Tools";
import { analyzeDeepClean, deepCleanState, resetDeepCleanResult } from "./library/deepCleanStore";
import { callBackend } from "./library/ipc";
import {
  lightCleanState,
  resetLightCleanAnalysis,
  scanLightClean,
} from "./library/lightCleanStore";
import { liveStatusState, startLiveStatus } from "./library/liveStatusStore";
import {
  clearResult,
  operationState,
  recentlyCompletedWrite,
  startOperationStatusPolling,
} from "./library/operationStore";
import { invalidateOverviewCache } from "./library/overviewStore";
import { loadQuickSummary } from "./library/summaryStore";

import type { InitializeInfo } from "./library/types";

const TAB_METADATA: Record<string, { title: string; subtitle: string }> = {
  overview: { title: "Overview", subtitle: "Storage breakdown and insights." },
  "light-clean": {
    title: "Light clean",
    subtitle: "Free up space without losing any conversations.",
  },
  "deep-clean": {
    title: "Deep clean",
    subtitle: "Permanently delete conversations older than a chosen date.",
  },
  "flush-database": {
    title: "Flush database",
    subtitle: "Apply Cursor's destructive database flush recipe.",
  },
  tools: { title: "Tools", subtitle: "Advanced utilities." },
};

export default function App() {
  const [tab, setTab] = createSignal("overview");
  const [info, setInfo] = createSignal<InitializeInfo | null>(null);
  const [refreshToken, setRefreshToken] = createSignal(0);

  const loadInfo = async () => {
    try {
      setInfo(await callBackend<InitializeInfo>("initialize"));
    } catch {
      setInfo(null);
    }
  };

  onMount(() => {
    void loadInfo();
    startOperationStatusPolling();
    startLiveStatus();
  });

  const cursorRunning = () =>
    liveStatusState.loaded() ? liveStatusState.cursorRunning() : info()?.cursorRunning === true;

  const canRefresh = () => {
    switch (tab()) {
      case "overview":
        return true;
      case "light-clean":
        return lightCleanState.analysis() !== null;
      case "deep-clean":
        return deepCleanState.analysis() !== null;
      default:
        return false;
    }
  };

  let databaseChangeTimer: ReturnType<typeof setTimeout> | undefined;
  let lastFingerprint: string | null = null;
  let lastCursorRunning: boolean | null = null;
  let suppressRefreshUntil = 0;

  const refresh = () => {
    suppressRefreshUntil = Date.now() + 4000;
    void loadInfo();
    switch (tab()) {
      case "overview":
        setRefreshToken(refreshToken() + 1);
        break;
      case "light-clean":
        resetLightCleanAnalysis();
        void scanLightClean();
        break;
      case "deep-clean":
        resetDeepCleanResult();
        void analyzeDeepClean(deepCleanState.selectedDays());
        break;
      default:
        break;
    }
  };

  createEffect(() => {
    const running = liveStatusState.cursorRunning();
    const fingerprint = liveStatusState.databaseFingerprint();
    if (!liveStatusState.loaded()) {
      return;
    }
    const fingerprintChanged =
      lastFingerprint !== null && fingerprint !== "" && fingerprint !== lastFingerprint;
    const cursorJustClosed = lastCursorRunning === true && running === false;
    if (fingerprint !== "") {
      lastFingerprint = fingerprint;
    }
    lastCursorRunning = running;
    if (running || (!fingerprintChanged && !cursorJustClosed)) {
      return;
    }
    if (databaseChangeTimer) {
      clearTimeout(databaseChangeTimer);
    }
    databaseChangeTimer = setTimeout(() => {
      if (
        operationState.busy() ||
        cursorRunning() ||
        Date.now() < suppressRefreshUntil ||
        recentlyCompletedWrite()
      ) {
        return;
      }
      suppressRefreshUntil = Date.now() + 4000;
      invalidateOverviewCache();
      void loadInfo();
      void loadQuickSummary(true);
      switch (tab()) {
        case "overview":
          setRefreshToken(refreshToken() + 1);
          break;
        case "light-clean":
          resetLightCleanAnalysis();
          void scanLightClean();
          break;
        case "deep-clean":
          resetDeepCleanResult();
          void analyzeDeepClean(deepCleanState.selectedDays());
          break;
        default:
          break;
      }
    }, 1500);
  });

  return (
    <div class="flex h-full w-full flex-col overflow-hidden">
      <TitleBar closeBlocked={operationState.closeBlocked()} />
      <div class="flex min-h-0 min-w-0 flex-1 overflow-hidden">
        <Sidebar
          active={tab()}
          onSelect={(id) => {
            if (!operationState.busy()) {
              clearResult();
            }
            setTab(id);
          }}
          info={info()}
          cursorRunning={cursorRunning()}
        />
        <main class="flex min-w-0 flex-1 flex-col overflow-hidden">
          <header class="flex items-center justify-between gap-3 border-b border-line px-8 py-5 max-[760px]:px-4 max-[560px]:py-3">
            <div class="min-w-0">
              <h1 class="truncate text-xl font-semibold tracking-tight text-ink max-[560px]:text-lg">
                {TAB_METADATA[tab()].title}
              </h1>
              <p class="mt-0.5 truncate text-sm text-muted max-[560px]:hidden">
                {TAB_METADATA[tab()].subtitle}
              </p>
            </div>
            <Show when={canRefresh()}>
              <button
                type="button"
                onClick={refresh}
                disabled={operationState.readBlocked()}
                class="shrink-0 rounded-full border border-line px-4 py-1.5 text-sm font-medium text-muted transition hover:text-ink disabled:cursor-not-allowed disabled:opacity-50 max-[560px]:px-3"
              >
                <i class="bi bi-arrow-clockwise" /> Refresh
              </button>
            </Show>
          </header>

          <div class="flex-1 overflow-y-auto px-8 py-6 max-[760px]:px-4 max-[560px]:py-4">
            <div class="mx-auto w-full max-w-4xl">
              <CursorBanner running={cursorRunning()} onChanged={setInfo} />
              <OperationPanel cursorRunning={cursorRunning()} />
              <Switch>
                <Match when={tab() === "overview"}>
                  <Overview refreshToken={refreshToken()} cursorRunning={cursorRunning()} />
                </Match>
                <Match when={tab() === "light-clean"}>
                  <LightClean cursorRunning={cursorRunning()} />
                </Match>
                <Match when={tab() === "deep-clean"}>
                  <DeepClean cursorRunning={cursorRunning()} />
                </Match>
                <Match when={tab() === "flush-database"}>
                  <FlushDatabase cursorRunning={cursorRunning()} />
                </Match>
                <Match when={tab() === "tools"}>
                  <Tools
                    cursorRunning={cursorRunning()}
                    walPresent={liveStatusState.writeAheadLogPresent()}
                  />
                </Match>
              </Switch>
            </div>
          </div>
        </main>
      </div>
      <StatusBar />
      <ConfirmDialog />
    </div>
  );
}
