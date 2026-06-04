import { createSignal } from "solid-js";

import { callBackend } from "./ipc";

type CursorStatus = {
  cursorRunning: boolean;
  writeAheadLogPresent: boolean;
  databaseFingerprint?: string;
};

declare global {
  interface Window {
    __cursorStatus?: (status: CursorStatus) => void;
  }
}

const [cursorRunning, setCursorRunning] = createSignal(false);
const [writeAheadLogPresent, setWriteAheadLogPresent] = createSignal(false);
const [databaseFingerprint, setDatabaseFingerprint] = createSignal("");
const [loaded, setLoaded] = createSignal(false);

let started = false;

export const liveStatusState = {
  cursorRunning,
  writeAheadLogPresent,
  databaseFingerprint,
  loaded,
};

function applyStatus(status: CursorStatus): void {
  setCursorRunning(status.cursorRunning === true);
  setWriteAheadLogPresent(status.writeAheadLogPresent === true);
  if (typeof status.databaseFingerprint === "string") {
    setDatabaseFingerprint(status.databaseFingerprint);
  }
  setLoaded(true);
}

export async function refreshLiveStatus(): Promise<void> {
  try {
    applyStatus(await callBackend<CursorStatus>("cursorStatus"));
  } catch {
    /* transient errors are ignored; the backend re-pushes on the next change */
  }
}

export function startLiveStatus(): void {
  if (started) {
    return;
  }
  started = true;
  window.__cursorStatus = (status) => applyStatus(status);
  void refreshLiveStatus();
}
