export type InitializeInfo = {
  databasePath: string | null;
  baseDirectory: string | null;
  databaseExists: boolean;
  cursorRunning: boolean;
  version: string;
};

export type QuickSummary = {
  databaseBytes: number;
  databaseHuman: string;
  writeAheadLogBytes: number;
  writeAheadLogHuman: string;
  totalBytes: number;
  totalHuman: string;
};

export type KeyGroup = {
  prefix: string;
  rowCount: number;
  bytes: number;
  human: string;
};

export type OverviewResult = {
  storage: {
    reclaimableBytes: number;
    reclaimableHuman: string;
  };
  agentBlobs: { count: number };
  keyPrefixes: KeyGroup[];
};

export type LightCleanAnalysis = {
  compactionReclaimBytes: number;
  compactionReclaimHuman: string;
  estimatedReclaimBytes: number;
  estimatedReclaimHuman: string;
};

export type LightCleanResult = {
  deletedRows: number;
  beforeHuman: string;
  afterHuman: string;
  reclaimedHuman: string;
};

export type DeepCleanAnalysis = {
  cutoffDays: number;
  matchingEntries: number;
  estimatedBytes: number;
  estimatedHuman: string;
  compactionReclaimBytes: number;
  compactionReclaimHuman: string;
  totalReclaimBytes: number;
  totalReclaimHuman: string;
};

export type DeepCleanResult = {
  deletedConversationRows: number;
  beforeHuman: string;
  afterHuman: string;
  reclaimedHuman: string;
};

export type BackupResult = {
  path: string;
  human: string;
  originalHuman: string;
  ratio: string;
};
export type IntegrityResult = { result: string; healthy: boolean };
export type CheckpointResult = { beforeHuman: string; afterHuman: string };
export type VacuumResult = { beforeHuman: string; afterHuman: string; reclaimedHuman: string };
export type FlushDatabaseResult = {
  beforeDatabaseHuman: string;
  afterDatabaseHuman: string;
  beforeWriteAheadLogHuman: string;
  afterWriteAheadLogHuman: string;
  reclaimedHuman: string;
};

export type StatRow = { label: string; value: string; kind?: "path" };

export type StorageEstimate = {
  databaseBytes: number;
  databaseHuman: string;
  writeAheadLogBytes: number;
  writeAheadLogHuman: string;
  availableBytes: number | null;
  availableHuman: string | null;
  backupDirectory: string;
  backupFileName: string;
};

export type { DirectoryEntry, DirectoryListing } from "@cursor-optimizer/user-interface/types";
