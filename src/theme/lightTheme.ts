import type { CSSProperties } from "react";

type StyleValue = string | number | undefined | null;

const transformedStyles = new WeakMap<CSSProperties, CSSProperties>();

const neutralText: Record<string, string> = {
  "#ffffff": "#111827",
  "#fff": "#111827",
  "#f4f8ff": "#111827",
  "#f3f3f3": "#111827",
  "#f1f3f7": "#111827",
  "#edf1f8": "#111827",
  "#eef1f6": "#111827",
  "#e5e7eb": "#111827",
  "#e3e8f0": "#111827",
  "#e0e0e0": "#111827",
  "#dbe2ed": "#1f2937",
  "#d8dde6": "#1f2937",
  "#d8e1ef": "#1f2937",
  "#d7deea": "#1f2937",
  "#d7dbe3": "#1f2937",
  "#d4d4d4": "#1f2937",
  "#d2d7e0": "#1f2937",
  "#cfd6e4": "#374151",
  "#cfd5e2": "#374151",
  "#ced4de": "#374151",
  "#cccccc": "#374151",
  "#b9c0cf": "#4b5563",
  "#aeb7c5": "#4b5563",
  "#a7afbf": "#4b5563",
  "#9ea6b6": "#6b7280",
  "#9aa3b4": "#6b7280",
  "#99a1b1": "#6b7280",
  "#8f96a3": "#6b7280",
  "#7f8796": "#6b7280",
  "#666": "#6b7280",
};

const semanticText: Record<string, string> = {
  "#8fc8ff": "#0f5f9a",
  "#b9d3ff": "#1d4ed8",
  "#bcd3f1": "#1d4ed8",
  "#bed3ee": "#1d4ed8",
  "#cfe0f7": "#1d4ed8",
  "#d9e7fa": "#1d4ed8",
  "#dce9ff": "#1d4ed8",
  "#eaf5ff": "#ffffff",
  "#59d6b2": "#047857",
  "#4ec9b0": "#047857",
  "#61d48c": "#047857",
  "#8ff2bc": "#047857",
  "#d7ba7d": "#9a5b00",
  "#e7c77a": "#9a5b00",
  "#ff7b72": "#b42318",
  "#ff9b9b": "#b42318",
  "#ffb4b4": "#b42318",
};

const surfaceColors: Record<string, string> = {
  "#0f1115": "#f8fafc",
  "#101721": "#ffffff",
  "#111821": "#ffffff",
  "#121620": "#ffffff",
  "#131d29": "#f8fafc",
  "#141820": "#ffffff",
  "#141b24": "#ffffff",
  "#142437": "#e8f3ff",
  "#161920": "#f8fafc",
  "#162233": "#e8f3ff",
  "#163d2f": "#e8f8f0",
  "#17191d": "#f1f5f9",
  "#171a1f": "#f8fafc",
  "#171a20": "#f8fafc",
  "#171b22": "#f8fafc",
  "#172536": "#e8f3ff",
  "#173247": "#e8f3ff",
  "#181a1f": "#ffffff",
  "#181b20": "#ffffff",
  "#181c23": "#ffffff",
  "#182433": "#eff6ff",
  "#18456a": "#dbeafe",
  "#191c22": "#ffffff",
  "#1a1d22": "#ffffff",
  "#1a2230": "#eff6ff",
  "#1a2736": "#e8f3ff",
  "#1a2737": "#eff6ff",
  "#1b1d22": "#ffffff",
  "#1b1f27": "#ffffff",
  "#1b2028": "#ffffff",
  "#1b2130": "#ffffff",
  "#1b2330": "#eff6ff",
  "#1b2431": "#f8fafc",
  "#1b2a3c": "#e8f3ff",
  "#1c2733": "#e8f3ff",
  "#1c2027": "#ffffff",
  "#1d2025": "#ffffff",
  "#1d2128": "#f8fafc",
  "#1e1e1e": "#f8fafc",
  "#1f2329": "#ffffff",
  "#1f242d": "#ffffff",
  "#1f2a35": "#e8f3ff",
  "#20242a": "#ffffff",
  "#20252d": "#f8fafc",
  "#212327": "#ffffff",
  "#222938": "#ffffff",
  "#223147": "#e8f3ff",
  "#22344a": "#eff6ff",
  "#223851": "#e8f3ff",
  "#232834": "#ffffff",
  "#252526": "#ffffff",
  "#252b38": "#f8fafc",
  "#26292f": "#ffffff",
  "#262c36": "#f8fafc",
  "#272c34": "#f8fafc",
  "#2a2619": "#fff7e6",
  "#2a3140": "#eef2ff",
  "#2b1d22": "#fff1f2",
  "#2b2f37": "#f8fafc",
  "#2b313b": "#ffffff",
  "#2c3139": "#f8fafc",
  "#2d323a": "#f8fafc",
  "#323233": "#e5e7eb",
  "#37373d": "#dbeafe",
  "#444": "#e5e7eb",
};

