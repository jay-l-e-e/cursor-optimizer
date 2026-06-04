import { confirmAction, confirmWithFields } from "./confirmStore";
import { runTrackedOperation } from "./operationStore";
import { describeStorageRisk, loadStorageEstimate } from "./storageEstimate";

import type { ConfirmOptions } from "./confirmStore";
import type {
  BackupResult,
  CheckpointResult,
  IntegrityResult,
  StatRow,
  StorageEstimate,
  VacuumResult,
} from "./types";

export type MiniTool = {
  icon: string;
  title: string;
  description: string;
  writes: boolean;
  requiresWriteAheadLog?: boolean;
  confirm?: (estimate: StorageEstimate | null) => ConfirmOptions;
  run: (values?: Record<string, string>) => Promise<StatRow[] | null>;
};

export const miniTools: MiniTool[] = [
  {
    icon: "bi-shield-check",
    title: "Create backup",
    description: "Save a compressed, dated copy of the current state.",
    writes: false,
    confirm: (estimate) => ({
      title: "Create a backup?",
      message: `Choose where to save the backup. ${describeStorageRisk(
        estimate,
        estimate?.databaseBytes ?? null,
      )}`,
      confirmLabel: "Next",
      fields: [
        {
          id: "backupDirectory",
          kind: "backupPath",
          label: "Backup path",
          value: estimate?.backupDirectory ?? "",
          fileNameId: "backupFileName",
          fileNameLabel: "File name",
          fileNameValue: estimate?.backupFileName ?? "",
        },
      ],
    }),
    run: async (values) => {
      const compressionValues = await confirmWithFields({
        title: "Compression level",
        message: "Higher levels are slower but produce smaller files.",
        confirmLabel: "Create backup",
        fields: [
          {
            id: "compressionLevel",
            kind: "select",
            label: "Compression",
            value: "10",
            options: [
              { label: "Fast", value: "3" },
              { label: "Balanced", value: "10" },
              { label: "Maximum", value: "22" },
            ],
          },
        ],
      });
      if (compressionValues === null) {
        return null;
      }
      await runTrackedOperation<BackupResult>({
        action: "toolBackup",
        access: "read",
        progressTitle: "Compressing",
        params: {
          backupDirectory: values?.backupDirectory ?? "",
          backupFileName: values?.backupFileName ?? "",
          compressionLevel: compressionValues.compressionLevel ?? "10",
        },
        resultRows: (data) => [
          { label: "Size", value: `${data.originalHuman} → ${data.human} (${data.ratio})` },
          { label: "Saved to", value: data.path, kind: "path" },
        ],
      });
      return null;
    },
  },
  {
    icon: "bi-heart-pulse",
    title: "Integrity check",
    description: "Verify that nothing is damaged.",
    writes: false,
    run: () =>
      runTrackedOperation<IntegrityResult>({
        action: "toolIntegrityCheck",
        access: "read",
        progressTitle: "Checking",
        resultRows: (data) => [{ label: "Status", value: data.healthy ? "Healthy" : data.result }],
      }).then(() => null),
  },
  {
    icon: "bi-arrow-down-square",
    title: "Flush pending writes",
    description: "Apply buffered changes to the main file.",
    writes: true,
    requiresWriteAheadLog: true,
    run: () =>
      runTrackedOperation<CheckpointResult>({
        action: "toolCheckpoint",
        progressTitle: "Flushing",
        resultRows: (data) => [
          { label: "Log before", value: data.beforeHuman },
          { label: "Log after", value: data.afterHuman },
        ],
      }).then(() => null),
  },
  {
    icon: "bi-arrows-collapse",
    title: "Compact file",
    description: "Reorganize storage to free unused space.",
    writes: true,
    confirm: (estimate) => ({
      title: "Compact storage?",
      message: `This reorganizes storage to free unused space. Some temporary disk space is needed. ${describeStorageRisk(
        estimate,
        estimate?.databaseBytes ?? null,
      )}`,
      confirmLabel: "Compact now",
    }),
    run: () =>
      runTrackedOperation<VacuumResult>({
        action: "toolVacuum",
        progressTitle: "Compacting",
        resultRows: (data) => [
          { label: "Before", value: data.beforeHuman },
          { label: "After", value: data.afterHuman },
          { label: "Freed", value: data.reclaimedHuman },
        ],
      }).then(() => null),
  },
  {
    icon: "bi-speedometer",
    title: "Refresh statistics",
    description: "Rebuild lookup statistics for faster queries. Your conversations are untouched.",
    writes: true,
    run: () =>
      runTrackedOperation<unknown>({
        action: "toolAnalyze",
        progressTitle: "Refreshing",
        resultRows: () => [{ label: "Result", value: "Statistics refreshed" }],
      }).then(() => null),
  },
];

export async function offerPreBackup(): Promise<boolean> {
  const wantsBackup = await confirmAction({
    title: "Create a backup first?",
    message: "This operation is destructive. You can save a compressed backup before proceeding.",
    confirmLabel: "Back up first",
    cancelLabel: "Skip",
  });
  if (!wantsBackup) {
    return true;
  }
  const estimate = await loadStorageEstimate();
  const pathValues = await confirmWithFields({
    title: "Create a backup?",
    message: `Choose where to save the backup. ${describeStorageRisk(
      estimate,
      estimate?.databaseBytes ?? null,
    )}`,
    confirmLabel: "Next",
    fields: [
      {
        id: "backupDirectory",
        kind: "backupPath",
        label: "Backup path",
        value: estimate?.backupDirectory ?? "",
        fileNameId: "backupFileName",
        fileNameLabel: "File name",
        fileNameValue: estimate?.backupFileName ?? "",
      },
    ],
  });
  if (pathValues === null) {
    return false;
  }
  const compressionValues = await confirmWithFields({
    title: "Compression level",
    message: "Higher levels are slower but produce smaller files.",
    confirmLabel: "Create backup",
    fields: [
      {
        id: "compressionLevel",
        kind: "select",
        label: "Compression",
        value: "10",
        options: [
          { label: "Fast", value: "3" },
          { label: "Balanced", value: "10" },
          { label: "Maximum", value: "22" },
        ],
      },
    ],
  });
  if (compressionValues === null) {
    return false;
  }
  const result = await runTrackedOperation<BackupResult>({
    action: "toolBackup",
    access: "read",
    progressTitle: "Compressing",
    params: {
      backupDirectory: pathValues.backupDirectory ?? "",
      backupFileName: pathValues.backupFileName ?? "",
      compressionLevel: compressionValues.compressionLevel ?? "10",
    },
    resultRows: (data) => [
      { label: "Size", value: `${data.originalHuman} → ${data.human} (${data.ratio})` },
      { label: "Saved to", value: data.path, kind: "path" },
    ],
  });
  return result !== null;
}

export async function executeTool(tool: MiniTool): Promise<void> {
  let values: Record<string, string> | undefined;
  if (tool.confirm) {
    const estimate = await loadStorageEstimate();
    const confirmedValues = await confirmWithFields(tool.confirm(estimate));
    if (confirmedValues === null) {
      return;
    }
    values = confirmedValues;
  }
  await tool.run(values);
}
