import { createSignal } from "solid-js";

export type SelectOption = {
  label: string;
  value: string;
};

export type ConfirmField = {
  id: string;
  label: string;
  value: string;
  placeholder?: string;
  kind?: "text" | "backupPath" | "select";
  fileNameId?: string;
  fileNameLabel?: string;
  fileNameValue?: string;
  options?: SelectOption[];
};

export type ConfirmOptions = {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel?: string | null;
  danger?: boolean;
  fields?: ConfirmField[];
};

type ConfirmResult =
  | { confirmed: false; values: Record<string, string> }
  | { confirmed: true; values: Record<string, string> };

type PendingConfirm = ConfirmOptions & {
  resolve: (result: ConfirmResult) => void;
};

const [pendingConfirm, setPendingConfirm] = createSignal<PendingConfirm | null>(null);
const [confirmValues, setConfirmValues] = createSignal<Record<string, string>>({});

export const confirmState = {
  pendingConfirm,
  confirmValues,
};

export function confirmAction(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    showConfirm(options).then((result) => resolve(result.confirmed));
  });
}

export function confirmWithFields(options: ConfirmOptions): Promise<Record<string, string> | null> {
  return showConfirm(options).then((result) => (result.confirmed ? result.values : null));
}

function showConfirm(options: ConfirmOptions): Promise<ConfirmResult> {
  return new Promise((resolve) => {
    const values = Object.fromEntries(
      (options.fields ?? []).flatMap((field) => [
        [field.id, field.value],
        ...(field.fileNameId ? [[field.fileNameId, field.fileNameValue ?? ""]] : []),
      ]),
    );
    setConfirmValues(values);
    setPendingConfirm({
      ...options,
      cancelLabel: options.cancelLabel === undefined ? "Cancel" : options.cancelLabel,
      resolve,
    });
  });
}

export function updateConfirmValue(id: string, value: string): void {
  setConfirmValues((values) => ({ ...values, [id]: value }));
}

export function settleConfirm(confirmed: boolean): void {
  const current = pendingConfirm();
  if (!current) {
    return;
  }
  const values = confirmValues();
  setPendingConfirm(null);
  setConfirmValues({});
  current.resolve({ confirmed, values });
}
