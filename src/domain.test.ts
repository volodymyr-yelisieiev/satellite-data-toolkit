import { describe, expect, it } from "vitest";
import { defaultAppSettings, compactUnit, formatNumber, initialRequest, normalizeAppSettings, toFiniteNumber } from "./domain";

describe("domain helpers", () => {
  it("formats missing and numeric values consistently", () => {
    expect(formatNumber(null)).toBe("-");
    expect(formatNumber(Number.NaN)).toBe("-");
    expect(formatNumber(4.567)).toBe("4.57");
    expect(formatNumber(101.22)).toBe("101.2");
  });

  it("compacts NASA POWER irradiance units for dense tables", () => {
    expect(compactUnit("kW-hr/m^2/day")).toBe("kWh/m2/day");
    expect(compactUnit("Wh/m^2")).toBe("Wh/m2");
    expect(compactUnit(undefined)).toBe("");
  });

  it("falls back for non-finite form values", () => {
    expect(toFiniteNumber("42.5", 10)).toBe(42.5);
    expect(toFiniteNumber("", 10)).toBe(0);
    expect(toFiniteNumber("not-a-number", 10)).toBe(10);
  });

  it("keeps the default NASA POWER request aligned with table columns", () => {
    expect(initialRequest.parameters).toContain("ALLSKY_SFC_SW_DWN");
    expect(initialRequest.temporal).toBe("daily");
  });

  it("normalizes stored app settings", () => {
    expect(normalizeAppSettings({ startupScreen: "pv", previewRows: 24 })).toEqual({
      startupScreen: "pv",
      previewRows: 24,
    });
    expect(normalizeAppSettings({ startupScreen: "missing", previewRows: 999 })).toEqual(defaultAppSettings);
    expect(normalizeAppSettings(null)).toEqual(defaultAppSettings);
  });
});
