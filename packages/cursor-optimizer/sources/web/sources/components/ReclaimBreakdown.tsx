import TruncatedText from "./TruncatedText";

export default function ReclaimBreakdown(props: {
  dataLabel: string;
  dataHuman: string;
  compactionHuman: string;
}) {
  return (
    <div class="mt-4 grid grid-cols-2 gap-3">
      <div class="min-w-0 rounded-lg border border-line bg-surface p-3">
        <TruncatedText
          text={props.dataHuman}
          class="text-base font-semibold text-ink max-[560px]:text-sm"
          placement="above"
        />
        <div class="mt-0.5 text-xs text-muted">{props.dataLabel}</div>
      </div>
      <div class="min-w-0 rounded-lg border border-line bg-surface p-3">
        <TruncatedText
          text={`≥ ${props.compactionHuman}`}
          class="text-base font-semibold text-ink max-[560px]:text-sm"
          placement="above"
        />
        <div class="mt-0.5 text-xs text-muted">storage compaction (at least)</div>
      </div>
    </div>
  );
}
