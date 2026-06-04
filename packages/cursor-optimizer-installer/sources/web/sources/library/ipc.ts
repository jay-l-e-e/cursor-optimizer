import {
  startRequest as baseStartRequest,
  callBackend,
} from "@cursor-optimizer/user-interface/ipc";

import type { PendingRequest } from "@cursor-optimizer/user-interface/ipc";

export { callBackend };

export function startRequest<ResultType>(
  action: string,
  params: Record<string, unknown> = {},
  onProgress?: (text: string) => void,
): Promise<ResultType> {
  const request: PendingRequest<ResultType> = baseStartRequest(action, params, onProgress);
  return request.promise;
}
