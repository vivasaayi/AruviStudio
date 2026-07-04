import { describe, expect, it } from "vitest";

import { parseBooleanSetting } from "./settingsKeys";

describe("parseBooleanSetting", () => {
  it("parses common true and false values", () => {
    expect(parseBooleanSetting("true", false)).toBe(true);
    expect(parseBooleanSetting("1", false)).toBe(true);
    expect(parseBooleanSetting("on", false)).toBe(true);
    expect(parseBooleanSetting("false", true)).toBe(false);
    expect(parseBooleanSetting("0", true)).toBe(false);
    expect(parseBooleanSetting("off", true)).toBe(false);
  });

  it("falls back for missing or unrecognized values", () => {
    expect(parseBooleanSetting(null, true)).toBe(true);
    expect(parseBooleanSetting(undefined, false)).toBe(false);
    expect(parseBooleanSetting("sometimes", true)).toBe(true);
  });
});
