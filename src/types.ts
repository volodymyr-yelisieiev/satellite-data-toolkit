export type Screen =
  | "dashboard"
  | "power"
  | "eumetsat"
  | "ndvi"
  | "pv"
  | "saved"
  | "api"
  | "settings"
  | "about";

export type Temporal = "daily" | "hourly";

export interface PowerRequest {
  latitude: number;
  longitude: number;
  startDate: string;
  endDate: string;
  parameters: string[];
  temporal: Temporal;
  community: string;
  timeStandard: "LST" | "UTC";
}

export interface PowerRecord {
  rawTimestamp?: string;
  timestamp: string;
  values: Record<string, number | null>;
}

export interface PowerDataset {
  request: PowerRequest;
  records: PowerRecord[];
  units: Record<string, string>;
  longNames: Record<string, string>;
  statusCode: number;
  apiVersion: string;
  timeStandard: string;
  fillValue: number;
  dataTimeSeconds: number;
  processTimeSeconds: number;
  fetchedAt: string;
}

export interface ActivityLogEntry {
  time: string;
  message: string;
}

export interface PvEstimateInput {
  dataset: PowerDataset;
  capacityKw: number;
  irradianceParameter: string;
  lossesPercent: number;
  inverterEfficiencyPercent: number;
}

export interface PvEstimate {
  energyKwh: number;
  averagePowerKw: number;
  capacityFactorPercent: number;
  performanceRatio: number;
  recordCount: number;
  usedRecordCount: number;
  missingRecordCount: number;
  unitMode: string;
  method: string;
  assumptions: string[];
}

export interface SavedDataset {
  id: string;
  name: string;
  kind: string;
  createdAt: string;
  recordCount: number;
}

export interface ExportResult {
  path: string;
  format: "csv" | "json";
  bytes: number;
}

export interface CredentialTestResult {
  slot: string;
  ok: boolean;
  message: string;
}

export interface NdviJob {
  redPath: string;
  nirPath: string;
  outputPath: string;
  redScale: number;
  nirScale: number;
  nodataValue?: number;
}

export interface NdviResult {
  outputPath: string;
  width: number;
  height: number;
  validPixelCount: number;
  nodataPixelCount: number;
  min: number | null;
  max: number | null;
  mean: number | null;
  georeferencingPreserved: boolean;
  warnings: string[];
}

export interface EumetsatQuery {
  collectionId: string;
  bbox: string;
  startTime: string;
  endTime: string;
  limit: number;
}

export interface EumetsatProduct {
  id: string;
  title: string;
  raw: unknown;
}

export interface ProductList {
  products: EumetsatProduct[];
  rawOutput: string;
}

export interface DownloadResult {
  collectionId: string;
  productId: string;
  outputDir: string;
  stdout: string;
  stderr: string;
}

export interface PvWattsRequest {
  latitude: number;
  longitude: number;
  systemCapacityKw: number;
  tiltDegrees: number;
  azimuthDegrees: number;
  lossesPercent: number;
  moduleType: number;
  arrayType: number;
  timeframe: string;
}

export interface PvWattsResult {
  acAnnualKwh: number;
  solradAnnualKwhPerM2Day: number;
  capacityFactorPercent: number;
  stationInfo: unknown;
  warnings: string[];
  method: string;
}
