import type { ImageStyle, TextStyle, ViewStyle } from "react-native";

export const productModalStyles = {
  productModalOverlay: {
    flex: 1,
    justifyContent: "flex-end",
    backgroundColor: "rgba(0,0,0,0.58)",
  },
  productModal: {
    maxHeight: "72%",
    borderTopLeftRadius: 16,
    borderTopRightRadius: 16,
    borderWidth: 1,
    borderColor: "#2f3948",
    backgroundColor: "#10151d",
    paddingTop: 14,
  },
  productModalHeader: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 12,
    paddingHorizontal: 16,
    paddingBottom: 12,
    borderBottomWidth: 1,
    borderBottomColor: "#242b35",
  },
  productModalTitleBlock: {
    flex: 1,
    minWidth: 0,
    gap: 2,
  },
  productModalTitle: {
    color: "#f4f8ff",
    fontSize: 18,
    fontWeight: "900",
  },
  productModalMeta: {
    color: "#8f9caf",
    fontSize: 12,
    fontWeight: "800",
  },
  productModalClose: {
    minHeight: 34,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#384657",
    backgroundColor: "#172231",
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: 12,
  },
  productModalCloseText: {
    color: "#eaf2fb",
    fontSize: 12,
    fontWeight: "900",
  },
  productModalList: {
    padding: 12,
    gap: 8,
  },
  productModalRow: {
    minHeight: 76,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#2f3948",
    backgroundColor: "#111820",
    flexDirection: "row",
    alignItems: "center",
    gap: 10,
    padding: 12,
  },
  productModalRowActive: {
    borderColor: "#4aa3d8",
    backgroundColor: "#123149",
  },
  productModalRowCopy: {
    flex: 1,
    minWidth: 0,
    gap: 4,
  },
  productModalRowTitle: {
    color: "#f4f8ff",
    fontSize: 15,
    fontWeight: "900",
  },
  productModalRowSummary: {
    color: "#98a5b7",
    fontSize: 12,
    lineHeight: 17,
    fontWeight: "700",
  },
  productModalRowStatus: {
    color: "#9fcaf0",
    fontSize: 11,
    fontWeight: "900",
    textTransform: "uppercase",
  },
} satisfies Record<string, ViewStyle | TextStyle | ImageStyle>;
