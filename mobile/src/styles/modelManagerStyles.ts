import type { ImageStyle, TextStyle, ViewStyle } from "react-native";

export const modelManagerStyles = {
  modelPage: {
    flex: 1,
    backgroundColor: "#0d1015",
  },
  modelPageContent: {
    padding: 16,
    gap: 12,
  },
  modelHeader: {
    gap: 6,
  },
  sectionTitle: {
    color: "#f4f8ff",
    fontSize: 22,
    fontWeight: "900",
  },
  sectionText: {
    color: "#a8b3c4",
    fontSize: 13,
    lineHeight: 19,
  },
  runtimePanel: {
    borderWidth: 1,
    borderColor: "#2c3542",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 12,
    gap: 10,
  },
  panelLabel: {
    color: "#e7edf7",
    fontSize: 13,
    fontWeight: "800",
  },
  segmentedControl: {
    flexDirection: "row",
    borderWidth: 1,
    borderColor: "#354253",
    borderRadius: 8,
    overflow: "hidden",
    minHeight: 42,
  },
  segmentButton: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#12161c",
    paddingHorizontal: 8,
  },
  segmentButtonActive: {
    backgroundColor: "#0e639c",
  },
  segmentButtonText: {
    color: "#a8b3c4",
    fontSize: 13,
    fontWeight: "800",
  },
  segmentButtonTextActive: {
    color: "#ffffff",
  },
  modelStatusPanel: {
    borderWidth: 1,
    borderColor: "#2c3542",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 12,
    gap: 10,
  },
  modelStatusRow: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 10,
  },
  modelStatusText: {
    flex: 1,
    color: "#a8b3c4",
    fontSize: 12,
    lineHeight: 17,
  },
  progressText: {
    color: "#f4f8ff",
    fontSize: 12,
    fontWeight: "800",
  },
  progressTrack: {
    height: 8,
    borderRadius: 4,
    backgroundColor: "#252d38",
    overflow: "hidden",
  },
  progressFill: {
    height: 8,
    borderRadius: 4,
    backgroundColor: "#56b6c2",
  },
  modelList: {
    gap: 10,
  },
  modelCard: {
    borderWidth: 1,
    borderColor: "#2c3542",
    borderRadius: 8,
    backgroundColor: "#111820",
    padding: 12,
    gap: 10,
  },
  modelCardSelected: {
    borderColor: "#7bc8ff",
  },
  modelCardHeader: {
    flexDirection: "row",
    alignItems: "flex-start",
    justifyContent: "space-between",
    gap: 10,
  },
  modelTitleBlock: {
    flex: 1,
    minWidth: 0,
    gap: 3,
  },
  modelTitle: {
    color: "#f4f8ff",
    fontSize: 15,
    fontWeight: "800",
  },
  modelMeta: {
    color: "#90a0b8",
    fontSize: 12,
  },
  installBadge: {
    color: "#9aa8bd",
    borderWidth: 1,
    borderColor: "#425066",
    borderRadius: 8,
    paddingHorizontal: 8,
    paddingVertical: 5,
    fontSize: 11,
    fontWeight: "800",
    overflow: "hidden",
  },
  installBadgeActive: {
    color: "#dafbe1",
    borderColor: "#3b7f55",
    backgroundColor: "#193323",
  },
  modelDescription: {
    color: "#a8b3c4",
    fontSize: 12,
    lineHeight: 17,
  },
  modelActions: {
    flexDirection: "row",
    gap: 8,
  },
  smallButton: {
    minHeight: 38,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#425066",
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: 12,
    backgroundColor: "#18202a",
  },
  smallButtonActive: {
    borderColor: "#7bc8ff",
    backgroundColor: "#203348",
  },
  smallButtonPrimary: {
    backgroundColor: "#0e639c",
    borderColor: "#0e639c",
  },
  smallButtonText: {
    color: "#d9e4f2",
    fontSize: 13,
    fontWeight: "800",
  },
  smallButtonTextActive: {
    color: "#ffffff",
  },
  smallButtonPrimaryText: {
    color: "#ffffff",
    fontSize: 13,
    fontWeight: "800",
  },
  sourceButton: {
    minHeight: 42,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#425066",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#151922",
  },
  sourceButtonText: {
    color: "#d9e4f2",
    fontSize: 13,
    fontWeight: "800",
  },
} satisfies Record<string, ViewStyle | TextStyle | ImageStyle>;
