import {
  BarChart3,
  Calendar,
  CheckCircle2,
  CloudDownload,
  Database,
  Download,
  ExternalLink,
  Eye,
  FileJson,
  Gauge,
  GitBranch,
  Globe2,
  Home,
  Info,
  KeyRound,
  Leaf,
  Loader2,
  RotateCcw,
  Satellite,
  Save,
  Search,
  Settings,
  ShieldCheck,
  Trash2,
  Zap,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type {
  ActivityLogEntry,
  CredentialTestResult,
  EumetsatProduct,
  EumetsatQuery,
  NdviJob,
  NdviResult,
  PowerDataset,
  PowerRequest,
  PvEstimate,
  PvWattsResult,
  SavedDataset,
  Screen,
} from "./types";
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

const navItems: Array<{ id: Screen; title: string; subtitle: string; icon: typeof Home }> = [
  { id: "dashboard", title: "Dashboard", subtitle: "Overview", icon: Home },
  { id: "power", title: "NASA POWER", subtitle: "Solar & Meteorological Data", icon: Globe2 },
  { id: "eumetsat", title: "EUMETSAT", subtitle: "Satellite Products", icon: Satellite },
  { id: "ndvi", title: "NDVI Calculator", subtitle: "Vegetation Index", icon: Leaf },
  { id: "pv", title: "PV Power Estimate", subtitle: "Energy Production", icon: BarChart3 },
  { id: "saved", title: "Saved Data", subtitle: "View & Manage", icon: Database },
  { id: "api", title: "API Slots", subtitle: "Manage API Keys", icon: KeyRound },
  { id: "settings", title: "Settings", subtitle: "Application Settings", icon: Settings },
  { id: "about", title: "About", subtitle: "About This App", icon: Info },
];

const powerTabs = [
  { id: "power" as Screen, title: "NASA POWER", icon: Globe2 },
  { id: "eumetsat" as Screen, title: "EUMETSAT", icon: Satellite },
  { id: "ndvi" as Screen, title: "NDVI CALCULATOR", icon: Leaf },
  { id: "pv" as Screen, title: "PV ESTIMATE", icon: BarChart3 },
];

const availableParams = [
  "ALLSKY_SFC_SW_DWN",
  "T2M",
  "WS2M",
  "RH2M",
  "PRECTOTCORR",
  "PS",
  "T2M_MAX",
  "T2M_MIN",
];

const quickExamples = [
  { name: "New York", lat: 40.7128, lon: -74.006 },
  { name: "Los Angeles", lat: 34.0522, lon: -118.2437 },
  { name: "London", lat: 51.5072, lon: -0.1276 },
  { name: "Tokyo", lat: 35.6762, lon: 139.6503 },
  { name: "Sydney", lat: -33.8688, lon: 151.2093 },
];

const apiSlots = [
  { name: "eumetsat_consumer_key", label: "EUMETSAT Key", type: "password" },
  { name: "eumetsat_consumer_secret", label: "EUMETSAT Secret", type: "password" },
  { name: "nlr_pvwatts_key", label: "PVWatts/NLR Key", type: "password" },
];

const initialRequest: PowerRequest = {
  latitude: 40.7128,
  longitude: -74.006,
  startDate: "2024-05-01",
  endDate: "2024-05-31",
  parameters: ["ALLSKY_SFC_SW_DWN", "T2M", "WS2M"],
  temporal: "daily",
  community: "RE",
  timeStandard: "LST",
};

function timestamp() {
  return new Date().toLocaleTimeString("en-GB");
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function formatNumber(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "-";
  }
  return Math.abs(value) >= 100 ? value.toFixed(1) : value.toFixed(2);
}

function compactUnit(unit: string | undefined) {
  if (!unit) return "";
  return unit.replace("kWh/m^2/day", "kWh/m2/day").replace("kW-hr/m^2/day", "kWh/m2/day").replace("Wh/m^2", "Wh/m2");
}

function App() {
  const [active, setActive] = useState<Screen>("power");
  const [request, setRequest] = useState<PowerRequest>(initialRequest);
  const [dataset, setDataset] = useState<PowerDataset | null>(null);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState<"idle" | "success" | "error">("idle");
  const [error, setError] = useState("");
  const [logs, setLogs] = useState<ActivityLogEntry[]>([
    { time: timestamp(), message: "Application ready" },
    { time: timestamp(), message: "NASA POWER defaults loaded" },
  ]);
  const [previewLimit, setPreviewLimit] = useState(12);
  const [lastExportPath, setLastExportPath] = useState("");
  const [pvCapacity, setPvCapacity] = useState(100);
  const [pvLosses, setPvLosses] = useState(14);
  const [pvInverter, setPvInverter] = useState(96);
  const [pvTilt, setPvTilt] = useState(30);
  const [pvAzimuth, setPvAzimuth] = useState(180);
  const [pvEstimate, setPvEstimate] = useState<PvEstimate | null>(null);
  const [pvWattsResult, setPvWattsResult] = useState<PvWattsResult | null>(null);
  const [apiStatus, setApiStatus] = useState<Record<string, boolean>>({});
  const [savedCount, setSavedCount] = useState(0);

  const tableColumns = useMemo(() => dataset?.request.parameters ?? request.parameters, [dataset, request.parameters]);

  useEffect(() => {
    void refreshApiStatus();
    void refreshSavedCount();
  }, []);

  function addLog(message: string) {
    setLogs((current) => [...current.slice(-80), { time: timestamp(), message }]);
  }

  async function refreshSavedCount() {
    const saved = await listSavedDatasets();
    setSavedCount(saved.length);
  }

  function updateRequest<K extends keyof PowerRequest>(key: K, value: PowerRequest[K]) {
    setRequest((current) => ({ ...current, [key]: value }));
  }

  function toggleParameter(param: string) {
    setRequest((current) => {
      const exists = current.parameters.includes(param);
      const parameters = exists ? current.parameters.filter((item) => item !== param) : [...current.parameters, param];
      return { ...current, parameters };
    });
  }

  function applyExample(lat: number, lon: number) {
    setRequest((current) => ({ ...current, latitude: lat, longitude: lon }));
    addLog(`Quick example applied: ${lat.toFixed(4)}, ${lon.toFixed(4)}`);
  }

  async function handleFetch() {
    setLoading(true);
    setStatus("idle");
    setError("");
    setPreviewLimit(12);
    addLog("Sending request to NASA POWER API...");
    try {
      const response = await fetchPowerDataset(request);
      setDataset(response);
      setStatus("success");
      addLog(`Response received successfully: ${response.records.length} records`);
      addLog("Data parsed and normalized");
    } catch (err) {
      const message = errorMessage(err);
      setError(message);
      setStatus("error");
      addLog(`Request failed: ${message}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleExport(format: "csv" | "json") {
    if (!dataset) return;
    addLog(`Exporting dataset as ${format.toUpperCase()}...`);
    try {
      const result = await exportDataset(dataset, format);
      setLastExportPath(result.path);
      addLog(`Export ${format.toUpperCase()} completed: ${result.path}`);
    } catch (err) {
      addLog(`Export failed: ${errorMessage(err)}`);
    }
  }

  async function handleSave() {
    if (!dataset) return;
    try {
      const saved = await saveDataset(`NASA POWER ${dataset.request.startDate} to ${dataset.request.endDate}`, dataset);
      await refreshSavedCount();
      addLog(`Dataset saved locally: ${saved.name}`);
    } catch (err) {
      addLog(`Save failed: ${errorMessage(err)}`);
    }
  }

  async function handlePvEstimate() {
    if (!dataset) {
      addLog("PV estimate requires a fetched NASA POWER dataset");
      setActive("power");
      return;
    }
    try {
      const estimate = await estimatePv({
        dataset,
        capacityKw: pvCapacity,
        irradianceParameter: "ALLSKY_SFC_SW_DWN",
        lossesPercent: pvLosses,
        inverterEfficiencyPercent: pvInverter,
      });
      setPvEstimate(estimate);
      addLog(`Local PV estimate completed: ${estimate.energyKwh.toFixed(2)} kWh`);
    } catch (err) {
      addLog(`PV estimate failed: ${errorMessage(err)}`);
    }
  }

  async function handlePvWatts() {
    const source = dataset?.request ?? request;
    try {
      const result = await estimatePvWatts({
        latitude: source.latitude,
        longitude: source.longitude,
        systemCapacityKw: pvCapacity,
        tiltDegrees: pvTilt,
        azimuthDegrees: pvAzimuth,
        lossesPercent: pvLosses,
        moduleType: 0,
        arrayType: 1,
        timeframe: "monthly",
      });
      setPvWattsResult(result);
      addLog(`PVWatts/NLR estimate completed: ${result.acAnnualKwh.toFixed(2)} kWh annual AC`);
    } catch (err) {
      addLog(`PVWatts/NLR estimate failed: ${errorMessage(err)}`);
    }
  }

  async function refreshApiStatus() {
    const entries = await Promise.all(
      apiSlots.map(async ({ name }) => [name, await invoke<boolean>("has_api_key", { name })] as const),
    );
    setApiStatus(Object.fromEntries(entries));
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <Satellite size={34} />
          </div>
          <div>
            <span>SATELLITE</span>
            <strong>DATA TOOLKIT</strong>
          </div>
          <small>v2.1.0</small>
        </div>

        <nav className="nav-list" aria-label="Main navigation">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                aria-current={active === item.id ? "page" : undefined}
                className={active === item.id ? "nav-item active" : "nav-item"}
                onClick={() => setActive(item.id)}
              >
                <Icon size={24} />
                <span>
                  <strong>{item.title}</strong>
                  <small>{item.subtitle}</small>
                </span>
              </button>
            );
          })}
        </nav>

        <div className="status-line">
          <span className="pulse" />
          <span>Ready</span>
        </div>
      </aside>

      <section className="workspace">
        <header className="top-tabs" role="tablist" aria-label="Primary workflows">
          {powerTabs.map((tab) => {
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                role="tab"
                aria-selected={active === tab.id}
                className={active === tab.id ? "top-tab active" : "top-tab"}
                onClick={() => setActive(tab.id)}
              >
                <Icon size={22} />
                {tab.title}
              </button>
            );
          })}
        </header>

        <div className="content-panel">
          {active === "dashboard" && <Dashboard dataset={dataset} savedCount={savedCount} setActive={setActive} />}
          {active === "power" && (
            <PowerScreen
              request={request}
              dataset={dataset}
              tableColumns={tableColumns}
              loading={loading}
              status={status}
              error={error}
              logs={logs}
              previewLimit={previewLimit}
              lastExportPath={lastExportPath}
              updateRequest={updateRequest}
              toggleParameter={toggleParameter}
              applyExample={applyExample}
              onFetch={handleFetch}
              onClear={() => {
                setDataset(null);
                setStatus("idle");
                setError("");
                setPvEstimate(null);
                setPvWattsResult(null);
                addLog("Response cleared");
              }}
              onExport={handleExport}
              onSave={handleSave}
              onPreviewMore={() => setPreviewLimit((current) => current + 24)}
              onClearLogs={() => setLogs([{ time: timestamp(), message: "Activity log cleared" }])}
            />
          )}
          {active === "eumetsat" && <EumetsatScreen addLog={addLog} />}
          {active === "ndvi" && <NdviScreen addLog={addLog} />}
          {active === "pv" && (
            <PvScreen
              dataset={dataset}
              request={request}
              capacity={pvCapacity}
              losses={pvLosses}
              inverter={pvInverter}
              tilt={pvTilt}
              azimuth={pvAzimuth}
              estimate={pvEstimate}
              pvWattsResult={pvWattsResult}
              setCapacity={setPvCapacity}
              setLosses={setPvLosses}
              setInverter={setPvInverter}
              setTilt={setPvTilt}
              setAzimuth={setPvAzimuth}
              onEstimate={handlePvEstimate}
              onPvWatts={handlePvWatts}
            />
          )}
          {active === "saved" && (
            <SavedScreen
              addLog={addLog}
              onSavedCountChange={setSavedCount}
              onPreview={(loaded) => {
                setDataset(loaded);
                setActive("power");
                addLog(`Loaded saved dataset: ${loaded.records.length} records`);
              }}
            />
          )}
          {active === "api" && <ApiScreen apiStatus={apiStatus} refreshApiStatus={refreshApiStatus} addLog={addLog} />}
          {active === "settings" && <SettingsScreen />}
          {active === "about" && <AboutScreen />}
        </div>

        <footer className="footer">
          <a href="https://github.com" target="_blank" rel="noreferrer">
            <GitBranch size={16} />
            GitHub
          </a>
          <a href="https://power.larc.nasa.gov/docs/services/api/" target="_blank" rel="noreferrer">
            <FileJson size={16} />
            Documentation
          </a>
          <button type="button" onClick={() => addLog("Updater is not configured for this local build")}>
            <CloudDownload size={16} />
            Check for Updates
          </button>
        </footer>
      </section>
    </main>
  );
}

function PowerScreen(props: {
  request: PowerRequest;
  dataset: PowerDataset | null;
  tableColumns: string[];
  loading: boolean;
  status: "idle" | "success" | "error";
  error: string;
  logs: ActivityLogEntry[];
  previewLimit: number;
  lastExportPath: string;
  updateRequest: <K extends keyof PowerRequest>(key: K, value: PowerRequest[K]) => void;
  toggleParameter: (param: string) => void;
  applyExample: (lat: number, lon: number) => void;
  onFetch: () => void;
  onClear: () => void;
  onExport: (format: "csv" | "json") => void;
  onSave: () => void;
  onPreviewMore: () => void;
  onClearLogs: () => void;
}) {
  const visibleRecords = props.dataset?.records.slice(0, props.previewLimit) ?? [];
  return (
    <>
      <section className="screen-heading">
        <div className="heading-icon">
          <Globe2 size={35} />
        </div>
        <div>
          <h1>NASA POWER</h1>
          <p>Access solar and meteorological data from NASA POWER API</p>
        </div>
        <a className="doc-button" href="https://power.larc.nasa.gov/docs/services/api/" target="_blank" rel="noreferrer">
          API Documentation
          <ExternalLink size={16} />
        </a>
      </section>

      <section className="power-grid">
        <div className="card request-card">
          <h2>1. Request Parameters</h2>
          <div className="field-grid two">
            <label>
              Latitude
              <input value={props.request.latitude} type="number" step="0.0001" onChange={(event) => props.updateRequest("latitude", Number(event.target.value))} />
            </label>
            <label>
              Longitude
              <input value={props.request.longitude} type="number" step="0.0001" onChange={(event) => props.updateRequest("longitude", Number(event.target.value))} />
            </label>
            <label>
              Start Date
              <span className="input-icon">
                <input value={props.request.startDate} type="date" onChange={(event) => props.updateRequest("startDate", event.target.value)} />
                <Calendar size={16} />
              </span>
            </label>
            <label>
              End Date
              <span className="input-icon">
                <input value={props.request.endDate} type="date" onChange={(event) => props.updateRequest("endDate", event.target.value)} />
                <Calendar size={16} />
              </span>
            </label>
          </div>

          <label className="wide-label">
            Parameters (select one or more)
            <div className="chip-box">
              {availableParams.map((param) => (
                <button key={param} aria-pressed={props.request.parameters.includes(param)} className={props.request.parameters.includes(param) ? "chip active" : "chip"} onClick={() => props.toggleParameter(param)} type="button">
                  {param}
                </button>
              ))}
            </div>
          </label>

          <div className="field-grid two">
            <label>
              Community
              <select value={props.request.community} onChange={(event) => props.updateRequest("community", event.target.value)}>
                <option value="RE">RE</option>
                <option value="SB">SB</option>
                <option value="AG">AG</option>
              </select>
            </label>
            <label>
              Time Format
              <select value={props.request.temporal} onChange={(event) => props.updateRequest("temporal", event.target.value as PowerRequest["temporal"])}>
                <option value="daily">DAILY</option>
                <option value="hourly">HOURLY</option>
              </select>
            </label>
          </div>

          <label className="wide-label">
            Time Standard
            <div className="segmented">
              {(["LST", "UTC"] as const).map((item) => (
                <button key={item} aria-pressed={props.request.timeStandard === item} className={props.request.timeStandard === item ? "active" : ""} type="button" onClick={() => props.updateRequest("timeStandard", item)}>
                  {item}
                </button>
              ))}
            </div>
          </label>

          <div className="action-row">
            <button className="primary-action" onClick={props.onFetch} type="button" disabled={props.loading}>
              {props.loading ? <Loader2 className="spin" size={18} /> : <CloudDownload size={18} />}
              Fetch Data
            </button>
            <button className="secondary-action" onClick={props.onClear} type="button">
              <RotateCcw size={18} />
              Clear
            </button>
          </div>

          <div className="quick-examples">
            <span>Quick Examples</span>
            <div>
              {quickExamples.map((example) => (
                <button key={example.name} type="button" onClick={() => props.applyExample(example.lat, example.lon)}>
                  {example.name}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="card response-card">
          <div className="section-title-row">
            <h2>2. Response</h2>
            {props.status === "success" && (
              <span className="status-badge success">
                <CheckCircle2 size={15} />
                Success
              </span>
            )}
            {props.status === "error" && <span className="status-badge error">Error</span>}
          </div>

          <div className="stat-strip">
            <Metric label="Records" value={props.dataset?.records.length.toString() ?? "0"} />
            <Metric label="Time (s)" value={props.dataset ? (props.dataset.dataTimeSeconds + props.dataset.processTimeSeconds).toFixed(2) : "0.00"} />
            <Metric label="Status Code" value={props.dataset?.statusCode.toString() ?? "-"} />
          </div>

          {props.error && <div className="error-box">{props.error}</div>}

          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Date</th>
                  {props.tableColumns.map((column) => (
                    <th key={column}>
                      {column}
                      <small>{compactUnit(props.dataset?.units[column])}</small>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {visibleRecords.map((record) => (
                  <tr key={`${record.rawTimestamp ?? record.timestamp}-${record.timestamp}`}>
                    <td>{record.timestamp}</td>
                    {props.tableColumns.map((column) => (
                      <td key={column}>{formatNumber(record.values[column])}</td>
                    ))}
                  </tr>
                ))}
                {!props.dataset && (
                  <tr>
                    <td colSpan={props.tableColumns.length + 1} className="empty-cell">
                      Fetch data to preview normalized NASA POWER records.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          <div className="response-actions">
            <button type="button" className="secondary-action" disabled={!props.dataset || visibleRecords.length >= (props.dataset?.records.length ?? 0)} onClick={props.onPreviewMore}>
              <Eye size={18} />
              Preview More
            </button>
            <span />
            <button type="button" className="secondary-action" disabled={!props.dataset} onClick={props.onSave}>
              <Save size={18} />
              Save
            </button>
            <button type="button" className="secondary-action" disabled={!props.dataset} onClick={() => props.onExport("csv")}>
              <Download size={18} />
              Export CSV
            </button>
            <button type="button" className="secondary-action" disabled={!props.dataset} onClick={() => props.onExport("json")}>
              <Download size={18} />
              Export JSON
            </button>
          </div>
          {props.lastExportPath && <p className="muted-result">Last export: {props.lastExportPath}</p>}
        </div>
      </section>

      <ActivityLog logs={props.logs} onClear={props.onClearLogs} />
    </>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function ActivityLog({ logs, onClear }: { logs: ActivityLogEntry[]; onClear: () => void }) {
  return (
    <section className="card log-card">
      <div className="section-title-row">
        <h2>3. Activity Log</h2>
        <button type="button" className="icon-button" onClick={onClear}>
          <Trash2 size={16} />
          Clear Log
        </button>
      </div>
      <div className="log-output">
        {logs.map((entry, index) => (
          <p key={`${entry.time}-${index}`}>
            [{entry.time}] {entry.message}
          </p>
        ))}
      </div>
    </section>
  );
}

function Dashboard({ dataset, savedCount, setActive }: { dataset: PowerDataset | null; savedCount: number; setActive: (screen: Screen) => void }) {
  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <Home size={34} />
        </div>
        <div>
          <h1>Dashboard</h1>
          <p>Operational overview for data, calculators, and exports</p>
        </div>
      </section>
      <div className="overview-grid">
        <DashboardTile icon={Globe2} label="NASA Records" value={dataset?.records.length.toString() ?? "0"} onClick={() => setActive("power")} />
        <DashboardTile icon={Database} label="Saved Datasets" value={String(savedCount)} onClick={() => setActive("saved")} />
        <DashboardTile icon={Gauge} label="PV Status" value={dataset ? "Ready" : "Need Data"} onClick={() => setActive("pv")} />
        <DashboardTile icon={ShieldCheck} label="Secrets" value="Keychain" onClick={() => setActive("api")} />
      </div>
    </section>
  );
}

function DashboardTile(props: { icon: typeof Home; label: string; value: string; onClick: () => void }) {
  const Icon = props.icon;
  return (
    <button className="dashboard-tile" type="button" onClick={props.onClick}>
      <Icon size={24} />
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </button>
  );
}

function EumetsatScreen({ addLog }: { addLog: (message: string) => void }) {
  const [query, setQuery] = useState<EumetsatQuery>({
    collectionId: "EO:EUM:DAT:METOP:OSI-104",
    bbox: "51.28,51.69,0.51,0.33",
    startTime: "2024-11-10T08:00:00",
    endTime: "2024-11-10T09:00:00",
    limit: 20,
  });
  const [sidecarAvailable, setSidecarAvailable] = useState<boolean | null>(null);
  const [products, setProducts] = useState<EumetsatProduct[]>([]);
  const [outputDir, setOutputDir] = useState("");
  const [selectedProduct, setSelectedProduct] = useState("");

  function update<K extends keyof EumetsatQuery>(key: K, value: EumetsatQuery[K]) {
    setQuery((current) => ({ ...current, [key]: value }));
  }

  async function checkSidecar() {
    const available = await invoke<boolean>("check_eumdac_sidecar");
    setSidecarAvailable(available);
    addLog(available ? "EUMDAC sidecar is available" : "EUMDAC sidecar is not bundled yet");
  }

  async function searchProducts() {
    try {
      const result = await invoke<{ products: EumetsatProduct[] }>("fetch_eumetsat_products", { query });
      setProducts(result.products);
      setSelectedProduct(result.products[0]?.id ?? "");
      addLog(`EUMETSAT search returned ${result.products.length} products`);
    } catch (err) {
      addLog(`EUMETSAT search failed: ${errorMessage(err)}`);
    }
  }

  async function downloadProduct() {
    try {
      await invoke("download_eumetsat_product", { collectionId: query.collectionId, productId: selectedProduct, outputDir });
      addLog(`EUMETSAT product download started: ${selectedProduct}`);
    } catch (err) {
      addLog(`EUMETSAT download failed: ${errorMessage(err)}`);
    }
  }

  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <Satellite size={34} />
        </div>
        <div>
          <h1>EUMETSAT</h1>
          <p>Discover and download satellite products through bundled EUMDAC tooling</p>
        </div>
      </section>
      <div className="card form-card">
        <div className="field-grid two">
          <label>
            Collection ID
            <input value={query.collectionId} onChange={(event) => update("collectionId", event.target.value)} />
          </label>
          <label>
            Bounding Box
            <input value={query.bbox} onChange={(event) => update("bbox", event.target.value)} />
          </label>
          <label>
            Start Time
            <input value={query.startTime} onChange={(event) => update("startTime", event.target.value)} />
          </label>
          <label>
            End Time
            <input value={query.endTime} onChange={(event) => update("endTime", event.target.value)} />
          </label>
          <label>
            Limit
            <input type="number" value={query.limit} onChange={(event) => update("limit", Number(event.target.value))} />
          </label>
          <label>
            Output Directory
            <input value={outputDir} placeholder="/path/to/downloads" onChange={(event) => setOutputDir(event.target.value)} />
          </label>
        </div>
        <div className="action-row left wrap">
          <button className="secondary-action" type="button" onClick={checkSidecar}>
            <Satellite size={18} />
            Check Sidecar
          </button>
          <button className="primary-action" type="button" onClick={searchProducts}>
            <Search size={18} />
            Search Products
          </button>
          {sidecarAvailable !== null && <span className={sidecarAvailable ? "status-badge success" : "status-badge error"}>{sidecarAvailable ? "Available" : "Missing"}</span>}
        </div>
      </div>
      <div className="card table-card">
        <table>
          <thead>
            <tr>
              <th>Product ID</th>
              <th>Title</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {products.map((product) => (
              <tr key={product.id}>
                <td>{product.id}</td>
                <td>{product.title}</td>
                <td>
                  <button className="secondary-action compact" type="button" onClick={() => setSelectedProduct(product.id)}>
                    Select
                  </button>
                </td>
              </tr>
            ))}
            {products.length === 0 && (
              <tr>
                <td colSpan={3} className="empty-cell">
                  Search results will appear here after EUMDAC is bundled and credentials are stored.
                </td>
              </tr>
            )}
          </tbody>
        </table>
        <div className="action-row left wrap">
          <input value={selectedProduct} placeholder="Selected product id" onChange={(event) => setSelectedProduct(event.target.value)} />
          <button className="primary-action" type="button" onClick={downloadProduct} disabled={!selectedProduct || !outputDir}>
            <Download size={18} />
            Download Selected
          </button>
        </div>
      </div>
    </section>
  );
}

function NdviScreen({ addLog }: { addLog: (message: string) => void }) {
  const [job, setJob] = useState<NdviJob>({
    redPath: "",
    nirPath: "",
    outputPath: "ndvi.tif",
    redScale: 1,
    nirScale: 1,
    nodataValue: -9999,
  });
  const [result, setResult] = useState<NdviResult | null>(null);
  const [summary, setSummary] = useState("");

  function update<K extends keyof NdviJob>(key: K, value: NdviJob[K]) {
    setJob((current) => ({ ...current, [key]: value }));
  }

  async function validate() {
    try {
      const response = await invoke<string>("validate_ndvi_inputs", { job });
      setSummary(response);
      addLog("NDVI inputs validated");
    } catch (err) {
      const message = errorMessage(err);
      setSummary(message);
      addLog(`NDVI validation failed: ${message}`);
    }
  }

  async function run() {
    try {
      const response = await invoke<NdviResult>("run_ndvi", { job });
      setResult(response);
      setSummary(`NDVI written to ${response.outputPath}`);
      addLog(`NDVI completed: ${response.validPixelCount} valid pixels`);
    } catch (err) {
      const message = errorMessage(err);
      setSummary(message);
      addLog(`NDVI run failed: ${message}`);
    }
  }

  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <Leaf size={34} />
        </div>
        <div>
          <h1>NDVI Calculator</h1>
          <p>Compute vegetation index from Red and NIR TIFF bands</p>
        </div>
      </section>
      <div className="card form-card">
        <div className="field-grid two">
          <label>
            Red Band TIFF
            <input value={job.redPath} placeholder="/path/to/red.tif" onChange={(event) => update("redPath", event.target.value)} />
          </label>
          <label>
            NIR Band TIFF
            <input value={job.nirPath} placeholder="/path/to/nir.tif" onChange={(event) => update("nirPath", event.target.value)} />
          </label>
          <label>
            Red Scale
            <input type="number" step="0.0001" value={job.redScale} onChange={(event) => update("redScale", Number(event.target.value))} />
          </label>
          <label>
            NIR Scale
            <input type="number" step="0.0001" value={job.nirScale} onChange={(event) => update("nirScale", Number(event.target.value))} />
          </label>
          <label>
            Output File
            <input value={job.outputPath} onChange={(event) => update("outputPath", event.target.value)} />
          </label>
          <label>
            Nodata Value
            <input type="number" value={job.nodataValue ?? -9999} onChange={(event) => update("nodataValue", Number(event.target.value))} />
          </label>
        </div>
        <div className="action-row left wrap">
          <button className="secondary-action" type="button" onClick={validate}>
            <Leaf size={18} />
            Validate
          </button>
          <button className="primary-action" type="button" onClick={run}>
            <Zap size={18} />
            Run NDVI
          </button>
          {summary && <span className="muted-result">{summary}</span>}
        </div>
      </div>
      {result && (
        <div className="card result-card">
          <h2>NDVI Result</h2>
          <div className="result-grid">
            <Metric label="Size" value={`${result.width}x${result.height}`} />
            <Metric label="Valid" value={String(result.validPixelCount)} />
            <Metric label="Nodata" value={String(result.nodataPixelCount)} />
            <Metric label="Mean" value={formatNumber(result.mean)} />
          </div>
          {result.warnings.map((warning) => (
            <p className="muted-result" key={warning}>
              {warning}
            </p>
          ))}
        </div>
      )}
    </section>
  );
}

function PvScreen(props: {
  dataset: PowerDataset | null;
  request: PowerRequest;
  capacity: number;
  losses: number;
  inverter: number;
  tilt: number;
  azimuth: number;
  estimate: PvEstimate | null;
  pvWattsResult: PvWattsResult | null;
  setCapacity: (value: number) => void;
  setLosses: (value: number) => void;
  setInverter: (value: number) => void;
  setTilt: (value: number) => void;
  setAzimuth: (value: number) => void;
  onEstimate: () => void;
  onPvWatts: () => void;
}) {
  const source = props.dataset?.request ?? props.request;
  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <BarChart3 size={34} />
        </div>
        <div>
          <h1>PV Power Estimate</h1>
          <p>Local quick estimate plus PVWatts/NLR mode when an API key is stored</p>
        </div>
      </section>
      <div className="power-grid">
        <div className="card form-card">
          <div className="field-grid two">
            <label>
              PV Capacity (kW)
              <input type="number" value={props.capacity} onChange={(event) => props.setCapacity(Number(event.target.value))} />
            </label>
            <label>
              Irradiance Column
              <input value="ALLSKY_SFC_SW_DWN" readOnly />
            </label>
            <label>
              System Losses (%)
              <input type="number" value={props.losses} onChange={(event) => props.setLosses(Number(event.target.value))} />
            </label>
            <label>
              Inverter Efficiency (%)
              <input type="number" value={props.inverter} onChange={(event) => props.setInverter(Number(event.target.value))} />
            </label>
            <label>
              Tilt (deg)
              <input type="number" value={props.tilt} onChange={(event) => props.setTilt(Number(event.target.value))} />
            </label>
            <label>
              Azimuth (deg)
              <input type="number" value={props.azimuth} onChange={(event) => props.setAzimuth(Number(event.target.value))} />
            </label>
          </div>
          <p className="muted-result">PVWatts location: {source.latitude.toFixed(4)}, {source.longitude.toFixed(4)}</p>
          <div className="action-row">
            <button className="primary-action" type="button" onClick={props.onEstimate}>
              <Zap size={18} />
              Estimate Local PV
            </button>
            <button className="secondary-action" type="button" onClick={props.onPvWatts}>
              <CloudDownload size={18} />
              PVWatts/NLR
            </button>
          </div>
        </div>
        <div className="card result-card">
          <h2>Estimate Result</h2>
          {props.estimate ? (
            <>
              <div className="result-grid">
                <Metric label="Energy" value={`${props.estimate.energyKwh.toFixed(2)} kWh`} />
                <Metric label="Avg Power" value={`${props.estimate.averagePowerKw.toFixed(2)} kW`} />
                <Metric label="Capacity Factor" value={`${props.estimate.capacityFactorPercent.toFixed(1)}%`} />
                <Metric label="Used/Missing" value={`${props.estimate.usedRecordCount}/${props.estimate.missingRecordCount}`} />
              </div>
              {props.estimate.assumptions.map((assumption) => (
                <p className="muted-result" key={assumption}>
                  {assumption}
                </p>
              ))}
            </>
          ) : (
            <p className="empty-text">{props.dataset ? "Run estimate to calculate output." : "Fetch NASA POWER data first, then estimate local PV output."}</p>
          )}
          {props.pvWattsResult && (
            <div className="result-grid pvwatts-grid">
              <Metric label="PVWatts AC Annual" value={`${props.pvWattsResult.acAnnualKwh.toFixed(0)} kWh`} />
              <Metric label="Solrad Annual" value={`${props.pvWattsResult.solradAnnualKwhPerM2Day.toFixed(2)}`} />
              <Metric label="PVWatts CF" value={`${props.pvWattsResult.capacityFactorPercent.toFixed(1)}%`} />
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

function SavedScreen({
  addLog,
  onPreview,
  onSavedCountChange,
}: {
  addLog: (message: string) => void;
  onPreview: (dataset: PowerDataset) => void;
  onSavedCountChange: (count: number) => void;
}) {
  const [items, setItems] = useState<SavedDataset[]>([]);
  const [lastPath, setLastPath] = useState("");

  async function refresh() {
    const saved = await listSavedDatasets();
    setItems(saved);
    onSavedCountChange(saved.length);
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function preview(id: string) {
    try {
      onPreview(await loadSavedDataset(id));
    } catch (err) {
      addLog(`Saved preview failed: ${errorMessage(err)}`);
    }
  }

  async function exportSaved(id: string, format: "csv" | "json") {
    try {
      const result = await exportSavedDataset(id, format);
      setLastPath(result.path);
      addLog(`Saved dataset exported: ${result.path}`);
    } catch (err) {
      addLog(`Saved export failed: ${errorMessage(err)}`);
    }
  }

  async function remove(id: string) {
    try {
      await deleteSavedDataset(id);
      await refresh();
      addLog("Saved dataset deleted");
    } catch (err) {
      addLog(`Delete failed: ${errorMessage(err)}`);
    }
  }

  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <Database size={34} />
        </div>
        <div>
          <h1>Saved Data</h1>
          <p>Review locally stored datasets and export history</p>
        </div>
        <button className="doc-button" type="button" onClick={refresh}>
          Refresh
        </button>
      </section>
      <div className="card table-card">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Records</th>
              <th>Created</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <tr key={item.id}>
                <td>{item.name}</td>
                <td>{item.recordCount}</td>
                <td>{item.createdAt}</td>
                <td>
                  <div className="row-actions">
                    <button className="secondary-action compact" type="button" onClick={() => preview(item.id)}>Preview</button>
                    <button className="secondary-action compact" type="button" onClick={() => exportSaved(item.id, "csv")}>CSV</button>
                    <button className="secondary-action compact" type="button" onClick={() => exportSaved(item.id, "json")}>JSON</button>
                    <button className="secondary-action compact danger" type="button" onClick={() => remove(item.id)}>Delete</button>
                  </div>
                </td>
              </tr>
            ))}
            {items.length === 0 && (
              <tr>
                <td colSpan={4} className="empty-cell">
                  No saved datasets yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
        {lastPath && <p className="muted-result">Last export: {lastPath}</p>}
      </div>
    </section>
  );
}

function ApiScreen({
  apiStatus,
  refreshApiStatus,
  addLog,
}: {
  apiStatus: Record<string, boolean>;
  refreshApiStatus: () => Promise<void>;
  addLog: (message: string) => void;
}) {
  const [values, setValues] = useState<Record<string, string>>({});
  const [tests, setTests] = useState<Record<string, CredentialTestResult>>({});

  async function storeKey(name: string) {
    try {
      await invoke<void>("store_api_key", { name, value: values[name] ?? "" });
      setValues((current) => ({ ...current, [name]: "" }));
      await refreshApiStatus();
      addLog(`API slot stored: ${name}`);
    } catch (err) {
      addLog(`API slot store failed: ${errorMessage(err)}`);
    }
  }

  async function deleteKey(name: string) {
    try {
      await invoke<void>("delete_api_key", { name });
      await refreshApiStatus();
      addLog(`API slot deleted: ${name}`);
    } catch (err) {
      addLog(`API slot delete failed: ${errorMessage(err)}`);
    }
  }

  async function testKey(name: string) {
    try {
      const result = await invoke<CredentialTestResult>("test_api_key", { name });
      setTests((current) => ({ ...current, [name]: result }));
      addLog(`API slot test ${result.ok ? "passed" : "failed"}: ${name}`);
    } catch (err) {
      addLog(`API slot test failed: ${errorMessage(err)}`);
    }
  }

  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <KeyRound size={34} />
        </div>
        <div>
          <h1>API Slots</h1>
          <p>Credential slots are stored through the operating-system keychain</p>
        </div>
        <button className="doc-button" type="button" onClick={refreshApiStatus}>
          Refresh
        </button>
      </section>
      <div className="overview-grid api-grid">
        {apiSlots.map((slot) => (
          <div className="card api-card" key={slot.name}>
            <KeyRound size={22} />
            <strong>{slot.label}</strong>
            <span className={apiStatus[slot.name] ? "status-badge success" : "status-badge muted"}>{apiStatus[slot.name] ? "Stored" : "Empty"}</span>
            <input
              type={slot.type}
              value={values[slot.name] ?? ""}
              placeholder="Paste key, then Store"
              onChange={(event) => setValues((current) => ({ ...current, [slot.name]: event.target.value }))}
            />
            <div className="row-actions">
              <button type="button" className="secondary-action compact" onClick={() => storeKey(slot.name)}>Store</button>
              <button type="button" className="secondary-action compact" onClick={() => testKey(slot.name)}>Test</button>
              <button type="button" className="secondary-action compact danger" onClick={() => deleteKey(slot.name)}>Delete</button>
            </div>
            {tests[slot.name] && <p className={tests[slot.name].ok ? "muted-result success-text" : "muted-result danger-text"}>{tests[slot.name].message}</p>}
          </div>
        ))}
      </div>
    </section>
  );
}

function SettingsScreen() {
  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <Settings size={34} />
        </div>
        <div>
          <h1>Settings</h1>
          <p>Defaults for units, theme, storage, and request handling</p>
        </div>
      </section>
      <div className="card form-card">
        <div className="field-grid two">
          <label>
            Theme
            <select defaultValue="dark">
              <option value="dark">Dark</option>
              <option value="system">System</option>
            </select>
          </label>
          <label>
            Request Timeout
            <input defaultValue="60 seconds" readOnly />
          </label>
        </div>
      </div>
    </section>
  );
}

function AboutScreen() {
  return (
    <section className="stack-screen">
      <section className="screen-heading">
        <div className="heading-icon">
          <Info size={34} />
        </div>
        <div>
          <h1>Satellite Data Toolkit</h1>
          <p>Tauri + React + Rust implementation for one-click scientific workflows</p>
        </div>
      </section>
      <div className="card about-card">
        <p>
          Current release status: macOS local build is supported; Windows installers, Apple notarization,
          EUMDAC bundling, and live PVWatts/NLR validation require external release credentials or platform runners.
        </p>
        <div className="source-list">
          <a href="https://power.larc.nasa.gov/docs/services/api/" target="_blank" rel="noreferrer">NASA POWER API</a>
          <a href="https://tauri.app/" target="_blank" rel="noreferrer">Tauri</a>
          <a href="https://developer.nlr.gov/docs/solar/pvwatts/" target="_blank" rel="noreferrer">PVWatts V8</a>
        </div>
      </div>
    </section>
  );
}

export default App;
