import React from "react";
import { Pressable, Text, View } from "react-native";
import { styles } from "../styles/appStyles";

export type MobileTabId = "planner" | "products" | "voice" | "models" | "calls" | "activity";

export const MOBILE_TABS: Array<{ id: MobileTabId; label: string }> = [
  { id: "planner", label: "Planner" },
  { id: "products", label: "Products" },
  { id: "voice", label: "Voice" },
  { id: "models", label: "Models" },
  { id: "calls", label: "Calls" },
  { id: "activity", label: "Activity" },
];

type MobileBottomTabsProps = {
  activeTab: MobileTabId;
  isHidden: boolean;
  onSwitchTab: (nextTab: MobileTabId) => void;
};

export function MobileBottomTabs({ activeTab, isHidden, onSwitchTab }: MobileBottomTabsProps) {
  if (isHidden) {
    return null;
  }

  return (
    <View style={styles.bottomTabs}>
      {MOBILE_TABS.map((tab) => (
        <Pressable
          key={tab.id}
          style={[styles.tabItem, activeTab === tab.id && styles.tabItemActive]}
          onPress={() => onSwitchTab(tab.id)}
        >
          <View style={[styles.tabIndicator, activeTab === tab.id && styles.tabIndicatorActive]} />
          <Text style={[styles.tabText, activeTab === tab.id && styles.tabTextActive]} numberOfLines={1}>
            {tab.label}
          </Text>
        </Pressable>
      ))}
    </View>
  );
}
