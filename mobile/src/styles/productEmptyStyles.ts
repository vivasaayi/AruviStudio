import type { ImageStyle, TextStyle, ViewStyle } from "react-native";

export const productEmptyStyles = {
  productEmptyScreen: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#0d1015",
    padding: 22,
    gap: 12,
  },
  productEmptyBlock: {
    borderWidth: 1,
    borderColor: "#2f3948",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 14,
    gap: 6,
  },
  productEmptyTitle: {
    color: "#f4f8ff",
    fontSize: 16,
    fontWeight: "900",
    textAlign: "center",
  },
  productEmptyText: {
    color: "#9ca8ba",
    fontSize: 13,
    lineHeight: 19,
    fontWeight: "700",
    textAlign: "center",
  },
  productPrimaryAction: {
    minHeight: 42,
    borderRadius: 8,
    backgroundColor: "#0e639c",
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: 16,
  },
  productPrimaryActionText: {
    color: "#ffffff",
    fontSize: 14,
    fontWeight: "900",
  },
} satisfies Record<string, ViewStyle | TextStyle | ImageStyle>;
