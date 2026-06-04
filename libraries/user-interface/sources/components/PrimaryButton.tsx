export default function PrimaryButton(props: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={props.disabled}
      onClick={() => props.onClick()}
      class="rounded-full bg-accent px-6 py-2.5 text-[13px] font-semibold text-surface transition hover:bg-accent-strong disabled:cursor-not-allowed disabled:opacity-50"
    >
      {props.label}
    </button>
  );
}
