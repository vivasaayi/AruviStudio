import type { ImageStyle, TextStyle, ViewStyle } from "react-native";

export const productPlannerStyles = {
  productPlannerPanel: {
    borderWidth: 1,
    borderColor: "#334154",
    borderRadius: 8,
    backgroundColor: "#0e141c",
    padding: 10,
    gap: 8,
  },
  productPlannerHeader: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 10,
  },
  productPlannerCopy: {
    flex: 1,
    minWidth: 0,
    gap: 2,
  },
  productPlannerTitle: {
    color: "#f4f8ff",
    fontSize: 13,
    fontWeight: "900",
  },
  productPlannerStatus: {
    color: "#8f9caf",
    fontSize: 11,
    fontWeight: "800",
  },
  productPlannerIconButton: {
    minHeight: 30,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#384657",
    backgroundColor: "#172231",
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: 10,
  },
  productPlannerIconText: {
    color: "#eaf2fb",
    fontSize: 11,
    fontWeight: "900",
  },
  productPlannerInput: {
    minHeight: 46,
    maxHeight: 90,
    borderWidth: 1,
    borderColor: "#2f3948",
    borderRadius: 8,
    backgroundColor: "#111820",
    color: "#f4f8ff",
    paddingHorizontal: 10,
    paddingVertical: 8,
    fontSize: 13,
    lineHeight: 18,
    fontWeight: "700",
  },
  productPlannerActions: {
    flexDirection: "row",
    gap: 8,
  },
  productPlannerAction: {
    flex: 1,
    minHeight: 36,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#39485c",
    backgroundColor: "#1a2330",
    alignItems: "center",
    justifyContent: "center",
  },
  productPlannerActionRecording: {
    borderColor: "#d65f5f",
    backgroundColor: "#8d3030",
  },
  productPlannerActionPrimary: {
    borderColor: "#2f8fc8",
    backgroundColor: "#123149",
  },
  productPlannerActionText: {
    color: "#eaf2fb",
    fontSize: 12,
    fontWeight: "900",
  },
  productPlannerPrimaryText: {
    color: "#eef8ff",
    fontSize: 12,
    fontWeight: "900",
  },
  productPlannerReply: {
    color: "#b9c6d8",
    fontSize: 12,
    lineHeight: 17,
    fontWeight: "700",
  },
  productPlannerTraceList: {
    gap: 4,
  },
  productPlannerTraceItem: {
    color: "#85c9f5",
    fontSize: 10,
    fontWeight: "900",
  },
} satisfies Record<string, ViewStyle | TextStyle | ImageStyle>;
