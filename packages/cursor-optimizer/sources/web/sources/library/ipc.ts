import {
  startRequest as baseStartRequest,
  callBackend,
  cancelRequest,
} from "@cursor-optimizer/user-interface/ipc";

import { idleProgress, parseProgress } from "./progress";

import type { PendingRequest } from "@cursor-optimizer/user-interface/ipc";
import type { Progress } from "./progress";

export type { PendingRequest };
export { callBackend, cancelRequest };

export function startRequest<ResultType>(
  action: string,
  params: Record<string, unknown> = {},
  onProgress?: (progress: Progress) => void,
): PendingRequest<ResultType> {
  const wrappedProgress = onProgress
    ? (text: string) => {
        const structured = parseProgress(text);
        onProgress(structured ?? { ...idleProgress, active: true, label: text });
      }
    : undefined;
  return baseStartRequest<ResultType>(action, params, wrappedProgress);
}
