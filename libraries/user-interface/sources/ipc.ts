type PendingEntry = {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  onProgress?: (text: string) => void;
};

const pendingRequests = new Map<number, PendingEntry>();
let nextRequestId = 1;

declare global {
  interface Window {
    ipc: { postMessage: (message: string) => void };
    __resolve: (requestId: number, succeeded: boolean, payload: unknown) => void;
    __progress: (requestId: number, text: string) => void;
  }
}

export type PendingRequest<ResultType> = {
  id: number;
  promise: Promise<ResultType>;
};

export function startRequest<ResultType>(
  action: string,
  params: Record<string, unknown> = {},
  onProgress?: (text: string) => void,
): PendingRequest<ResultType> {
  const id = nextRequestId;
  nextRequestId += 1;
  const promise = new Promise<ResultType>((resolve, reject) => {
    pendingRequests.set(id, {
      resolve: (value: unknown) => resolve(value as ResultType),
      reject,
      onProgress,
    });
  });
  window.ipc.postMessage(JSON.stringify({ requestId: id, action, params }));
  return { id, promise };
}

export function callBackend<ResultType>(
  action: string,
  params: Record<string, unknown> = {},
): Promise<ResultType> {
  return startRequest<ResultType>(action, params).promise;
}

export function cancelRequest(targetId: number): void {
  const id = nextRequestId;
  nextRequestId += 1;
  window.ipc.postMessage(JSON.stringify({ requestId: id, action: "cancel", params: { targetId } }));
}

window.__resolve = (requestId, succeeded, payload) => {
  const entry = pendingRequests.get(requestId);
  if (!entry) {
    return;
  }
  pendingRequests.delete(requestId);
  if (succeeded) {
    entry.resolve(payload);
  } else {
    const failure = payload as { message?: string } | null;
    const message = failure?.message;
    entry.reject(new Error(message != null && message !== "" ? message : "Something went wrong"));
  }
};

window.__progress = (requestId, text) => {
  const entry = pendingRequests.get(requestId);
  entry?.onProgress?.(text);
};
