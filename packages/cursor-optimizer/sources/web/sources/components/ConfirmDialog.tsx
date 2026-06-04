import { For, Show } from "solid-js";

import { confirmState, settleConfirm, updateConfirmValue } from "../library/confirmStore";
import FileBrowserField from "./FileBrowserField";

export default function ConfirmDialog() {
  const hasFileBrowser = () =>
    (confirmState.pendingConfirm()?.fields ?? []).some((field) => field.kind === "backupPath");

  return (
    <Show when={confirmState.pendingConfirm()}>
      {(dialog) => (
        <div class="fixed inset-0 z-50 grid place-items-center bg-ink/20 p-4 backdrop-blur-sm">
          <div
            class="fade-rise flex max-h-[calc(100vh-2rem)] w-full flex-col rounded-2xl border border-line bg-surface p-5 shadow-xl"
            classList={{
              "max-w-5xl min-h-0": hasFileBrowser(),
              "max-w-md": !hasFileBrowser(),
            }}
          >
            <div class="shrink-0 flex items-start gap-3">
              <div
                class="grid h-9 w-9 shrink-0 place-items-center rounded-full"
                classList={{
                  "bg-danger/10 text-danger": dialog().danger === true,
                  "bg-accent-soft text-accent": dialog().danger !== true,
                }}
              >
                <i
                  class={`bi ${dialog().danger === true ? "bi-exclamation-triangle" : "bi-info-circle"}`}
                />
              </div>
              <div>
                <h2 class="text-base font-semibold text-ink">{dialog().title}</h2>
                <p class="mt-1.5 text-sm leading-6 text-muted">{dialog().message}</p>
              </div>
            </div>
            <Show when={(dialog().fields?.length ?? 0) > 0}>
              <div class="mt-5 min-h-0 flex-1 space-y-3 overflow-hidden">
                <For each={dialog().fields ?? []}>
                  {(field) => (
                    <Show
                      when={field.kind === "backupPath" && field.fileNameId}
                      fallback={
                        <Show
                          when={field.kind === "select" && field.options}
                          fallback={
                            <label class="block">
                              <span class="text-xs font-medium text-muted">{field.label}</span>
                              <input
                                type="text"
                                value={confirmState.confirmValues()[field.id] ?? ""}
                                placeholder={field.placeholder}
                                onInput={(event) =>
                                  updateConfirmValue(field.id, event.currentTarget.value)
                                }
                                class="mt-1 w-full rounded-xl border border-line bg-canvas px-3 py-2 text-sm text-ink outline-none transition focus:border-accent"
                              />
                            </label>
                          }
                        >
                          <div class="block">
                            <span class="text-xs font-medium text-muted">{field.label}</span>
                            <div class="mt-1 flex gap-2">
                              <For each={field.options ?? []}>
                                {(option) => (
                                  <button
                                    type="button"
                                    onClick={() => updateConfirmValue(field.id, option.value)}
                                    class="flex-1 rounded-xl border px-3 py-2 text-sm font-medium transition"
                                    classList={{
                                      "border-accent bg-accent-soft text-accent":
                                        (confirmState.confirmValues()[field.id] ?? "") ===
                                        option.value,
                                      "border-line bg-canvas text-muted hover:text-ink":
                                        (confirmState.confirmValues()[field.id] ?? "") !==
                                        option.value,
                                    }}
                                  >
                                    {option.label}
                                  </button>
                                )}
                              </For>
                            </div>
                          </div>
                        </Show>
                      }
                    >
                      <FileBrowserField
                        directoryFieldId={field.id}
                        fileNameFieldId={field.fileNameId ?? ""}
                        fileNameLabel={field.fileNameLabel ?? "File name"}
                        initialDirectory={field.value}
                        initialFileName={field.fileNameValue ?? ""}
                      />
                    </Show>
                  )}
                </For>
              </div>
            </Show>
            <div class="mt-5 flex shrink-0 justify-end gap-2">
              <Show when={dialog().cancelLabel !== null}>
                <button
                  type="button"
                  onClick={() => settleConfirm(false)}
                  class="rounded-full border border-line px-4 py-2 text-sm font-medium text-muted transition hover:text-ink"
                >
                  {dialog().cancelLabel}
                </button>
              </Show>
              <button
                type="button"
                onClick={() => settleConfirm(true)}
                class="rounded-full px-4 py-2 text-sm font-semibold text-surface transition"
                classList={{
                  "bg-danger hover:opacity-90": dialog().danger === true,
                  "bg-accent hover:bg-accent-strong": dialog().danger !== true,
                }}
              >
                {dialog().confirmLabel}
              </button>
            </div>
          </div>
        </div>
      )}
    </Show>
  );
}
