import { mergeProps } from "solid-js";

export default function TruncatedText(rawProps: {
  text: string;
  class?: string;
  placement?: "above" | "below";
  align?: "start" | "end";
}) {
  const props = mergeProps({ placement: "below" as const, align: "start" as const }, rawProps);
  const positionClass = () => {
    const vertical = props.placement === "above" ? "bottom-full mb-1" : "top-full mt-1";
    const horizontal = props.align === "end" ? "right-0" : "left-0";
    return `${vertical} ${horizontal}`;
  };
  return (
    <span class="group/tooltip relative block min-w-0">
      <span class={`block truncate ${props.class ?? ""}`}>{props.text}</span>
      <span
        class={`pointer-events-none absolute z-20 w-max max-w-xs whitespace-normal wrap-break-word rounded-lg border border-line bg-ink px-2 py-1 text-[11px] font-medium leading-snug text-surface opacity-0 shadow-sm transition group-hover/tooltip:opacity-100 ${positionClass()}`}
      >
        {props.text}
      </span>
    </span>
  );
}
