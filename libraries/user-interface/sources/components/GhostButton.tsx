export default function GhostButton(props: { label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={() => props.onClick()}
      class="rounded-full border border-line bg-surface px-6 py-2.5 text-[13px] font-semibold text-ink transition hover:bg-subtle"
    >
      {props.label}
    </button>
  );
}