const brandBackgrounds: Record<string, string> = {
  "#0e639c": "#2563eb",
  "#12304a": "#2563eb",
  "#123d5a": "#dbeafe",
  "#1f79b9": "#2f7ebb",
  "#2d6a3f": "#15803d",
  "#6c2020": "#dc2626",
  "#7a5b16": "#ca8a04",
  "#7d2a2a": "#dc2626",
  "#8b2d2d": "#dc2626",
};

const borderColors: Record<string, string> = {
  "#1e1e1e": "#d8dee8",
  "#232831": "#e5e7eb",
  "#233041": "#d8dee8",
  "#263142": "#d8dee8",
  "#293341": "#d8dee8",
  "#2a2f37": "#d8dee8",
  "#2a3340": "#d8dee8",
  "#2b3038": "#d8dee8",
  "#2c3139": "#d8dee8",
  "#2c3340": "#d8dee8",
  "#2d3138": "#d8dee8",
  "#2d3139": "#d8dee8",
  "#2d3442": "#d8dee8",
  "#2f343d": "#d8dee8",
  "#2f3540": "#d8dee8",
  "#2f3641": "#d8dee8",
  "#2f3643": "#d8dee8",
  "#30343c": "#d8dee8",
  "#303640": "#d8dee8",
  "#303742": "#d8dee8",
  "#313844": "#d8dee8",
  "#32353d": "#d8dee8",
  "#323f52": "#d8dee8",
  "#32445e": "#bfdbfe",
  "#333": "#d8dee8",
  "#333841": "#d8dee8",
  "#334152": "#d8dee8",
  "#35506f": "#93c5fd",
  "#36506f": "#bfdbfe",
  "#36516e": "#bfdbfe",
  "#38404d": "#d8dee8",
  "#384456": "#bfdbfe",
  "#38506f": "#bfdbfe",
  "#39404a": "#d8dee8",
  "#3a404a": "#d8dee8",
  "#3b4049": "#d8dee8",
  "#3c4048": "#d8dee8",
  "#3f4550": "#d8dee8",
  "#406183": "#93c5fd",
  "#434a55": "#d8dee8",
  "#444": "#d8dee8",
  "#5a2f35": "#fecdd3",
  "#5a5034": "#fde68a",
};

const activeTextOnSurface = new Set(["#123d5a", "#173247", "#18456a", "#1f2a35", "#1c2733", "#142437", "#172536"]);
const strongBackgrounds = new Set(["#0e639c", "#12304a", "#2d6a3f", "#6c2020", "#7a5b16", "#7d2a2a", "#8b2d2d"]);

export function applyLightThemeToProps<T extends { style?: CSSProperties } | null | undefined>(props: T): T {
  if (!props?.style || typeof props.style !== "object") {
    return props;
  }

  const nextStyle = transformStyle(props.style);
  if (nextStyle === props.style) {
    return props;
  }

  return { ...props, style: nextStyle };
}

