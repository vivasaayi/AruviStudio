import { describe, expect, it, vi } from "vitest";
import type { QueryClient, QueryKey } from "@tanstack/react-query";

import { refreshScopedProductQueries } from "./productQueryRefresh";

function makeQueryClient(overrides: Partial<QueryClient> = {}): QueryClient {
  return {
    cancelQueries: vi.fn(() => Promise.resolve()),
    invalidateQueries: vi.fn(() => Promise.resolve()),
    refetchQueries: vi.fn(() => Promise.resolve()),
    ...overrides,
  } as unknown as QueryClient;
}

describe("productQueryRefresh", () => {
  it("filters incomplete query keys and deduplicates repeated keys", async () => {
    const queryClient = makeQueryClient();

    await refreshScopedProductQueries(queryClient, [
      ["products"],
      ["productTree", null],
      ["productTree", undefined],
      ["products"],
      ["productTree", "product-1"],
    ] as QueryKey[]);

    expect(queryClient.cancelQueries).toHaveBeenCalledTimes(2);
    expect(queryClient.invalidateQueries).toHaveBeenCalledTimes(2);
    expect(queryClient.refetchQueries).toHaveBeenCalledTimes(2);
    expect(queryClient.refetchQueries).toHaveBeenNthCalledWith(
      1,
      { queryKey: ["products"], type: "active" },
      { throwOnError: true },
    );
    expect(queryClient.refetchQueries).toHaveBeenNthCalledWith(
      2,
      { queryKey: ["productTree", "product-1"], type: "active" },
      { throwOnError: true },
    );
  });

  it("returns the same in-flight refresh for the same scoped query set", async () => {
    let resolveCancel: () => void = () => {};
    const cancelPromise = new Promise<void>((resolve) => {
      resolveCancel = resolve;
    });
    const queryClient = makeQueryClient({
      cancelQueries: vi.fn(() => cancelPromise),
    });

    const firstRefresh = refreshScopedProductQueries(queryClient, [["products"]]);
    const secondRefresh = refreshScopedProductQueries(queryClient, [["products"]]);

    expect(secondRefresh).toBe(firstRefresh);
    expect(queryClient.cancelQueries).toHaveBeenCalledTimes(1);

    resolveCancel();
    await firstRefresh;

    expect(queryClient.invalidateQueries).toHaveBeenCalledTimes(1);
    expect(queryClient.refetchQueries).toHaveBeenCalledTimes(1);
  });
});
