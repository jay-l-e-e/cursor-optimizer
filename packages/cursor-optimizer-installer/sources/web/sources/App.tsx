import { FileBrowserField, GhostButton, PrimaryButton } from "@cursor-optimizer/user-interface";
import { createSignal, Match, onMount, Show, Switch } from "solid-js";
import { createStore } from "solid-js/store";

import TitleBar from "./components/TitleBar";
import { callBackend, startRequest } from "./library/ipc";
import {
  buildInstallParams,
  buildUninstallParams,
  fallbackInfo,
  type InstallerInfo,
  type InstallForm,
  initialInstallForm,
  initialUninstallForm,
  isInstallDirectoryValid,
  parseInstallerInfo,
  readExecutablePath,
  type UninstallForm,
} from "./library/wizardModel";

type Step = "loading" | "welcome" | "options" | "confirm" | "progress" | "done" | "error";

function closeWindow(): void {
  void callBackend("windowClose");
}

function CheckRow(props: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label class="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-surface px-4 py-3 transition hover:border-accent/40">
      <input
        type="checkbox"
        class="mt-0.5 h-4 w-4 accent-accent"
        checked={props.checked}
        onChange={(event) => props.onChange(event.currentTarget.checked)}
      />
      <span class="min-w-0">
        <span class="block text-[13px] font-medium text-ink">{props.label}</span>
        <span class="block text-[12px] text-muted">{props.description}</span>
      </span>
    </label>
  );
}

