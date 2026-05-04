import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type {
  CredentialTestResult,
  DownloadResult,
  EumetsatQuery,
  ExportResult,
  NdviJob,
  NdviResult,
  PowerDataset,
  PowerRequest,
  ProductList,
  PvEstimate,
  PvEstimateInput,
  PvWattsRequest,
  PvWattsResult,
  SavedDataset,
} from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const demoSavedDatasets: Array<SavedDataset & { dataset: PowerDataset }> = [];
const demoApiSlots = new Set<string>();

const sampleDataset = (request: PowerRequest): PowerDataset => ({
  request,
  records: [
    { rawTimestamp: "20240501", timestamp: "2024-05-01", values: { ALLSKY_SFC_SW_DWN: 6.3082, T2M: 13.13, WS2M: 2.6 } },
    { rawTimestamp: "20240502", timestamp: "2024-05-02", values: { ALLSKY_SFC_SW_DWN: 6.7711, T2M: 15.01, WS2M: 3.32 } },
    { rawTimestamp: "20240503", timestamp: "2024-05-03", values: { ALLSKY_SFC_SW_DWN: 4.6524, T2M: 11.64, WS2M: 4.39 } },
    { rawTimestamp: "20240504", timestamp: "2024-05-04", values: { ALLSKY_SFC_SW_DWN: 2.7413, T2M: 10.94, WS2M: 4.22 } },
    { rawTimestamp: "20240505", timestamp: "2024-05-05", values: { ALLSKY_SFC_SW_DWN: 1.2036, T2M: 10.67, WS2M: 4.75 } },
  ],
  units: {
    ALLSKY_SFC_SW_DWN: "kWh/m^2/day",
    T2M: "C",
    WS2M: "m/s",
  },
  longNames: {
    ALLSKY_SFC_SW_DWN: "All Sky Surface Shortwave Downward Irradiance",
    T2M: "Temperature at 2 Meters",
    WS2M: "Wind Speed at 2 Meters",
  },
  statusCode: 200,
  apiVersion: "demo",
  timeStandard: request.timeStandard,
  fillValue: -999,
  dataTimeSeconds: 0.33,
  processTimeSeconds: 0.01,
  fetchedAt: new Date().toISOString(),
});

