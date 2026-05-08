import { describe, expect, it } from "vitest";
import { initialRequest } from "./domain";
import type { CredentialTestResult, EumdacSidecarStatus, PowerDataset, ProductList } from "./types";
import {
  deleteSavedDataset,
  estimatePv,
  estimatePvWatts,
  exportDataset,
  exportSavedDataset,
  fetchPowerDataset,
  invoke,
  listSavedDatasets,
  loadSavedDataset,
  saveDataset,
} from "./tauri";

describe("browser demo IPC fallback", () => {
  it("returns a coherent NASA POWER dataset and PV estimates", async () => {
    const dataset = await fetchPowerDataset(initialRequest);

    expect(dataset.request).toEqual(initialRequest);
    expect(dataset.records).toHaveLength(5);
    expect(dataset.units.ALLSKY_SFC_SW_DWN).toBe("kWh/m^2/day");

    const estimate = await estimatePv({
      dataset,
      capacityKw: 10,
      irradianceParameter: "ALLSKY_SFC_SW_DWN",
      lossesPercent: 14,
      inverterEfficiencyPercent: 96,
    });
    expect(estimate.usedRecordCount).toBe(dataset.records.length);
    expect(estimate.missingRecordCount).toBe(0);
    expect(estimate.energyKwh).toBeGreaterThan(0);

    const pvWatts = await estimatePvWatts({
      latitude: initialRequest.latitude,
      longitude: initialRequest.longitude,
      systemCapacityKw: 10,
      tiltDegrees: 30,
      azimuthDegrees: 180,
      lossesPercent: 14,
      moduleType: 0,
      arrayType: 1,
      timeframe: "monthly",
    });
    expect(pvWatts.method).toContain("PVWatts");
    expect(pvWatts.acAnnualKwh).toBeGreaterThan(0);
  });

  it("keeps saved dataset commands round-trippable", async () => {
    const dataset = await fetchPowerDataset(initialRequest);
    const saved = await saveDataset("Demo roundtrip dataset", dataset);

    try {
      const directExport = await exportDataset(dataset, "csv");
      expect(directExport.format).toBe("csv");
      expect(directExport.bytes).toBeGreaterThan(0);

      const list = await listSavedDatasets();
      expect(list.some((item) => item.id === saved.id)).toBe(true);

      const loaded = await loadSavedDataset(saved.id);
      expect(loaded.records).toHaveLength(dataset.records.length);

      const exported = await exportSavedDataset(saved.id, "json");
      expect(exported.format).toBe("json");
      expect(exported.bytes).toBeGreaterThan(0);
    } finally {
      await deleteSavedDataset(saved.id);
    }
  });

  it("keeps credential and satellite sidecar commands aligned with UI expectations", async () => {
    const slot = "nlr_pvwatts_key";

    expect(await invoke<boolean>("has_api_key", { name: slot })).toBe(false);
    await invoke<void>("store_api_key", { name: slot, value: "demo-key" });
    expect(await invoke<boolean>("has_api_key", { name: slot })).toBe(true);

    const credential = await invoke<CredentialTestResult>("test_api_key", { name: slot });
    expect(credential).toMatchObject({ slot, ok: true });

    await invoke<void>("delete_api_key", { name: slot });
    expect(await invoke<boolean>("has_api_key", { name: slot })).toBe(false);

    const sidecar = await invoke<EumdacSidecarStatus>("get_eumdac_sidecar_status");
    expect(sidecar).toMatchObject({ found: false, trusted: false });
    expect(await invoke<boolean>("check_eumdac_sidecar")).toBe(false);

    const products = await invoke<ProductList>("fetch_eumetsat_products", {
      query: {
        collectionId: "EO:EUM:DAT:MSG:HRSEVIRI",
        bbox: "-10 35 30 60",
        startTime: "2024-05-01T00:00:00Z",
        endTime: "2024-05-01T01:00:00Z",
        limit: 1,
      },
    });
    expect(products.products[0]?.id).toContain("EO:EUM:DAT:MSG:HRSEVIRI");

    const download = await invoke("download_eumetsat_product", {
      collectionId: "EO:EUM:DAT:MSG:HRSEVIRI",
      productId: products.products[0]?.id,
      outputDir: "/demo/downloads",
    });
    expect(download).toMatchObject({
      collectionId: "EO:EUM:DAT:MSG:HRSEVIRI",
      outputDir: "/demo/downloads",
    });
  });

  it("supports NDVI and rejects unknown demo commands", async () => {
    const message = await invoke<string>("validate_ndvi_inputs", {
      job: {
        redPath: "red.tif",
        nirPath: "nir.tif",
        outputPath: "ndvi.tif",
        redScale: 1,
        nirScale: 1,
      },
    });
    expect(message).toContain("NDVI job is structurally valid");

    const result = await invoke("run_ndvi", {
      job: {
        redPath: "red.tif",
        nirPath: "nir.tif",
        outputPath: "ndvi.tif",
        redScale: 1,
        nirScale: 1,
      },
    });
    expect(result).toMatchObject({ outputPath: "ndvi.tif", georeferencingPreserved: true });

    await expect(invoke<PowerDataset>("unknown_command")).rejects.toThrow("Demo command not implemented");
  });
});
