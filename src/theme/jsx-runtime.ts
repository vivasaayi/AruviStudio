import { Fragment, jsx as reactJsx, jsxs as reactJsxs } from "react/jsx-runtime";
import type * as React from "react";
import { applyLightThemeToProps } from "./lightTheme";

export { Fragment };

export function jsx(type: React.ElementType, props: Record<string, unknown>, key?: string) {
  return reactJsx(type, applyLightThemeToProps(props), key);
}

export function jsxs(type: React.ElementType, props: Record<string, unknown>, key?: string) {
  return reactJsxs(type, applyLightThemeToProps(props), key);
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