async function demoInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((resolve) => window.setTimeout(resolve, 250));
  if (command === "fetch_power_dataset") {
    return sampleDataset(args?.request as PowerRequest) as T;
  }
  if (command === "estimate_pv") {
    const input = args?.input as PvEstimateInput;
    const performanceRatio = (1 - input.lossesPercent / 100) * (input.inverterEfficiencyPercent / 100);
    const energyKwh = input.dataset.records.reduce((sum, record) => {
      const value = record.values[input.irradianceParameter] ?? 0;
      return sum + input.capacityKw * value * performanceRatio;
    }, 0);
    return {
      energyKwh,
      averagePowerKw: energyKwh / Math.max(input.dataset.records.length * 24, 1),
      capacityFactorPercent: (energyKwh / Math.max(input.capacityKw * input.dataset.records.length * 24, 1)) * 100,
      performanceRatio,
      recordCount: input.dataset.records.length,
      usedRecordCount: input.dataset.records.length,
      missingRecordCount: 0,
      unitMode: "daily_kwh_per_m2",
      method: "Local quick estimate",
      assumptions: ["Demo mode uses daily NASA POWER kWh/m^2/day values."],
    } satisfies PvEstimate as T;
  }
  if (command === "estimate_pvwatts") {
    return {
      acAnnualKwh: 14520,
      solradAnnualKwhPerM2Day: 4.6,
      capacityFactorPercent: 16.6,
      stationInfo: { demo: true },
      warnings: ["Demo PVWatts response; native mode uses the stored PVWatts/NLR key."],
      method: "PVWatts V8/NLR demo",
    } satisfies PvWattsResult as T;
  }
  if (command === "list_saved_datasets") {
    return demoSavedDatasets.map(({ dataset: _dataset, ...item }) => item) as T;
  }
  if (command === "load_saved_dataset") {
    const item = demoSavedDatasets.find((saved) => saved.id === args?.id);
    if (!item) throw new Error("saved dataset not found");
    return item.dataset as T;
  }
  if (command === "save_dataset") {
    const dataset = args?.dataset as PowerDataset;
    const item = {
      id: crypto.randomUUID(),
      name: String(args?.name ?? "Demo dataset"),
      kind: "nasa_power",
      createdAt: new Date().toISOString(),
      recordCount: dataset.records.length,
      dataset,
    };
    demoSavedDatasets.unshift(item);
    return item as T;
  }
  if (command === "delete_saved_dataset") {
    const index = demoSavedDatasets.findIndex((item) => item.id === args?.id);
    if (index >= 0) demoSavedDatasets.splice(index, 1);
    return undefined as T;
  }
  if (command === "export_dataset" || command === "export_saved_dataset") {
    return {
      path: "/demo/exports/nasa_power_demo.csv",
      format: args?.format === "json" ? "json" : "csv",
      bytes: 1024,
    } satisfies ExportResult as T;
  }
  if (command === "run_ndvi") {
    const job = args?.job as NdviJob;
    return {
      outputPath: job.outputPath,
      width: 2,
      height: 2,
      validPixelCount: 3,
      nodataPixelCount: 1,
      min: -0.1,
      max: 0.7,
      mean: 0.35,
      georeferencingPreserved: false,
      warnings: ["Demo NDVI response."],
    } satisfies NdviResult as T;
  }
  if (command === "validate_ndvi_inputs") {
    return "NDVI job is structurally valid." as T;
  }
  if (command === "store_api_key") {
    demoApiSlots.add(String(args?.name));
    return undefined as T;
  }
  if (command === "delete_api_key") {
    demoApiSlots.delete(String(args?.name));
    return undefined as T;
  }
  if (command === "has_api_key") {
    return demoApiSlots.has(String(args?.name)) as T;
  }
  if (command === "test_api_key") {
    const slot = String(args?.name);
    return {
      slot,
      ok: demoApiSlots.has(slot),
      message: demoApiSlots.has(slot) ? "Demo credential is stored." : "No credential stored for this slot.",
    } satisfies CredentialTestResult as T;
  }
  if (command === "check_eumdac_sidecar") {
    return false as T;
  }
  if (command === "fetch_eumetsat_products") {
    const query = args?.query as EumetsatQuery;
    return {
      products: [
        {
          id: `${query.collectionId}:demo-product`,
          title: "Demo EUMETSAT Product",
          raw: { demo: true },
        },
      ],
      rawOutput: "demo",
    } satisfies ProductList as T;
  }
  if (command === "download_eumetsat_product") {
    return {
      collectionId: String(args?.collectionId),
      productId: String(args?.productId),
      outputDir: String(args?.outputDir),
      stdout: "demo download complete",
      stderr: "",
    } satisfies DownloadResult as T;
  }
  throw new Error(`Demo command not implemented: ${command}`);
}

export function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    return tauriInvoke<T>(command, args);
  }
  return demoInvoke<T>(command, args);
}

export async function fetchPowerDataset(request: PowerRequest): Promise<PowerDataset> {
  return invoke<PowerDataset>("fetch_power_dataset", { request });
}

export async function estimatePv(input: PvEstimateInput): Promise<PvEstimate> {
  return invoke<PvEstimate>("estimate_pv", { input });
}

export async function estimatePvWatts(request: PvWattsRequest): Promise<PvWattsResult> {
  return invoke<PvWattsResult>("estimate_pvwatts", { request });
}

export async function saveDataset(name: string, dataset: PowerDataset): Promise<SavedDataset> {
  return invoke<SavedDataset>("save_dataset", { name, dataset });
}

export async function exportDataset(dataset: PowerDataset, format: "csv" | "json"): Promise<ExportResult> {
  return invoke<ExportResult>("export_dataset", { dataset, format });
}

export async function listSavedDatasets(): Promise<SavedDataset[]> {
  return invoke<SavedDataset[]>("list_saved_datasets");
}

export async function loadSavedDataset(id: string): Promise<PowerDataset> {
  return invoke<PowerDataset>("load_saved_dataset", { id });
}

export async function deleteSavedDataset(id: string): Promise<void> {
  return invoke<void>("delete_saved_dataset", { id });
}

export async function exportSavedDataset(id: string, format: "csv" | "json"): Promise<ExportResult> {
  return invoke<ExportResult>("export_saved_dataset", { id, format });
}
