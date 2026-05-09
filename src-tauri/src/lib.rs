use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::Utc;
use keyring::Entry;
use rusqlite::{params, Connection, OptionalExtension};
use satellite_core::{
    estimate_pv as estimate_pv_core, estimate_pvwatts as estimate_pvwatts_core,
    fetch_power_dataset as fetch_power_dataset_core, run_ndvi as run_ndvi_core,
    validate_ndvi_inputs as validate_ndvi_inputs_core, NdviJob, NdviResult, PowerDataset,
    PowerRequest, PvEstimate, PvEstimateInput, PvWattsRequest, PvWattsResult,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const KEYCHAIN_SERVICE: &str = "Satellite Data Toolkit";
const MAX_DATASET_RECORDS: usize = 120_000;
const MAX_DATASET_JSON_BYTES: usize = 64 * 1024 * 1024;
const MAX_SAVED_NAME_LEN: usize = 160;
const EUMDAC_SIDECAR_NAMES: &[&str] = &["eumdac", "eumdac.exe", "eumdac-cli", "eumdac-cli.exe"];
const EUMDAC_MANIFEST_NAMES: &[&str] = &["eumdac-sidecar-manifest.json", "eumdac-sidecars.json"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedDataset {
    id: String,
    name: String,
    kind: String,
    created_at: String,
    record_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    path: String,
    format: String,
    bytes: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialTestResult {
    slot: String,
    ok: bool,
    message: String,
}

struct EumetsatCredentials {
    consumer_key: String,
    consumer_secret: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EumetsatQuery {
    collection_id: String,
    bbox: String,
    start_time: String,
    end_time: String,
    #[serde(default = "default_eumetsat_limit")]
    limit: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EumetsatProduct {
    id: String,
    title: String,
    raw: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductList {
    products: Vec<EumetsatProduct>,
    raw_output: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadResult {
    collection_id: String,
    product_id: String,
    output_dir: String,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EumdacSidecarStatus {
    found: bool,
    trusted: bool,
    path: Option<String>,
    file_name: Option<String>,
    sha256: Option<String>,
    manifest_path: Option<String>,
    version: Option<String>,
    source: Option<String>,
    license: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EumdacSidecarManifest {
    #[serde(default)]
    binaries: Vec<EumdacSidecarManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EumdacSidecarManifestEntry {
    #[serde(alias = "fileName")]
    name: String,
    sha256: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    license: Option<String>,
}

#[tauri::command]
async fn fetch_power_dataset(request: PowerRequest) -> Result<PowerDataset, String> {
    fetch_power_dataset_core(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn estimate_pv(input: PvEstimateInput) -> Result<PvEstimate, String> {
    estimate_pv_core(input).map_err(|error| error.to_string())
}

#[tauri::command]
async fn estimate_pvwatts(request: PvWattsRequest) -> Result<PvWattsResult, String> {
    let api_key = get_api_key("nlr_pvwatts_key")?
        .ok_or_else(|| "PVWatts/NLR API key is not stored".to_string())?;
    estimate_pvwatts_core(request, &api_key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn validate_ndvi_inputs(job: NdviJob) -> Result<String, String> {
    validate_ndvi_inputs_core(&job).map_err(|error| error.to_string())
}

#[tauri::command]
fn run_ndvi(job: NdviJob) -> Result<NdviResult, String> {
    run_ndvi_core(&job).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_dataset(
    app: AppHandle,
    name: String,
    dataset: PowerDataset,
) -> Result<SavedDataset, String> {
    validate_dataset_for_storage(&dataset)?;
    let clean_name = validate_saved_name(&name)?;
    let connection = open_db(&app)?;
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let payload = serde_json::to_string(&dataset).map_err(|error| error.to_string())?;
    if payload.len() > MAX_DATASET_JSON_BYTES {
        return Err("dataset payload is too large to store".to_string());
    }
    connection
        .execute(
            "insert into saved_datasets (id, name, kind, created_at, record_count, payload) values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, clean_name, "nasa_power", created_at, dataset.records.len() as i64, payload],
        )
        .map_err(|error| error.to_string())?;
    Ok(SavedDataset {
        id,
        name: clean_name,
        kind: "nasa_power".to_string(),
        created_at,
        record_count: dataset.records.len(),
    })
}

#[tauri::command]
fn list_saved_datasets(app: AppHandle) -> Result<Vec<SavedDataset>, String> {
    let connection = open_db(&app)?;
    let mut statement = connection
        .prepare(
            "select id, name, kind, created_at, record_count from saved_datasets order by created_at desc",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(SavedDataset {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                created_at: row.get(3)?,
                record_count: row.get::<_, i64>(4)? as usize,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn load_saved_dataset(app: AppHandle, id: String) -> Result<PowerDataset, String> {
    let connection = open_db(&app)?;
    let payload = connection
        .query_row(
            "select payload from saved_datasets where id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "saved dataset not found".to_string())?;
    serde_json::from_str(&payload).map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_saved_dataset(app: AppHandle, id: String) -> Result<(), String> {
    let connection = open_db(&app)?;
    let changed = connection
        .execute("delete from saved_datasets where id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("saved dataset not found".to_string());
    }
    Ok(())
}

#[tauri::command]
fn export_dataset(
    app: AppHandle,
    dataset: PowerDataset,
    format: String,
) -> Result<ExportResult, String> {
    validate_dataset_for_storage(&dataset)?;
    write_dataset_export(&app, &dataset, &format, None, None)
}

#[tauri::command]
fn export_saved_dataset(
    app: AppHandle,
    id: String,
    format: String,
    destination: Option<String>,
) -> Result<ExportResult, String> {
    let dataset = load_saved_dataset(app.clone(), id.clone())?;
    write_dataset_export(&app, &dataset, &format, Some(&id), destination.as_deref())
}

#[tauri::command]
fn store_api_key(name: String, value: String) -> Result<(), String> {
    validate_secret_name(&name)?;
    let clean_value = value.trim();
    if clean_value.is_empty() || clean_value == "replace-with-real-key" {
        return Err("API key value is empty or still a placeholder".to_string());
    }
    Entry::new(KEYCHAIN_SERVICE, &name)
        .map_err(|error| error.to_string())?
        .set_password(clean_value)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn has_api_key(name: String) -> Result<bool, String> {
    validate_secret_name(&name)?;
    get_api_key(&name).map(|value| value.is_some_and(|value| !value.is_empty()))
}

#[tauri::command]
fn delete_api_key(name: String) -> Result<(), String> {
    validate_secret_name(&name)?;
    match Entry::new(KEYCHAIN_SERVICE, &name)
        .map_err(|error| error.to_string())?
        .delete_credential()
    {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
async fn test_api_key(name: String) -> Result<CredentialTestResult, String> {
    validate_secret_name(&name)?;

    if name == "nlr_pvwatts_key" {
        let present = get_api_key(&name)?.is_some_and(|value| !value.is_empty());
        if !present {
            return Ok(CredentialTestResult {
                slot: name,
                ok: false,
                message: "No credential stored for this slot".to_string(),
            });
        }
        let request = PvWattsRequest {
            latitude: 40.7128,
            longitude: -74.006,
            system_capacity_kw: 1.0,
            tilt_degrees: 30.0,
            azimuth_degrees: 180.0,
            losses_percent: 14.0,
            module_type: 0,
            array_type: 1,
            timeframe: "monthly".to_string(),
        };
        match estimate_pvwatts(request).await {
            Ok(_) => {
                return Ok(CredentialTestResult {
                    slot: name,
                    ok: true,
                    message: "PVWatts/NLR key accepted by the API".to_string(),
                })
            }
            Err(error) => {
                return Ok(CredentialTestResult {
                    slot: name,
                    ok: false,
                    message: error,
                })
            }
        }
    }

    let key_present = get_api_key("eumetsat_consumer_key")?.is_some_and(|value| !value.is_empty());
    let secret_present =
        get_api_key("eumetsat_consumer_secret")?.is_some_and(|value| !value.is_empty());
    let sidecar_status = eumdac_sidecar_status()?;
    Ok(evaluate_eumetsat_credential_status(
        name,
        key_present,
        secret_present,
        &sidecar_status,
        allow_unverified_eumdac_sidecar(),
    ))
}

#[tauri::command]
fn check_eumdac_sidecar() -> Result<bool, String> {
    let status = eumdac_sidecar_status()?;
    Ok(status.trusted || (status.found && allow_unverified_eumdac_sidecar()))
}

#[tauri::command]
fn get_eumdac_sidecar_status() -> Result<EumdacSidecarStatus, String> {
    eumdac_sidecar_status()
}

#[tauri::command]
fn fetch_eumetsat_products(app: AppHandle, query: EumetsatQuery) -> Result<ProductList, String> {
    validate_eumetsat_query(&query)?;
    let sidecar = trusted_eumdac_sidecar()?;
    sync_eumdac_credentials(&app, &sidecar)?;
    let limit = query.limit.clamp(1, 100).to_string();
    let bbox = parse_eumdac_bbox(&query.bbox)?;
    let mut command = Command::new(sidecar);
    configure_eumdac_command(&app, &mut command)?;
    command.args([
        "search",
        "-c",
        query.collection_id.trim(),
        "-s",
        query.start_time.trim(),
        "-e",
        query.end_time.trim(),
        "--bbox",
    ]);
    for coordinate in &bbox {
        command.arg(coordinate);
    }
    let output = command
        .args(["--limit", &limit])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(redacted_process_error(&output.stderr));
    }
    let raw_output = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(ProductList {
        products: parse_eumdac_products(&raw_output),
        raw_output,
    })
}

#[tauri::command]
fn download_eumetsat_product(
    app: AppHandle,
    collection_id: String,
    product_id: String,
    output_dir: String,
) -> Result<DownloadResult, String> {
    let (clean_collection_id, clean_product_id, output_path) =
        validate_eumetsat_download_request(&collection_id, &product_id, &output_dir)?;
    let sidecar = trusted_eumdac_sidecar()?;
    sync_eumdac_credentials(&app, &sidecar)?;
    let mut command = Command::new(sidecar);
    configure_eumdac_command(&app, &mut command)?;
    let output = command
        .args([
            "download",
            "-c",
            &clean_collection_id,
            "-p",
            &clean_product_id,
            "-o",
        ])
        .arg(&output_path)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(redacted_process_error(&output.stderr));
    }
    Ok(DownloadResult {
        collection_id: clean_collection_id,
        product_id: clean_product_id,
        output_dir: output_path.to_string_lossy().to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn open_db(app: &AppHandle) -> Result<Connection, String> {
    let path = app_data_dir(app)?.join("toolkit.sqlite");
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(
            "create table if not exists saved_datasets (
                id text primary key,
                name text not null,
                kind text not null,
                created_at text not null,
                record_count integer not null,
                payload text not null
            );
            create table if not exists export_history (
                id text primary key,
                dataset_id text,
                format text not null,
                path text not null,
                created_at text not null,
                bytes integer not null
            );",
        )
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn write_dataset_export(
    app: &AppHandle,
    dataset: &PowerDataset,
    format: &str,
    dataset_id: Option<&str>,
    destination: Option<&str>,
) -> Result<ExportResult, String> {
    let extension = match format {
        "csv" => "csv",
        "json" => "json",
        _ => return Err("export format must be csv or json".to_string()),
    };
    let default_name = format!(
        "nasa_power_{}_{}.{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        Uuid::new_v4().simple(),
        extension
    );
    let path = resolve_export_path(app, destination, &default_name)?;
    let body = if extension == "json" {
        serde_json::to_string_pretty(dataset).map_err(|error| error.to_string())?
    } else {
        dataset_to_csv(dataset)
    };
    if body.len() > MAX_DATASET_JSON_BYTES {
        return Err("export payload is too large".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, &body).map_err(|error| error.to_string())?;

    let result = ExportResult {
        path: path.to_string_lossy().to_string(),
        format: extension.to_string(),
        bytes: body.len(),
    };
    let connection = open_db(app)?;
    connection
        .execute(
            "insert into export_history (id, dataset_id, format, path, created_at, bytes) values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                Uuid::new_v4().to_string(),
                dataset_id,
                result.format,
                result.path,
                Utc::now().to_rfc3339(),
                result.bytes as i64
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(result)
}

fn resolve_export_path(
    app: &AppHandle,
    destination: Option<&str>,
    default_name: &str,
) -> Result<PathBuf, String> {
    if let Some(destination) = destination.map(str::trim).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(destination);
        if path.is_dir() {
            return Ok(path.join(default_name));
        }
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err("export destination parent directory does not exist".to_string());
            }
        }
        return Ok(path);
    }
    let exports_dir = app_data_dir(app)?.join("exports");
    fs::create_dir_all(&exports_dir).map_err(|error| error.to_string())?;
    Ok(exports_dir.join(default_name))
}

fn dataset_to_csv(dataset: &PowerDataset) -> String {
    let mut lines = Vec::with_capacity(dataset.records.len() + 3);
    lines.push(format!("# fetched_at,{}", csv_escape(&dataset.fetched_at)));
    lines.push(format!(
        "# time_standard,{}",
        csv_escape(&dataset.time_standard)
    ));
    let mut header = vec!["timestamp".to_string(), "raw_timestamp".to_string()];
    header.extend(dataset.request.parameters.iter().cloned());
    lines.push(
        header
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );

    for record in &dataset.records {
        let mut row = vec![record.timestamp.clone(), record.raw_timestamp.clone()];
        row.extend(dataset.request.parameters.iter().map(|parameter| {
            record
                .values
                .get(parameter)
                .and_then(|value| *value)
                .map(|value| value.to_string())
                .unwrap_or_default()
        }));
        lines.push(
            row.iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    lines.join("\n")
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn validate_dataset_for_storage(dataset: &PowerDataset) -> Result<(), String> {
    if dataset.records.len() > MAX_DATASET_RECORDS {
        return Err(format!(
            "dataset has too many records; maximum is {MAX_DATASET_RECORDS}"
        ));
    }
    Ok(())
}

fn validate_saved_name(name: &str) -> Result<String, String> {
    let clean = name.trim();
    if clean.is_empty() {
        return Err("dataset name is required".to_string());
    }
    if clean.len() > MAX_SAVED_NAME_LEN {
        return Err(format!(
            "dataset name is too long; maximum is {MAX_SAVED_NAME_LEN} bytes"
        ));
    }
    Ok(clean.to_string())
}

fn validate_secret_name(name: &str) -> Result<(), String> {
    match name {
        "eumetsat_consumer_key" | "eumetsat_consumer_secret" | "nlr_pvwatts_key" => Ok(()),
        _ => Err("unknown API slot".to_string()),
    }
}

fn evaluate_eumetsat_credential_status(
    slot: String,
    key_present: bool,
    secret_present: bool,
    sidecar_status: &EumdacSidecarStatus,
    allow_unverified_sidecar: bool,
) -> CredentialTestResult {
    if !key_present || !secret_present {
        return CredentialTestResult {
            slot,
            ok: false,
            message: "Both EUMETSAT consumer key and consumer secret must be stored".to_string(),
        };
    }

    if sidecar_status.trusted || (sidecar_status.found && allow_unverified_sidecar) {
        CredentialTestResult {
            slot,
            ok: true,
            message: "EUMETSAT credentials are stored and the EUMDAC sidecar is ready".to_string(),
        }
    } else {
        CredentialTestResult {
            slot,
            ok: false,
            message: sidecar_status.message.clone(),
        }
    }
}

fn get_api_key(name: &str) -> Result<Option<String>, String> {
    validate_secret_name(name)?;
    let entry = Entry::new(KEYCHAIN_SERVICE, name).map_err(|error| error.to_string())?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn get_eumetsat_credentials() -> Result<EumetsatCredentials, String> {
    let consumer_key = get_api_key("eumetsat_consumer_key")?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "EUMETSAT consumer key and secret must both be stored".to_string())?;
    let consumer_secret = get_api_key("eumetsat_consumer_secret")?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "EUMETSAT consumer key and secret must both be stored".to_string())?;
    Ok(EumetsatCredentials {
        consumer_key,
        consumer_secret,
    })
}

fn sync_eumdac_credentials(app: &AppHandle, sidecar: &Path) -> Result<(), String> {
    let credentials = get_eumetsat_credentials()?;
    let mut command = Command::new(sidecar);
    configure_eumdac_command(app, &mut command)?;
    let output = command
        .args([
            "set-credentials",
            credentials.consumer_key.as_str(),
            credentials.consumer_secret.as_str(),
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(());
    }

    let mut fallback = Command::new(sidecar);
    configure_eumdac_command(app, &mut fallback)?;
    let fallback_output = fallback
        .args([
            "--set-credentials",
            credentials.consumer_key.as_str(),
            credentials.consumer_secret.as_str(),
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if fallback_output.status.success() {
        Ok(())
    } else {
        Err(redacted_process_error_with_secrets(
            &fallback_output.stderr,
            &[
                credentials.consumer_key.as_str(),
                credentials.consumer_secret.as_str(),
            ],
        ))
    }
}

fn configure_eumdac_command(app: &AppHandle, command: &mut Command) -> Result<(), String> {
    let config_dir = app_data_dir(app)?.join("eumdac");
    fs::create_dir_all(&config_dir).map_err(|error| error.to_string())?;
    command
        .env("EUMDAC_CONFIG_DIR", &config_dir)
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("APPDATA", &config_dir);
    Ok(())
}

fn find_eumdac_sidecar() -> Result<Option<PathBuf>, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let Some(dir) = exe.parent() else {
        return Ok(None);
    };
    Ok(EUMDAC_SIDECAR_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| is_executable_candidate(path)))
}

fn is_executable_candidate(path: &Path) -> bool {
    path.exists() && path.is_file()
}

fn trusted_eumdac_sidecar() -> Result<PathBuf, String> {
    let status = eumdac_sidecar_status()?;
    let Some(path) = status.path.as_deref().map(PathBuf::from) else {
        return Err(status.message);
    };
    if status.trusted || allow_unverified_eumdac_sidecar() {
        Ok(path)
    } else {
        Err(status.message)
    }
}

fn allow_unverified_eumdac_sidecar() -> bool {
    std::env::var("SATELLITE_ALLOW_UNVERIFIED_EUMDAC")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn eumdac_sidecar_status() -> Result<EumdacSidecarStatus, String> {
    match find_eumdac_sidecar()? {
        Some(path) => eumdac_sidecar_status_for_path(path),
        None => Ok(EumdacSidecarStatus {
            found: false,
            trusted: false,
            path: None,
            file_name: None,
            sha256: None,
            manifest_path: None,
            version: None,
            source: None,
            license: None,
            message: "EUMDAC sidecar is not bundled".to_string(),
        }),
    }
}

fn eumdac_sidecar_status_for_path(path: PathBuf) -> Result<EumdacSidecarStatus, String> {
    let sha256 = sha256_file(&path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("eumdac")
        .to_string();
    let manifest_path = path.parent().and_then(|dir| {
        EUMDAC_MANIFEST_NAMES
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.exists() && candidate.is_file())
    });
    let Some(manifest_path) = manifest_path else {
        return Ok(EumdacSidecarStatus {
            found: true,
            trusted: false,
            path: Some(path.to_string_lossy().to_string()),
            file_name: Some(file_name),
            sha256: Some(sha256),
            manifest_path: None,
            version: None,
            source: None,
            license: None,
            message: "EUMDAC sidecar is present but checksum manifest is missing".to_string(),
        });
    };
    let manifest_path_string = manifest_path.to_string_lossy().to_string();
    let manifest = match read_eumdac_sidecar_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Ok(EumdacSidecarStatus {
                found: true,
                trusted: false,
                path: Some(path.to_string_lossy().to_string()),
                file_name: Some(file_name),
                sha256: Some(sha256),
                manifest_path: Some(manifest_path_string),
                version: None,
                source: None,
                license: None,
                message: error,
            })
        }
    };
    let Some(entry) = manifest
        .binaries
        .iter()
        .find(|entry| entry.name == file_name)
    else {
        return Ok(EumdacSidecarStatus {
            found: true,
            trusted: false,
            path: Some(path.to_string_lossy().to_string()),
            file_name: Some(file_name.clone()),
            sha256: Some(sha256),
            manifest_path: Some(manifest_path_string),
            version: None,
            source: None,
            license: None,
            message: format!("EUMDAC sidecar manifest has no entry for {file_name}"),
        });
    };
    let expected_sha256 = normalized_sha256(&entry.sha256);
    let trusted = expected_sha256 == sha256;
    Ok(EumdacSidecarStatus {
        found: true,
        trusted,
        path: Some(path.to_string_lossy().to_string()),
        file_name: Some(file_name),
        sha256: Some(sha256),
        manifest_path: Some(manifest_path_string),
        version: entry.version.clone(),
        source: entry.source.clone(),
        license: entry.license.clone(),
        message: if trusted {
            "EUMDAC sidecar checksum matches manifest".to_string()
        } else {
            "EUMDAC sidecar checksum does not match manifest".to_string()
        },
    })
}

fn read_eumdac_sidecar_manifest(path: &Path) -> Result<EumdacSidecarManifest, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| format!("invalid EUMDAC manifest: {error}"))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn normalized_sha256(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_eumetsat_query(query: &EumetsatQuery) -> Result<(), String> {
    if query.collection_id.trim().is_empty() {
        return Err("collection id is required".to_string());
    }
    parse_eumdac_bbox(&query.bbox)?;
    if query.start_time.trim().is_empty() || query.end_time.trim().is_empty() {
        return Err("start and end time are required".to_string());
    }
    Ok(())
}

fn validate_eumetsat_download_request(
    collection_id: &str,
    product_id: &str,
    output_dir: &str,
) -> Result<(String, String, PathBuf), String> {
    let clean_collection_id = collection_id.trim().to_string();
    let clean_product_id = product_id.trim().to_string();
    if clean_collection_id.is_empty() {
        return Err("collection id is required".to_string());
    }
    if clean_product_id.is_empty() {
        return Err("product id is required".to_string());
    }
    let clean_output_dir = output_dir.trim();
    if clean_output_dir.is_empty() {
        return Err("output directory is required".to_string());
    }
    let output_path = PathBuf::from(clean_output_dir);
    if !output_path.exists() || !output_path.is_dir() {
        return Err("output directory must exist".to_string());
    }
    Ok((clean_collection_id, clean_product_id, output_path))
}

fn parse_eumdac_bbox(value: &str) -> Result<[String; 4], String> {
    let parts = value
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("bbox must contain four comma- or space-separated numbers".to_string());
    }
    for part in &parts {
        let number = part
            .parse::<f64>()
            .map_err(|_| "bbox must contain only finite numbers".to_string())?;
        if !number.is_finite() {
            return Err("bbox must contain only finite numbers".to_string());
        }
    }
    Ok([
        parts[0].clone(),
        parts[1].clone(),
        parts[2].clone(),
        parts[3].clone(),
    ])
}

fn parse_eumdac_products(raw_output: &str) -> Vec<EumetsatProduct> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw_output) else {
        return raw_output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| EumetsatProduct {
                id: line.trim().to_string(),
                title: line.trim().to_string(),
                raw: serde_json::Value::String(line.trim().to_string()),
            })
            .collect();
    };
    json_products(value)
}

fn json_products(value: serde_json::Value) -> Vec<EumetsatProduct> {
    let candidates = if let Some(items) = value.as_array() {
        items.clone()
    } else if let Some(features) = value
        .get("features")
        .and_then(|features| features.as_array())
    {
        features.clone()
    } else {
        vec![value]
    };

    candidates
        .into_iter()
        .map(|raw| {
            let id = raw
                .get("id")
                .or_else(|| raw.get("identifier"))
                .or_else(|| raw.pointer("/properties/id"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string();
            let title = raw
                .get("title")
                .or_else(|| raw.get("name"))
                .or_else(|| raw.pointer("/properties/title"))
                .and_then(|value| value.as_str())
                .unwrap_or(&id)
                .to_string();
            EumetsatProduct { id, title, raw }
        })
        .collect()
}

fn redacted_process_error(stderr: &[u8]) -> String {
    redacted_process_error_with_secrets(stderr, &[])
}

fn redacted_process_error_with_secrets(stderr: &[u8], secrets: &[&str]) -> String {
    let message = String::from_utf8_lossy(stderr);
    if message.trim().is_empty() {
        "EUMDAC command failed".to_string()
    } else {
        let mut redacted = message
            .replace("consumer_secret", "consumer_secret[redacted]")
            .replace("consumer_key", "consumer_key[redacted]");
        for secret in secrets
            .iter()
            .map(|secret| secret.trim())
            .filter(|secret| !secret.is_empty())
        {
            redacted = redacted.replace(secret, "[redacted]");
        }
        redacted
    }
}

fn default_eumetsat_limit() -> usize {
    20
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            fetch_power_dataset,
            estimate_pv,
            estimate_pvwatts,
            validate_ndvi_inputs,
            run_ndvi,
            save_dataset,
            list_saved_datasets,
            load_saved_dataset,
            delete_saved_dataset,
            export_dataset,
            export_saved_dataset,
            store_api_key,
            has_api_key,
            delete_api_key,
            test_api_key,
            check_eumdac_sidecar,
            get_eumdac_sidecar_status,
            fetch_eumetsat_products,
            download_eumetsat_product
        ])
        .run(tauri::generate_context!())
        .expect("error while running Satellite Data Toolkit");
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use satellite_core::{PowerRecord, PowerRequest};

    use super::*;

    #[test]
    fn csv_escapes_values_and_preserves_raw_timestamp() {
        let dataset = PowerDataset {
            request: PowerRequest {
                latitude: 0.0,
                longitude: 0.0,
                start_date: "2024-05-01".to_string(),
                end_date: "2024-05-01".to_string(),
                parameters: vec!["A,B".to_string()],
                temporal: "daily".to_string(),
                community: "RE".to_string(),
                time_standard: "LST".to_string(),
            },
            records: vec![PowerRecord {
                raw_timestamp: "20240501".to_string(),
                timestamp: "2024-05-01".to_string(),
                values: BTreeMap::from([("A,B".to_string(), Some(1.25))]),
            }],
            units: BTreeMap::new(),
            long_names: BTreeMap::new(),
            status_code: 200,
            api_version: "test".to_string(),
            time_standard: "LST".to_string(),
            fill_value: -999.0,
            data_time_seconds: 0.0,
            process_time_seconds: 0.0,
            fetched_at: "now".to_string(),
        };
        let csv = dataset_to_csv(&dataset);
        assert!(csv.contains("timestamp,raw_timestamp,\"A,B\""));
        assert!(csv.contains("2024-05-01,20240501,1.25"));
    }

    #[test]
    fn parses_comma_or_space_separated_eumdac_bbox() {
        assert_eq!(
            parse_eumdac_bbox("51.28,51.69,0.51,0.33").unwrap(),
            ["51.28", "51.69", "0.51", "0.33"]
        );
        assert_eq!(
            parse_eumdac_bbox("51.28 51.69 0.51 0.33").unwrap(),
            ["51.28", "51.69", "0.51", "0.33"]
        );
        assert!(parse_eumdac_bbox("51.28,invalid,0.51,0.33").is_err());
    }

    #[test]
    fn validates_eumdac_download_request_before_sidecar_work() {
        assert!(validate_eumetsat_download_request(" ", "PRODUCT_A", "/tmp")
            .unwrap_err()
            .contains("collection id"));
        assert!(
            validate_eumetsat_download_request("COLLECTION", " ", "/tmp")
                .unwrap_err()
                .contains("product id")
        );
        assert!(
            validate_eumetsat_download_request("COLLECTION", "PRODUCT_A", " ")
                .unwrap_err()
                .contains("output directory is required")
        );
        assert!(validate_eumetsat_download_request(
            "COLLECTION",
            "PRODUCT_A",
            "/definitely/not/a/satellite/toolkit/path"
        )
        .unwrap_err()
        .contains("must exist"));

        let dir = temp_dir_path("eumdac_download_validation");
        fs::create_dir_all(&dir).unwrap();
        let padded_dir = format!(" {} ", dir.to_string_lossy());

        let (collection, product, output_dir) =
            validate_eumetsat_download_request(" COLLECTION ", " PRODUCT_A ", &padded_dir).unwrap();

        assert_eq!(collection, "COLLECTION");
        assert_eq!(product, "PRODUCT_A");
        assert_eq!(output_dir, dir);

        let _ = fs::remove_dir_all(output_dir);
    }

    #[test]
    fn parses_plain_text_eumdac_products() {
        let products = parse_eumdac_products("PRODUCT_A\n\nPRODUCT_B\n");
        assert_eq!(products.len(), 2);
        assert_eq!(products[0].id, "PRODUCT_A");
        assert_eq!(products[1].title, "PRODUCT_B");
    }

    #[test]
    fn redacts_eumdac_secret_values_from_process_errors() {
        let message = redacted_process_error_with_secrets(
            b"consumer_key abc123 failed with consumer_secret def456",
            &["abc123", "def456"],
        );
        assert!(!message.contains("abc123"));
        assert!(!message.contains("def456"));
        assert!(message.contains("[redacted]"));
    }

    #[test]
    fn eumetsat_credential_test_requires_both_slots() {
        let status = ready_eumdac_status();

        let result = evaluate_eumetsat_credential_status(
            "eumetsat_consumer_key".to_string(),
            true,
            false,
            &status,
            false,
        );

        assert!(!result.ok);
        assert!(result.message.contains("Both EUMETSAT"));
    }

    #[test]
    fn eumetsat_credential_test_requires_ready_sidecar() {
        let status = missing_eumdac_status();

        let result = evaluate_eumetsat_credential_status(
            "eumetsat_consumer_secret".to_string(),
            true,
            true,
            &status,
            false,
        );

        assert!(!result.ok);
        assert_eq!(result.message, "EUMDAC sidecar is not bundled");
    }

    #[test]
    fn eumetsat_credential_test_passes_with_credentials_and_sidecar() {
        let status = ready_eumdac_status();

        let result = evaluate_eumetsat_credential_status(
            "eumetsat_consumer_secret".to_string(),
            true,
            true,
            &status,
            false,
        );

        assert!(result.ok);
        assert!(result.message.contains("sidecar is ready"));
    }

    fn ready_eumdac_status() -> EumdacSidecarStatus {
        EumdacSidecarStatus {
            found: true,
            trusted: true,
            path: Some("/tmp/eumdac".to_string()),
            file_name: Some("eumdac".to_string()),
            sha256: Some("abc".to_string()),
            manifest_path: Some("/tmp/eumdac-sidecar-manifest.json".to_string()),
            version: Some("3.0.0".to_string()),
            source: None,
            license: None,
            message: "ready".to_string(),
        }
    }

    fn missing_eumdac_status() -> EumdacSidecarStatus {
        EumdacSidecarStatus {
            found: false,
            trusted: false,
            path: None,
            file_name: None,
            sha256: None,
            manifest_path: None,
            version: None,
            source: None,
            license: None,
            message: "EUMDAC sidecar is not bundled".to_string(),
        }
    }

    #[test]
    fn trusts_eumdac_sidecar_when_manifest_checksum_matches() {
        let dir = temp_dir_path("trusted_eumdac");
        fs::create_dir_all(&dir).unwrap();
        let sidecar = dir.join("eumdac");
        fs::write(&sidecar, b"fake eumdac binary").unwrap();
        let sha256 = sha256_file(&sidecar).unwrap();
        fs::write(
            dir.join("eumdac-sidecar-manifest.json"),
            format!(
                r#"{{
                  "binaries": [{{
                    "name": "eumdac",
                    "sha256": "{sha256}",
                    "version": "3.0.0",
                    "source": "https://example.invalid/eumdac",
                    "license": "Apache-2.0"
                  }}]
                }}"#
            ),
        )
        .unwrap();

        let status = eumdac_sidecar_status_for_path(sidecar).unwrap();
        assert!(status.found);
        assert!(status.trusted);
        assert_eq!(status.version.as_deref(), Some("3.0.0"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_eumdac_sidecar_when_manifest_checksum_mismatches() {
        let dir = temp_dir_path("untrusted_eumdac");
        fs::create_dir_all(&dir).unwrap();
        let sidecar = dir.join("eumdac");
        fs::write(&sidecar, b"fake eumdac binary").unwrap();
        fs::write(
            dir.join("eumdac-sidecar-manifest.json"),
            r#"{"binaries":[{"name":"eumdac","sha256":"0000"}]}"#,
        )
        .unwrap();

        let status = eumdac_sidecar_status_for_path(sidecar).unwrap();
        assert!(status.found);
        assert!(!status.trusted);
        assert!(status.message.contains("does not match"));

        let _ = fs::remove_dir_all(dir);
    }

    fn temp_dir_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "satellite_data_toolkit_{name}_{}_{}",
            std::process::id(),
            nanos
        ))
    }
}
