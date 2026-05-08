import type { PowerRequest } from "./types";

export const availableParams = [
  "ALLSKY_SFC_SW_DWN",
  "T2M",
  "WS2M",
  "RH2M",
  "PRECTOTCORR",
  "PS",
  "T2M_MAX",
  "T2M_MIN",
];

export const quickExamples = [
  { name: "New York", lat: 40.7128, lon: -74.006 },
  { name: "Los Angeles", lat: 34.0522, lon: -118.2437 },
  { name: "London", lat: 51.5072, lon: -0.1276 },
  { name: "Tokyo", lat: 35.6762, lon: 139.6503 },
  { name: "Sydney", lat: -33.8688, lon: 151.2093 },
];

export const apiSlots = [
  { name: "eumetsat_consumer_key", label: "EUMETSAT Key", type: "password" },
  { name: "eumetsat_consumer_secret", label: "EUMETSAT Secret", type: "password" },
  { name: "nlr_pvwatts_key", label: "PVWatts/NLR Key", type: "password" },
] as const;

export const initialRequest: PowerRequest = {
  latitude: 40.7128,
  longitude: -74.006,
  startDate: "2024-05-01",
  endDate: "2024-05-31",
  parameters: ["ALLSKY_SFC_SW_DWN", "T2M", "WS2M"],
  temporal: "daily",
  community: "RE",
  timeStandard: "LST",
};

export function timestamp() {
  return new Date().toLocaleTimeString("en-GB");
}

export function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function formatNumber(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "-";
  }
  return Math.abs(value) >= 100 ? value.toFixed(1) : value.toFixed(2);
}

export function compactUnit(unit: string | undefined) {
  if (!unit) return "";
  return unit
    .replace("kWh/m^2/day", "kWh/m2/day")
    .replace("kW-hr/m^2/day", "kWh/m2/day")
    .replace("Wh/m^2", "Wh/m2");
}

export function toFiniteNumber(value: string, fallback: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}
