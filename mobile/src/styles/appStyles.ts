import { StyleSheet } from "react-native";
import { callStyles } from "./callStyles";
import { modelManagerStyles } from "./modelManagerStyles";
import { productStyles } from "./productStyles";
import { shellStyles } from "./shellStyles";
import { voiceStyles } from "./voiceStyles";

export const styles = StyleSheet.create({
  ...shellStyles,
  ...productStyles,
  ...voiceStyles,
  ...callStyles,
  ...modelManagerStyles,
});
