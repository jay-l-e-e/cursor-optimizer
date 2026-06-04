import { FileBrowserField as SharedFileBrowserField } from "@cursor-optimizer/user-interface";

import { updateConfirmValue } from "../library/confirmStore";

export default function FileBrowserField(props: {
  directoryFieldId: string;
  fileNameFieldId: string;
  fileNameLabel: string;
  initialDirectory: string;
  initialFileName: string;
}) {
  return (
    <SharedFileBrowserField
      initialDirectory={props.initialDirectory}
      initialFileName={props.initialFileName}
      fileNameLabel={props.fileNameLabel}
      showFileName
      onDirectoryChange={(directory) => updateConfirmValue(props.directoryFieldId, directory)}
      onFileNameChange={(fileName) => updateConfirmValue(props.fileNameFieldId, fileName)}
    />
  );
}
