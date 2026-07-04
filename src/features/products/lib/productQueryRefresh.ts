import type { QueryClient, QueryKey } from "@tanstack/react-query";

const inFlightRefreshes = new WeakMap<QueryClient, Map<string, Promise<void>>>();

function queryKeySignature(queryKey: QueryKey): string {
  return JSON.stringify(queryKey);
}

function scopedQueryKeys(queryKeys: QueryKey[]): QueryKey[] {
  const seen = new Set<string>();
  const scoped: QueryKey[] = [];

  for (const queryKey of queryKeys) {
    if (queryKey.some((part) => part === null || part === undefined)) {
      continue;
    }
    const signature = queryKeySignature(queryKey);
    if (seen.has(signature)) {
      continue;
    }
    seen.add(signature);
    scoped.push(queryKey);
  }

  return scoped;
}

async function runRefresh(queryClient: QueryClient, queryKeys: QueryKey[]): Promise<void> {
  await Promise.all(
    queryKeys.map((queryKey) => queryClient.cancelQueries({ queryKey, type: "active" })),
  );
  await Promise.all(
    queryKeys.map(async (queryKey) => {
      await queryClient.invalidateQueries({ queryKey, refetchType: "none" });
      await queryClient.refetchQueries({ queryKey, type: "active" }, { throwOnError: true });
    }),
  );
}

export function refreshScopedProductQueries(
  queryClient: QueryClient,
  queryKeys: QueryKey[],
): Promise<void> {
  const queryKeysToRefresh = scopedQueryKeys(queryKeys);
  if (queryKeysToRefresh.length === 0) {
    return Promise.resolve();
  }

  const refreshSignature = queryKeysToRefresh.map(queryKeySignature).sort().join("\n");
  let clientRefreshes = inFlightRefreshes.get(queryClient);
  if (!clientRefreshes) {
    clientRefreshes = new Map();
    inFlightRefreshes.set(queryClient, clientRefreshes);
  }

  const activeRefresh = clientRefreshes.get(refreshSignature);
  if (activeRefresh) {
    return activeRefresh;
  }

  const refresh = runRefresh(queryClient, queryKeysToRefresh).finally(() => {
    clientRefreshes.delete(refreshSignature);
    if (clientRefreshes.size === 0) {
      inFlightRefreshes.delete(queryClient);
    }
  });
  clientRefreshes.set(refreshSignature, refresh);
  return refresh;
}
