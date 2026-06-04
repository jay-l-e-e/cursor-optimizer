import { TitleBar as SharedTitleBar } from "@cursor-optimizer/user-interface";

import { confirmAction } from "../library/confirmStore";

function showCloseBlockedReason(): void {
  void confirmAction({
    title: "Task in progress",
    message:
      "A task is running that needs to complete. Closing now could leave data in an inconsistent state.",
    confirmLabel: "Keep open",
    cancelLabel: null,
  });
}

export default function TitleBar(props: { closeBlocked: boolean }) {
  return (
    <SharedTitleBar
      showMaximize
      closeBlocked={props.closeBlocked}
      onCloseBlocked={showCloseBlockedReason}
    />
  );
}
