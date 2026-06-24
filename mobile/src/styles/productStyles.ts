import { productEmptyStyles } from "./productEmptyStyles";
import { productModalStyles } from "./productModalStyles";
import { productPlannerStyles } from "./productPlannerStyles";
import { productShellStyles } from "./productShellStyles";
import { productTreeStyles } from "./productTreeStyles";

export const productStyles = {
  ...productShellStyles,
  ...productModalStyles,
  ...productTreeStyles,
  ...productPlannerStyles,
  ...productEmptyStyles,
};
