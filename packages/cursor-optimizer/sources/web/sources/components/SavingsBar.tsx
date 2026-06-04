import { Show } from "solid-js";

import { percentOf } from "../library/format";
import TruncatedText from "./TruncatedText";

export default function SavingsBar(props: {
  reclaimBytes: number;
  totalBytes: number;
  reclaimHuman: string;
  totalHuman: string;
  atLeast?: boolean;
}) {
  const hasTotal = () => props.totalBytes > 0 && props.totalHuman.length > 0;
  const ratio = () => percentOf(props.reclaimBytes, props.totalBytes);
  return (
    <div class="fade-rise min-w-0">
      <div class="flex items-end justify-between gap-3">
        <div class="min-w-0">
          <TruncatedText
            text={props.atLeast ? `≥ ${props.reclaimHuman}` : props.reclaimHuman}
            class="text-3xl font-bold tracking-tight text-ink max-[560px]:text-2xl"
            placement="above"
          />
          <div class="text-xs text-muted">
            {props.atLeast ? "estimated space freed (at least)" : "estimated space freed"}
          </div>
        </div>
        <Show when={hasTotal()}>
          <div class="shrink-0 text-right">
            <div class="text-lg font-semibold text-accent">{ratio()}%</div>
            <div class="max-w-32 truncate text-xs text-faint">of {props.totalHuman}</div>
          </div>
        </Show>
      </div>
      <div class="mt-3 h-3 w-full overflow-hidden rounded-full bg-subtle">
        <div
          class="bar-grow h-full rounded-full bg-accent"
          style={{ width: `${hasTotal() ? ratio() : 0}%` }}
        />
      </div>
    </div>
  );
}
