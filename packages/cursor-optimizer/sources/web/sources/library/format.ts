const PREFIX_LABELS: Record<string, string> = {
  "agentKv:blob": "Agent message data",
  agentKv: "Agent data",
  composerData: "Chat sessions",
  bubbleId: "Chat messages",
  messageRequestContext: "Request context",
  checkpointId: "Editor checkpoints",
  ItemTable: "Workspace items",
  history: "History entries",
  workbench: "Workbench state",
};

export function formatNumber(value: number): string {
  return Number(value).toLocaleString("en-US");
}

function humanize(prefix: string): string {
  const spaced = prefix
    .replace(/[:_]/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .toLowerCase()
    .trim();
  if (!spaced) {
    return "Other";
  }
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

export function friendlyPrefix(prefix: string): string {
  if (PREFIX_LABELS[prefix]) {
    return PREFIX_LABELS[prefix];
  }
  return humanize(prefix);
}

export function percentOf(part: number, whole: number): number {
  if (whole <= 0) {
    return 0;
  }
  return Math.min(100, Math.round((part / whole) * 100));
}
