export type DirectoryEntry = {
  name: string;
  path: string;
};

export type DirectoryListing = {
  currentDirectory: string;
  parentDirectory: string | null;
  roots: { path: string }[];
  quickLocations: { name: string; path: string }[];
  entries: DirectoryEntry[];
  pendingSegments: string[];
};

export type PendingDirectory = {
  id: string;
  name: string;
};
