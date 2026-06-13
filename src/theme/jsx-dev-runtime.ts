import { Fragment, jsxDEV as reactJsxDEV } from "react/jsx-dev-runtime";
import type * as React from "react";
import { applyLightThemeToProps } from "./lightTheme";

export { Fragment };

export function jsxDEV(
  type: React.ElementType,
  props: Record<string, unknown>,
  key: string | undefined,
  isStaticChildren: boolean,
  source: Parameters<typeof reactJsxDEV>[4],
  self: unknown,
) {
  return reactJsxDEV(type, applyLightThemeToProps(props), key, isStaticChildren, source, self);
}

export namespace JSX {
  export type ElementType = React.JSX.ElementType;
  export type Element = React.JSX.Element;
  export type ElementClass = React.JSX.ElementClass;
  export type ElementAttributesProperty = React.JSX.ElementAttributesProperty;
  export type ElementChildrenAttribute = React.JSX.ElementChildrenAttribute;
  export type LibraryManagedAttributes<C, P> = React.JSX.LibraryManagedAttributes<C, P>;
  export type IntrinsicAttributes = React.JSX.IntrinsicAttributes;
  export type IntrinsicClassAttributes<T> = React.JSX.IntrinsicClassAttributes<T>;
  export type IntrinsicElements = React.JSX.IntrinsicElements;
}