export default function App() {
  const [info, setInfo] = createSignal<InstallerInfo>(fallbackInfo);
  const [step, setStep] = createSignal<Step>("loading");
  const [progressText, setProgressText] = createSignal("");
  const [errorMessage, setErrorMessage] = createSignal("");
  const [executablePath, setExecutablePath] = createSignal("");
  const [showBrowser, setShowBrowser] = createSignal(false);
  const [installForm, setInstallForm] = createStore<InstallForm>(initialInstallForm(fallbackInfo));
  const [uninstallForm, setUninstallForm] = createStore<UninstallForm>(initialUninstallForm());

  onMount(async () => {
    try {
      const payload = await callBackend<unknown>("installerInfo");
      const parsed = parseInstallerInfo(payload);
      setInfo(parsed);
      setInstallForm(initialInstallForm(parsed));
      if (parsed.mode === "uninstall") {
        setStep("confirm");
      } else if (!parsed.hasEmbeddedBinary) {
        setErrorMessage("The installer package is incomplete.");
        setStep("error");
      } else {
        setStep("welcome");
      }
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "The installer could not start.");
      setStep("error");
    }
  });

  const runInstall = async () => {
    if (!isInstallDirectoryValid(installForm)) {
      return;
    }
    setStep("progress");
    setProgressText("Starting the installation…");
    try {
      const result = await startRequest<unknown>(
        "install",
        buildInstallParams(installForm),
        setProgressText,
      );
      setExecutablePath(readExecutablePath(result));
      setStep("done");
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "The installation failed.");
      setStep("error");
    }
  };

  const runUninstall = async () => {
    setStep("progress");
    setProgressText("Starting the removal…");
    try {
      await startRequest<unknown>(
        "uninstall",
        buildUninstallParams(uninstallForm),
        setProgressText,
      );
      setStep("done");
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "The removal failed.");
      setStep("error");
    }
  };

  const launchApp = async () => {
    try {
      await callBackend("launchApp", { executablePath: executablePath() });
    } catch {
      closeWindow();
      return;
    }
    closeWindow();
  };

  const isUninstall = () => info().mode === "uninstall";
  const actionKind = (): "install" | "update" | "repair" => {
    if (info().mode !== "install" || info().installedDirectory === null) {
      return "install";
    }
    const installed = info().installedVersion;
    if (installed !== null && installed === info().version) {
      return "repair";
    }
    return "update";
  };
  const actionLabel = () =>
    actionKind() === "repair" ? "Repair" : actionKind() === "update" ? "Update" : "Install";
  const actionIcon = () =>
    actionKind() === "repair"
      ? "bi-wrench-adjustable"
      : actionKind() === "update"
        ? "bi-arrow-repeat"
        : "bi-download";

  return (
    <div class="flex h-full flex-col bg-canvas text-ink">
      <TitleBar
        title={isUninstall() ? "Uninstall Cursor Optimizer" : `${actionLabel()} Cursor Optimizer`}
      />
      <main class="flex flex-1 items-center justify-center overflow-hidden px-8 py-7">
        <div class="fade-rise w-full max-w-lg">
          <Switch>
            <Match when={step() === "loading"}>
              <div class="flex flex-col items-center gap-3 text-muted">
                <div class="scan-track w-48">
                  <div class="scan-bar" />
                </div>
                <p class="text-[13px]">Preparing…</p>
              </div>
            </Match>

            <Match when={step() === "welcome"}>
              <div class="flex flex-col gap-6">
                <header class="flex flex-col gap-2">
                  <span class="grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-accent">
                    <i class={`bi ${actionIcon()} text-xl`} />
                  </span>
                  <h1 class="text-2xl font-semibold tracking-tight">
                    {actionLabel()} {info().productName}
                    <Show when={info().version !== ""}> {info().version}</Show>
                  </h1>
                  <Switch
                    fallback={
                      <p class="text-[13px] leading-relaxed text-muted">
                        Set up {info().productName} and keep your editor storage clean.
                      </p>
                    }
                  >
                    <Match when={actionKind() === "update"}>
                      <p class="text-[13px] leading-relaxed text-muted">
                        An existing installation was found. Continue to update it in place.
                      </p>
                    </Match>
                    <Match when={actionKind() === "repair"}>
                      <p class="text-[13px] leading-relaxed text-muted">
                        {info().productName} {info().version} is already installed. Continue to
                        repair it in place.
                      </p>
                    </Match>
                  </Switch>
                  <Show when={actionKind() !== "install" && info().installedDirectory}>
                    <span class="truncate rounded-lg border border-line bg-surface px-3 py-2 text-[12px] text-muted">
                      {info().installedDirectory}
                    </span>
                  </Show>
                  <Show when={actionKind() === "update" && info().installedVersion}>
                    <span class="flex items-center gap-2 text-[13px] font-medium text-ink">
                      <span class="text-muted">{info().installedVersion}</span>
                      <i class="bi bi-arrow-right text-[11px] text-faint" />
                      <span class="text-accent">{info().version}</span>
                    </span>
                  </Show>
                </header>
                <div class="flex justify-end gap-3">
                  <GhostButton label="Cancel" onClick={closeWindow} />
                  <PrimaryButton label="Continue" onClick={() => setStep("options")} />
                </div>
              </div>
            </Match>

            <Match when={step() === "options"}>
              <div class="flex flex-col gap-5">
                <header class="flex flex-col gap-1">
                  <h1 class="text-xl font-semibold tracking-tight">Choose your options</h1>
                  <p class="text-[13px] text-muted">
                    Pick where to install and which shortcuts to add.
                  </p>
                </header>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] font-medium text-muted">Install location</span>
                  <div class="flex items-center gap-2">
                    <input
                      type="text"
                      value={installForm.installDirectory}
                      onInput={(event) =>
                        setInstallForm("installDirectory", event.currentTarget.value)
                      }
                      class="min-w-0 flex-1 rounded-xl border border-line bg-surface px-3.5 py-2.5 text-[13px] text-ink outline-none transition focus:border-accent"
                      spellcheck={false}
                    />
                    <button
                      type="button"
                      onClick={() => setShowBrowser(true)}
                      class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-line bg-surface text-muted transition hover:border-accent/40 hover:text-ink"
                      aria-label="Browse folders"
                    >
                      <i class="bi bi-folder2-open" />
                    </button>
                  </div>
                </div>
                <Show when={showBrowser()}>
                  <div class="fixed inset-0 z-50 grid place-items-center bg-ink/20 p-4 backdrop-blur-sm">
                    <div class="fade-rise flex w-full max-w-4xl flex-col gap-3 rounded-2xl border border-line bg-surface p-4 shadow-xl">
                      <div class="flex items-center justify-between">
                        <span class="text-[13px] font-semibold text-ink">Choose folder</span>
                        <button
                          type="button"
                          onClick={() => setShowBrowser(false)}
                          class="grid h-7 w-7 place-items-center rounded-full text-muted transition hover:bg-subtle hover:text-ink"
                          aria-label="Close browser"
                        >
                          <i class="bi bi-x-lg text-xs" />
                        </button>
                      </div>
                      <FileBrowserField
                        initialDirectory={installForm.installDirectory}
                        onDirectoryChange={(directory) => {
                          setInstallForm("installDirectory", directory);
                        }}
                      />
                      <div class="flex justify-end">
                        <PrimaryButton label="Select" onClick={() => setShowBrowser(false)} />
                      </div>
                    </div>
                  </div>
                </Show>
                <div class="flex flex-col gap-2.5">
                  <CheckRow
                    label="Start menu shortcut"
                    description="Add Cursor Optimizer to your applications menu."
                    checked={installForm.createStartMenuShortcut}
                    onChange={(value) => setInstallForm("createStartMenuShortcut", value)}
                  />
                  <CheckRow
                    label="Desktop shortcut"
                    description="Place a shortcut on the desktop."
                    checked={installForm.createDesktopShortcut}
                    onChange={(value) => setInstallForm("createDesktopShortcut", value)}
                  />
                </div>
                <div class="flex justify-between gap-3">
                  <GhostButton label="Back" onClick={() => setStep("welcome")} />
                  <PrimaryButton
                    label={actionLabel()}
                    disabled={!isInstallDirectoryValid(installForm)}
                    onClick={() => void runInstall()}
                  />
                </div>
              </div>
            </Match>

            <Match when={step() === "confirm"}>
              <div class="flex flex-col gap-5">
                <header class="flex flex-col gap-2">
                  <span class="grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-accent">
                    <i class="bi bi-trash3 text-xl" />
                  </span>
                  <h1 class="text-xl font-semibold tracking-tight">Remove {info().productName}?</h1>
                  <p class="text-[13px] text-muted">
                    This removes the application and its shortcuts from this computer.
                  </p>
                </header>
                <CheckRow
                  label="Also remove app data"
                  description="Remove preferences and cached data."
                  checked={uninstallForm.removeApplicationData}
                  onChange={(value) => setUninstallForm("removeApplicationData", value)}
                />
                <div class="flex justify-between gap-3">
                  <GhostButton label="Cancel" onClick={closeWindow} />
                  <PrimaryButton label="Uninstall" onClick={() => void runUninstall()} />
                </div>
              </div>
            </Match>

            <Match when={step() === "progress"}>
              <div class="flex flex-col items-center gap-4 text-center">
                <div class="scan-track w-56">
                  <div class="scan-bar" />
                </div>
                <p class="text-[13px] text-muted">{progressText() || "Working…"}</p>
              </div>
            </Match>

            <Match when={step() === "done"}>
              <div class="flex flex-col gap-5">
                <header class="flex flex-col gap-2">
                  <span class="grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-ok">
                    <i class="bi bi-check2 text-2xl" />
                  </span>
                  <h1 class="text-xl font-semibold tracking-tight">
                    <Show
                      when={isUninstall()}
                      fallback={
                        actionKind() === "repair"
                          ? "Repair complete"
                          : actionKind() === "update"
                            ? "Update complete"
                            : "Installation complete"
                      }
                    >
                      Removal complete
                    </Show>
                  </h1>
                  <p class="text-[13px] text-muted">
                    <Show when={isUninstall()} fallback={`${info().productName} is ready to use.`}>
                      {info().productName} has been removed from this computer.
                    </Show>
                  </p>
                </header>
                <div class="flex justify-end gap-3">
                  <Show
                    when={!isUninstall() && executablePath() !== ""}
                    fallback={<PrimaryButton label="Close" onClick={closeWindow} />}
                  >
                    <GhostButton label="Close" onClick={closeWindow} />
                    <PrimaryButton label="Launch now" onClick={() => void launchApp()} />
                  </Show>
                </div>
              </div>
            </Match>

            <Match when={step() === "error"}>
              <div class="flex flex-col gap-5">
                <header class="flex flex-col gap-2">
                  <span class="grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-danger">
                    <i class="bi bi-exclamation-triangle text-xl" />
                  </span>
                  <h1 class="text-xl font-semibold tracking-tight">Something went wrong</h1>
                  <p class="text-[13px] text-muted">{errorMessage()}</p>
                </header>
                <div class="flex justify-end">
                  <PrimaryButton label="Close" onClick={closeWindow} />
                </div>
              </div>
            </Match>
          </Switch>
        </div>
      </main>
    </div>
  );
}
