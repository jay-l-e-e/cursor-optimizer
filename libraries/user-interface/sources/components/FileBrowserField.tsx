import { createEffect, createMemo, createSignal, For, onMount, Show } from "solid-js";

import { callBackend } from "../ipc";

import type { DirectoryListing, PendingDirectory } from "../types";

let nextPendingId = 0;
function generatePendingId(): string {
  nextPendingId = nextPendingId + 1;
  return `pending-${String(nextPendingId)}`;
}

function normalizePath(path: string): string {
  return path
    .replace(/[\\/]+$/, "")
    .replace(/\\/g, "/")
    .toLowerCase();
}

function joinPath(base: string, segment: string): string {
  const separator = base.includes("\\") ? "\\" : "/";
  const trimmed = base.endsWith("\\") || base.endsWith("/") ? base : `${base}${separator}`;
  return `${trimmed}${segment}`;
}

export default function FileBrowserField(props: {
  initialDirectory: string;
  fileNameFieldId?: string;
  fileNameLabel?: string;
  initialFileName?: string;
  showFileName?: boolean;
  onDirectoryChange?: (directory: string) => void;
  onFileNameChange?: (fileName: string) => void;
  onPendingDirectoriesChange?: (directories: PendingDirectory[]) => void;
}) {
  const [listing, setListing] = createSignal<DirectoryListing | null>(null);
  const [directory, setDirectory] = createSignal(props.initialDirectory);
  const [fileName, setFileName] = createSignal(props.initialFileName ?? "");
  const [error, setError] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [filterText, setFilterText] = createSignal("");
  const [insidePending, setInsidePending] = createSignal(false);
  const [pendingMap, setPendingMap] = createSignal<Map<string, PendingDirectory[]>>(new Map());
  const [renamingId, setRenamingId] = createSignal<string | null>(null);
  const [renameValue, setRenameValue] = createSignal("");

  const pathSeparator = createMemo(() => (directory().includes("\\") ? "\\" : "/"));

  const pathParts = createMemo(() => {
    const currentDirectory = directory();
    const separator = pathSeparator();
    const rawParts = currentDirectory.split(/[\\/]+/).filter((part) => part.length > 0);
    if (currentDirectory.match(/^[A-Za-z]:\\/)) {
      const drive = rawParts[0] ?? currentDirectory.slice(0, 2);
      return rawParts.map((part, position) => ({
        label: position === 0 ? `${drive}\\` : part,
        path:
          position === 0
            ? `${drive}\\`
            : `${rawParts.slice(0, position + 1).join(separator)}${separator}`,
      }));
    }
    return rawParts.map((part, position) => ({
      label: part,
      path: `${separator}${rawParts.slice(0, position + 1).join(separator)}`,
    }));
  });

  const pendingEntriesForCurrentDirectory = createMemo(() => {
    const key = normalizePath(directory());
    return pendingMap().get(key) ?? [];
  });

  const visibleEntries = createMemo(() => {
    const normalizedFilter = filterText().trim().toLowerCase();
    const realEntries = insidePending() ? [] : (listing()?.entries ?? []);
    if (normalizedFilter === "") {
      return realEntries;
    }
    return realEntries.filter((entry) => entry.name.toLowerCase().includes(normalizedFilter));
  });

  const visiblePendingEntries = createMemo(() => {
    const normalizedFilter = filterText().trim().toLowerCase();
    const entries = pendingEntriesForCurrentDirectory();
    if (normalizedFilter === "") {
      return entries;
    }
    return entries.filter((entry) => entry.name.toLowerCase().includes(normalizedFilter));
  });

  const destinationPath = createMemo(() => {
    const separator = pathSeparator();
    const base =
      directory().endsWith("\\") || directory().endsWith("/")
        ? directory()
        : `${directory()}${separator}`;
    return `${base}${fileName()}`;
  });

  const totalVisibleCount = createMemo(
    () => visibleEntries().length + visiblePendingEntries().length,
  );

  function addPendingEntry(parentPath: string, name: string): PendingDirectory {
    const entry: PendingDirectory = { id: generatePendingId(), name };
    const key = normalizePath(parentPath);
    setPendingMap((previous) => {
      const updated = new Map(previous);
      const existing = updated.get(key) ?? [];
      updated.set(key, [...existing, entry]);
      return updated;
    });
    notifyPendingChange();
    return entry;
  }

  function renamePendingEntry(entryId: string, newName: string): void {
    setPendingMap((previous) => {
      const updated = new Map(previous);
      for (const [key, entries] of updated) {
        const index = entries.findIndex((entry) => entry.id === entryId);
        if (index >= 0) {
          const patched = [...entries];
          patched[index] = { ...patched[index], name: newName };
          updated.set(key, patched);
          break;
        }
      }
      return updated;
    });
    notifyPendingChange();
  }

  function deletePendingEntry(entryId: string): void {
    setPendingMap((previous) => {
      const updated = new Map(previous);
      for (const [key, entries] of updated) {
        const filtered = entries.filter((entry) => entry.id !== entryId);
        if (filtered.length !== entries.length) {
          if (filtered.length === 0) {
            updated.delete(key);
          } else {
            updated.set(key, filtered);
          }
          break;
        }
      }
      return updated;
    });
    notifyPendingChange();
  }

  function notifyPendingChange(): void {
    if (!props.onPendingDirectoriesChange) {
      return;
    }
    const allPending: PendingDirectory[] = [];
    for (const entries of pendingMap().values()) {
      for (const entry of entries) {
        allPending.push(entry);
      }
    }
    props.onPendingDirectoriesChange(allPending);
  }

  function createNewFolder(): void {
    const baseName = "New folder";
    const existing = pendingEntriesForCurrentDirectory();
    const realEntries = listing()?.entries ?? [];
    const allNames = new Set([
      ...existing.map((entry) => entry.name.toLowerCase()),
      ...realEntries.map((entry) => entry.name.toLowerCase()),
    ]);
    let candidate = baseName;
    let counter = 2;
    while (allNames.has(candidate.toLowerCase())) {
      candidate = `${baseName} ${String(counter)}`;
      counter = counter + 1;
    }
    const entry = addPendingEntry(directory(), candidate);
    setRenamingId(entry.id);
    setRenameValue(candidate);
  }

  function commitRename(entryId: string): void {
    const trimmed = renameValue().trim();
    if (trimmed.length > 0) {
      renamePendingEntry(entryId, trimmed);
    }
    setRenamingId(null);
    setRenameValue("");
  }

  function startRename(entry: PendingDirectory): void {
    setRenamingId(entry.id);
    setRenameValue(entry.name);
  }

  const openDirectory = async (path: string) => {
    setError("");
    setLoading(true);
    setInsidePending(false);
    setFilterText("");
    try {
      const data = await callBackend<DirectoryListing>("browseDirectories", { path });
      setListing(data);
      setDirectory(data.currentDirectory);
      props.onDirectoryChange?.(data.currentDirectory);
      if (data.pendingSegments.length > 0) {
        let currentParent = data.currentDirectory;
        for (const segment of data.pendingSegments) {
          const key = normalizePath(currentParent);
          const existingEntries = pendingMap().get(key) ?? [];
          const alreadyExists = existingEntries.some(
            (entry) => entry.name.toLowerCase() === segment.toLowerCase(),
          );
          if (!alreadyExists) {
            addPendingEntry(currentParent, segment);
          }
          currentParent = joinPath(currentParent, segment);
        }
      }
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  };

  const openPendingDirectory = (parentPath: string, pendingName: string) => {
    const fullPath = joinPath(parentPath, pendingName);
    setDirectory(fullPath);
    setInsidePending(true);
    setFilterText("");
    setListing(null);
    props.onDirectoryChange?.(fullPath);
  };

  const navigateUp = () => {
    if (insidePending()) {
      const currentDir = directory();
      const separator = pathSeparator();
      const parts = currentDir.split(/[\\/]+/).filter((part) => part.length > 0);
      if (parts.length > 1) {
        parts.pop();
        let parentPath: string;
        if (currentDir.match(/^[A-Za-z]:\\/)) {
          parentPath = parts.length === 1 ? `${parts[0]}\\` : parts.join(separator);
        } else {
          parentPath = `${separator}${parts.join(separator)}`;
        }
        const parentKey = normalizePath(parentPath);
        const parentHasPending = pendingMap().has(parentKey);
        if (parentHasPending) {
          const isRealDirectory =
            listing()?.currentDirectory !== undefined &&
            normalizePath(listing()?.currentDirectory ?? "") === parentKey;
          if (isRealDirectory) {
            void openDirectory(parentPath);
          } else {
            setDirectory(parentPath);
            props.onDirectoryChange?.(parentPath);
          }
        } else {
          void openDirectory(parentPath);
        }
      }
    } else {
      const parentDirectory = listing()?.parentDirectory;
      if (parentDirectory) {
        void openDirectory(parentDirectory);
      }
    }
  };

  createEffect(() => {
    props.onFileNameChange?.(fileName());
  });

  onMount(() => {
    void openDirectory(directory());
  });

  return (
    <div class="h-[min(620px,calc(100vh-14rem))] min-h-0 overflow-x-auto overflow-y-hidden rounded-2xl border border-line bg-canvas">
      <div class="grid h-full min-h-0 min-w-136 grid-cols-[210px_minmax(0,1fr)]">
        <aside class="min-h-0 overflow-y-auto border-r border-line bg-surface p-3">
          <div class="text-xs font-semibold uppercase tracking-wide text-faint">
            Quick locations
          </div>
          <div class="mt-2 space-y-1">
            <For each={listing()?.quickLocations ?? []}>
              {(location) => (
                <button
                  type="button"
                  onClick={() => void openDirectory(location.path)}
                  class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm text-muted transition hover:bg-subtle hover:text-ink"
                  classList={{
                    "bg-subtle font-semibold text-ink": directory() === location.path,
                  }}
                >
                  <i class="bi bi-pin-map" />
                  <span class="truncate">{location.name}</span>
                </button>
              )}
            </For>
          </div>

          <div class="mt-5 text-xs font-semibold uppercase tracking-wide text-faint">Drives</div>
          <div class="mt-2 space-y-1">
            <For each={listing()?.roots ?? []}>
              {(root) => (
                <button
                  type="button"
                  onClick={() => void openDirectory(root.path)}
                  class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm text-muted transition hover:bg-subtle hover:text-ink"
                >
                  <i class="bi bi-device-hdd" />
                  {root.path}
                </button>
              )}
            </For>
          </div>
        </aside>

        <div class="grid min-h-0 min-w-0 grid-rows-[auto_minmax(0,1fr)_auto_auto]">
          <div class="shrink-0 border-b border-line bg-surface p-3">
            <div class="flex items-center gap-2">
              <button
                type="button"
                disabled={!insidePending() && !listing()?.parentDirectory}
                onClick={navigateUp}
                class="rounded-full border border-line px-3 py-1.5 text-xs font-medium text-muted transition hover:text-ink disabled:cursor-not-allowed disabled:opacity-40"
              >
                <i class="bi bi-arrow-up" /> Up
              </button>
              <div class="min-w-0 flex-1 rounded-full border border-line bg-canvas px-3 py-1.5">
                <div class="flex min-w-0 items-center gap-1 overflow-hidden text-xs">
                  <For each={pathParts()}>
                    {(part) => (
                      <>
                        <button
                          type="button"
                          onClick={() => void openDirectory(part.path)}
                          class="max-w-36 truncate rounded-full px-2 py-0.5 text-muted transition hover:bg-subtle hover:text-ink"
                        >
                          {part.label}
                        </button>
                        <span class="text-faint">/</span>
                      </>
                    )}
                  </For>
                </div>
              </div>
              <button
                type="button"
                onClick={createNewFolder}
                class="flex shrink-0 items-center gap-1.5 rounded-full border border-line px-3 py-1.5 text-xs font-medium text-muted transition hover:border-accent/40 hover:text-ink"
              >
                <i class="bi bi-folder-plus" />
                New folder
              </button>
            </div>
            <div class="mt-3 flex items-center gap-2">
              <div class="relative flex-1">
                <i class="bi bi-search absolute left-3 top-1/2 -translate-y-1/2 text-xs text-faint" />
                <input
                  type="text"
                  value={filterText()}
                  onInput={(event) => setFilterText(event.currentTarget.value)}
                  placeholder="Filter folders"
                  class="w-full rounded-full border border-line bg-canvas py-2 pl-8 pr-3 text-sm text-ink outline-none transition focus:border-accent"
                />
              </div>
              <div class="shrink-0 text-xs text-faint">{totalVisibleCount()} folders</div>
            </div>
          </div>

          <div class="min-h-0 overflow-y-auto p-3">
            <Show
              when={!loading()}
              fallback={
                <div class="rounded-xl bg-surface p-4 text-sm text-muted">Loading folders...</div>
              }
            >
              <Show
                when={totalVisibleCount() > 0}
                fallback={
                  <div class="rounded-xl bg-surface p-4 text-sm text-faint">No folders match</div>
                }
              >
                <div class="grid grid-cols-2 gap-2 lg:grid-cols-3">
                  <For each={visiblePendingEntries()}>
                    {(entry) => (
                      <div class="group relative flex min-w-0 items-center gap-2 rounded-xl border border-dashed border-accent/40 bg-accent-soft/30 px-3 py-3 text-left text-sm text-muted transition hover:bg-accent-soft/60">
                        <i class="bi bi-folder-plus shrink-0 text-accent/60" />
                        <Show
                          when={renamingId() === entry.id}
                          fallback={
                            <button
                              type="button"
                              class="min-w-0 flex-1 truncate text-left"
                              onClick={() => openPendingDirectory(directory(), entry.name)}
                            >
                              {entry.name}
                            </button>
                          }
                        >
                          <input
                            type="text"
                            value={renameValue()}
                            onInput={(event) => setRenameValue(event.currentTarget.value)}
                            onKeyDown={(event) => {
                              if (event.key === "Enter") {
                                commitRename(entry.id);
                              } else if (event.key === "Escape") {
                                setRenamingId(null);
                              }
                            }}
                            onBlur={() => commitRename(entry.id)}
                            ref={(element) => setTimeout(() => element.focus(), 0)}
                            class="min-w-0 flex-1 rounded border border-accent bg-surface px-1.5 py-0.5 text-sm text-ink outline-none"
                          />
                        </Show>
                        <Show when={renamingId() !== entry.id}>
                          <div class="absolute right-2 top-1/2 flex -translate-y-1/2 gap-0.5 opacity-0 transition group-hover:opacity-100">
                            <button
                              type="button"
                              onClick={(event) => {
                                event.stopPropagation();
                                startRename(entry);
                              }}
                              class="grid h-6 w-6 place-items-center rounded text-faint transition hover:bg-surface hover:text-ink"
                              aria-label="Rename folder"
                            >
                              <i class="bi bi-pencil text-[11px]" />
                            </button>
                            <button
                              type="button"
                              onClick={(event) => {
                                event.stopPropagation();
                                deletePendingEntry(entry.id);
                              }}
                              class="grid h-6 w-6 place-items-center rounded text-faint transition hover:bg-surface hover:text-danger"
                              aria-label="Remove folder"
                            >
                              <i class="bi bi-trash3 text-[11px]" />
                            </button>
                          </div>
                        </Show>
                      </div>
                    )}
                  </For>
                  <For each={visibleEntries()}>
                    {(entry) => (
                      <button
                        type="button"
                        onClick={() => void openDirectory(entry.path)}
                        class="flex min-w-0 items-center gap-2 rounded-xl border border-line bg-surface px-3 py-3 text-left text-sm text-muted transition hover:border-accent/40 hover:bg-accent-soft hover:text-ink"
                      >
                        <i class="bi bi-folder-fill shrink-0 text-accent" />
                        <span class="truncate">{entry.name}</span>
                      </button>
                    )}
                  </For>
                </div>
              </Show>
            </Show>
          </div>

          <Show when={props.showFileName}>
            <div class="shrink-0 border-t border-line bg-surface p-3">
              <label class="block">
                <span class="text-xs font-medium text-muted">
                  {props.fileNameLabel ?? "File name"}
                </span>
                <input
                  type="text"
                  value={fileName()}
                  onInput={(event) => setFileName(event.currentTarget.value)}
                  class="mt-1 w-full rounded-xl border border-line bg-canvas px-3 py-2 text-sm text-ink outline-none transition focus:border-accent"
                />
              </label>
              <div class="mt-2 rounded-xl bg-canvas px-3 py-2">
                <div class="text-xs font-medium text-muted">Save as</div>
                <div class="mt-0.5 break-all text-[11px] text-ink">{destinationPath()}</div>
              </div>
            </div>
          </Show>

          <Show when={error()}>
            <div class="border-t border-line bg-danger/5 px-3 py-2 text-xs text-danger">
              {error()}
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