function transformStyle(style: CSSProperties): CSSProperties {
  const cached = transformedStyles.get(style);
  if (cached) {
    return cached;
  }

  const backgroundToken = getBackgroundToken(style);
  const hasStrongBackground = backgroundToken ? strongBackgrounds.has(backgroundToken) : false;
  const hasActiveSurface = backgroundToken ? activeTextOnSurface.has(backgroundToken) : false;
  let changed = false;
  const next: CSSProperties = {};

  for (const [key, value] of Object.entries(style) as [keyof CSSProperties, StyleValue][]) {
    const transformed = transformStyleValue(key, value, hasStrongBackground, hasActiveSurface);
    next[key] = transformed as never;
    changed ||= transformed !== value;
  }

  const result = changed ? next : style;
  transformedStyles.set(style, result);
  return result;
}

function transformStyleValue(
  key: keyof CSSProperties,
  value: StyleValue,
  hasStrongBackground: boolean,
  hasActiveSurface: boolean,
): StyleValue {
  if (typeof value !== "string") {
    return value;
  }

  if (key === "boxShadow") {
    return value.replace(/rgba\(0,\s*0,\s*0,\s*(0?\.\d+|1(?:\.0)?)\)/gi, "rgba(15, 23, 42, 0.12)");
  }

  const property = String(key).toLowerCase();
  if (property.includes("border")) {
    return replaceColors(value, (token) => borderColors[token] ?? brandBackgrounds[token] ?? token);
  }

  if (property.includes("background")) {
    return replaceColors(value, (token) => brandBackgrounds[token] ?? surfaceColors[token] ?? transformRgbaSurface(token) ?? token);
  }

  if (property === "color") {
    if (hasStrongBackground && isWhiteish(value)) {
      return "#ffffff";
    }
    if (hasActiveSurface && isWhiteish(value)) {
      return "#0f172a";
    }
    return replaceColors(value, (token) => semanticText[token] ?? neutralText[token] ?? token);
  }

  return replaceColors(value, (token) => borderColors[token] ?? surfaceColors[token] ?? semanticText[token] ?? neutralText[token] ?? token);
}

function getBackgroundToken(style: CSSProperties): string | null {
  const raw = typeof style.backgroundColor === "string"
    ? style.backgroundColor
    : typeof style.background === "string"
      ? style.background
      : null;
  if (!raw) {
    return null;
  }
  return normalizeColorToken(raw.match(/#[0-9a-f]{3,8}\b/i)?.[0] ?? "");
}

function replaceColors(value: string, map: (token: string) => string): string {
  return value
    .replace(/#[0-9a-f]{3,8}\b/gi, (match) => map(normalizeColorToken(match)))
    .replace(/rgba?\([^)]+\)/gi, (match) => map(normalizeRgbToken(match)));
}

function normalizeColorToken(token: string): string {
  const lower = token.toLowerCase();
  if (/^#[0-9a-f]{3}$/i.test(lower)) {
    const [, r, g, b] = lower;
    return `#${r}${r}${g}${g}${b}${b}`;
  }
  return lower;
}

function normalizeRgbToken(token: string): string {
  return token.toLowerCase().replace(/\s+/g, "");
}

function isWhiteish(value: string): boolean {
  const token = normalizeColorToken(value.trim());
  return token === "#fff" || token === "#ffffff";
}

function transformRgbaSurface(token: string): string | null {
  if (token === "rgba(255,255,255,0.04)" || token === "rgba(255,255,255,0.06)" || token === "rgba(255,255,255,0.08)") {
    return "#ffffff";
  }
  if (token === "rgba(255,255,255,0.1)" || token === "rgba(255,255,255,0.12)") {
    return "#d8dee8";
  }
  if (token === "rgba(14,99,156,0.16)" || token === "rgba(14,99,156,0.18)") {
    return "rgba(37,99,235,0.12)";
  }
  if (token === "rgba(8,10,14,0.72)" || token === "rgba(5,8,12,0.72)") {
    return "rgba(15,23,42,0.28)";
  }
  if (token === "rgba(18,34,58,0.95)" || token === "rgba(18,29,42,0.95)") {
    return "rgba(239,246,255,0.96)";
  }
  if (token === "rgba(102,140,214,0.26)") {
    return "rgba(37,99,235,0.10)";
  }
  return null;
}
