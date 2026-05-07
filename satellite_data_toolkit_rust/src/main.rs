#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use chrono::{Local, Utc};
use eframe::{egui, egui::RichText};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const APP_NAME: &str = "Satellite Data Toolkit Pro";
const VERSION: &str = "v3.0.0";
const NASA_POWER_BASE: &str = "https://power.larc.nasa.gov/api/temporal";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
enum ApiAuthMode {
    PublicNoAuth,
    ApiKey,
    KeySecret,
    OAuth,
    CustomProxy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ApiSlot {
    name: String,
    provider: String,
    auth_mode: ApiAuthMode,
    status: String,
    key_name: String,
    secret_name: Option<String>,
    endpoint: String,
    notes: String,
    rate_limit_hint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RequestLog {
    ts: String,
    source: String,
    status: String,
    method: String,
    url: String,
    duration_ms: u128,
    records: usize,
    cache: String,
    provenance_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NasaRecord {
    date: String,
    allsky_sfc_sw_dwn: Option<f64>,
    t2m: Option<f64>,
    ws2m: Option<f64>,
    raw: serde_json::Value,
}

#[derive(Debug, PartialEq)]
enum Page {
    Dashboard,
    NasaPower,
    ApiSlots,
    SavedData,
    Settings,
    About,
}

#[derive(Debug, Clone)]
struct NasaForm {
    lat: String,
    lon: String,
    start: String,
    end: String,
    parameters: Vec<(String, bool, String)>,
    community: String,
    time_format: String,
    endpoint_mode: String,
    custom_endpoint: String,
}

impl Default for NasaForm {
    fn default() -> Self {
        Self {
            lat: "".into(),
            lon: "".into(),
            start: "".into(),
            end: "".into(),
            community: "RE".into(),
            time_format: "DAILY".into(),
            endpoint_mode: "Public NASA POWER endpoint".into(),
            custom_endpoint: NASA_POWER_BASE.into(),
            parameters: vec![
                ("ALLSKY_SFC_SW_DWN".into(), true, "Solar irradiance, kWh/m²/day".into()),
                ("T2M".into(), true, "Temperature at 2m, °C".into()),
                ("WS2M".into(), true, "Wind speed at 2m, m/s".into()),
                ("RH2M".into(), false, "Relative humidity at 2m, %".into()),
                ("PRECTOTCORR".into(), false, "Corrected precipitation, mm/day".into()),
                ("PS".into(), false, "Surface pressure, kPa".into()),
            ],
        }
    }
}

struct SatelliteApp {
    page: Page,
    nasa_form: NasaForm,
    api_slots: Vec<ApiSlot>,
    logs: Vec<RequestLog>,
    nasa_records: Vec<NasaRecord>,
    last_error: Option<String>,
    last_success: Option<String>,
    cache_enabled: bool,
    show_request_inspector: bool,
    last_request_url: String,
    last_response_preview: String,
    rate_counter: Vec<Instant>,
    app_dir: PathBuf,
}

impl Default for SatelliteApp {
    fn default() -> Self {
        let app_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("satellite_data_toolkit_pro");
        let _ = fs::create_dir_all(app_dir.join("exports"));
        let _ = fs::create_dir_all(app_dir.join("cache"));

        let api_slots = vec![
            ApiSlot {
                name: "NASA POWER".into(),
                provider: "NASA".into(),
                auth_mode: ApiAuthMode::PublicNoAuth,
                status: "Public API — no key required".into(),
                key_name: "not_required".into(),
                secret_name: None,
                endpoint: NASA_POWER_BASE.into(),
                notes: "NASA POWER works through an open public endpoint. This slot exists only for status, provenance and request inspection.".into(),
                rate_limit_hint: "Use cache and avoid aggressive loops.".into(),
            },
            ApiSlot {
                name: "EUMETSAT Data Store".into(),
                provider: "EUMETSAT".into(),
                auth_mode: ApiAuthMode::KeySecret,
                status: "Requires consumer key and consumer secret".into(),
                key_name: "eumetsat_consumer_key".into(),
                secret_name: Some("eumetsat_consumer_secret".into()),
                endpoint: "https://api.eumetsat.int".into(),
                notes: "Protected satellite products. Store credentials in OS keychain.".into(),
                rate_limit_hint: "Respect EUMETSAT product download quotas.".into(),
            },
            ApiSlot {
                name: "NREL PVWatts".into(),
                provider: "NREL".into(),
                auth_mode: ApiAuthMode::ApiKey,
                status: "Optional key for PV production calculations".into(),
                key_name: "nrel_pvwatts_api_key".into(),
                secret_name: None,
                endpoint: "https://developer.nrel.gov/api/pvwatts/v8.json".into(),
                notes: "Optional external production estimate. Local PV estimate still works without this.".into(),
                rate_limit_hint: "NREL has API limits; use cache.".into(),
            },
            ApiSlot {
                name: "Custom Weather Proxy".into(),
                provider: "Internal/Proxy".into(),
                auth_mode: ApiAuthMode::CustomProxy,
                status: "Optional".into(),
                key_name: "custom_proxy_token".into(),
                secret_name: None,
                endpoint: "http://localhost:8080".into(),
                notes: "Use this when routing requests through your own backend.".into(),
                rate_limit_hint: "Controlled by your backend.".into(),
            },
        ];

        Self {
            page: Page::Dashboard,
            nasa_form: NasaForm::default(),
            api_slots,
            logs: vec![],
            nasa_records: vec![],
            last_error: None,
            last_success: None,
            cache_enabled: true,
            show_request_inspector: true,
            last_request_url: String::new(),
            last_response_preview: String::new(),
            rate_counter: Vec::new(),
            app_dir,
        }
    }
}

impl eframe::App for SatelliteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(APP_NAME);
                ui.label(RichText::new(VERSION).color(egui::Color32::LIGHT_BLUE));
                ui.separator();
                if let Some(msg) = &self.last_success { ui.colored_label(egui::Color32::GREEN, msg); }
                if let Some(err) = &self.last_error { ui.colored_label(egui::Color32::RED, err); }
            });
        });

        egui::SidePanel::left("sidebar").min_width(190.0).show(ctx, |ui| {
            ui.heading("DATA TOOLKIT");
            ui.label("Professional API + provenance layer");
            ui.separator();
            nav_button(ui, &mut self.page, Page::Dashboard, "Dashboard");
            nav_button(ui, &mut self.page, Page::NasaPower, "NASA POWER");
            nav_button(ui, &mut self.page, Page::ApiSlots, "API Slots");
            nav_button(ui, &mut self.page, Page::SavedData, "Saved Data");
            nav_button(ui, &mut self.page, Page::Settings, "Settings");
            nav_button(ui, &mut self.page, Page::About, "About");
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Dashboard => self.dashboard(ui),
            Page::NasaPower => self.nasa_power(ui),
            Page::ApiSlots => self.api_slots(ui),
            Page::SavedData => self.saved_data(ui),
            Page::Settings => self.settings(ui),
            Page::About => self.about(ui),
        });
    }
}

fn nav_button(ui: &mut egui::Ui, page: &mut Page, target: Page, label: &str) {
    let selected = *page == target;
    if ui.selectable_label(selected, label).clicked() {
        *page = target;
    }
}

impl SatelliteApp {
    fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.label("Professional state overview: API status, cache, request logs, provenance.");
        ui.separator();
        ui.columns(4, |cols| {
            cols[0].group(|ui| { ui.label("Records"); ui.heading(self.nasa_records.len().to_string()); });
            cols[1].group(|ui| { ui.label("Requests"); ui.heading(self.logs.len().to_string()); });
            cols[2].group(|ui| { ui.label("Cache"); ui.heading(if self.cache_enabled { "ON" } else { "OFF" }); });
            cols[3].group(|ui| { ui.label("Last status"); ui.heading(self.logs.last().map(|l| l.status.as_str()).unwrap_or("none")); });
        });
        ui.separator();
        ui.heading("Recent logs");
        self.logs_table(ui);
    }

    fn nasa_power(&mut self, ui: &mut egui::Ui) {
        ui.heading("NASA POWER — Public API");
        ui.colored_label(egui::Color32::LIGHT_GREEN, "No API key required. This endpoint is public; provenance and request logs are still recorded.");
        ui.horizontal(|ui| {
            if ui.button("Load Tokyo Example").clicked() {
                self.nasa_form.lat = "35.6762".into(); self.nasa_form.lon = "139.6503".into();
                self.nasa_form.start = "20240501".into(); self.nasa_form.end = "20240531".into();
            }
            if ui.button("Load Pusey, France Example").clicked() {
                self.nasa_form.lat = "47.639".into(); self.nasa_form.lon = "6.130".into();
                self.nasa_form.start = "20240501".into(); self.nasa_form.end = "20240531".into();
            }
            if ui.button("Clear").clicked() { self.nasa_form = NasaForm::default(); }
        });
        ui.separator();

        ui.columns(2, |cols| {
            cols[0].group(|ui| {
                ui.heading("1. Request Parameters");
                ui.horizontal(|ui| { ui.label("Latitude"); ui.text_edit_singleline(&mut self.nasa_form.lat); });
                ui.horizontal(|ui| { ui.label("Longitude"); ui.text_edit_singleline(&mut self.nasa_form.lon); });
                ui.horizontal(|ui| { ui.label("Start YYYYMMDD"); ui.text_edit_singleline(&mut self.nasa_form.start); });
                ui.horizontal(|ui| { ui.label("End YYYYMMDD"); ui.text_edit_singleline(&mut self.nasa_form.end); });
                ui.label("Parameters");
                for (name, checked, desc) in &mut self.nasa_form.parameters {
                    ui.checkbox(checked, format!("{} — {}", name, desc));
                }
                ui.horizontal(|ui| { ui.label("Community"); ui.text_edit_singleline(&mut self.nasa_form.community); });
                ui.horizontal(|ui| { ui.label("Time format"); ui.text_edit_singleline(&mut self.nasa_form.time_format); });
                ui.checkbox(&mut self.cache_enabled, "Use cache when possible");
                if ui.button("Fetch Data").clicked() {
                    if let Err(e) = self.fetch_nasa() { self.last_error = Some(e.to_string()); self.last_success = None; }
                }
            });
            cols[1].group(|ui| {
                ui.heading("2. Response");
                ui.label(format!("Records: {}", self.nasa_records.len()));
                ui.horizontal(|ui| {
                    if ui.button("Export CSV").clicked() { self.export_csv(); }
                    if ui.button("Export JSON").clicked() { self.export_json(); }
                    if ui.button("Clear records").clicked() { self.nasa_records.clear(); }
                });
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    egui::Grid::new("nasa_grid").striped(true).show(ui, |ui| {
                        ui.label("Date"); ui.label("Irradiance"); ui.label("T2M"); ui.label("WS2M"); ui.end_row();
                        for r in self.nasa_records.iter().take(50) {
                            ui.label(&r.date);
                            ui.label(fmt_opt(r.allsky_sfc_sw_dwn));
                            ui.label(fmt_opt(r.t2m));
                            ui.label(fmt_opt(r.ws2m));
                            ui.end_row();
                        }
                    });
                });
            });
        });
        ui.separator();
        if self.show_request_inspector {
            ui.collapsing("Request Inspector", |ui| {
                ui.label("Last URL:");
                ui.code(&self.last_request_url);
                ui.label("Last response preview:");
                ui.code(&self.last_response_preview);
            });
        }
    }

    fn api_slots(&mut self, ui: &mut egui::Ui) {
        ui.heading("API Slots");
        ui.label("Public APIs are shown separately from protected APIs. Protected credentials are stored in OS keychain when available.");
        ui.separator();
        for slot in &mut self.api_slots {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(&slot.name);
                    ui.label(format!("Provider: {}", slot.provider));
                    ui.colored_label(egui::Color32::LIGHT_BLUE, format!("{:?}", slot.auth_mode));
                });
                ui.label(format!("Status: {}", slot.status));
                ui.label(format!("Endpoint: {}", slot.endpoint));
                ui.label(format!("Notes: {}", slot.notes));
                ui.label(format!("Rate limit: {}", slot.rate_limit_hint));
                if slot.auth_mode != ApiAuthMode::PublicNoAuth {
                    let mut key_value = String::new();
                    ui.horizontal(|ui| {
                        ui.label("Key/token"); ui.text_edit_singleline(&mut key_value);
                        if ui.button("Store").clicked() {
                            let _ = store_secret(&slot.key_name, &key_value);
                        }
                        if ui.button("Test read").clicked() {
                            slot.status = match read_secret(&slot.key_name) {
                                Ok(_) => "Key found in OS keychain".into(),
                                Err(e) => format!("Key read failed: {}", e),
                            };
                        }
                    });
                }
            });
        }
    }

    fn saved_data(&mut self, ui: &mut egui::Ui) {
        ui.heading("Saved Data");
        ui.label(format!("Data directory: {}", self.app_dir.display()));
        if ui.button("Open data directory path in log").clicked() {
            self.last_success = Some(format!("Exports: {}", self.app_dir.join("exports").display()));
        }
        ui.separator();
        self.logs_table(ui);
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.checkbox(&mut self.cache_enabled, "Enable cache");
        ui.checkbox(&mut self.show_request_inspector, "Show request inspector");
        ui.label("Professional production checklist:");
        ui.label("• separate public/protected API status");
        ui.label("• cache layer");
        ui.label("• source provenance");
        ui.label("• rate-limit monitor");
        ui.label("• request inspector");
        ui.label("• live request logs");
    }

    fn about(&mut self, ui: &mut egui::Ui) {
        ui.heading("About");
        ui.label("Satellite Data Toolkit Pro is a Rust desktop application for weather/satellite data workflows in energy trading.");
        ui.label("NASA POWER uses a public no-auth endpoint. EUMETSAT and some commercial providers require credentials.");
    }

    fn logs_table(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            egui::Grid::new("logs_grid").striped(true).show(ui, |ui| {
                ui.label("Time"); ui.label("Source"); ui.label("Status"); ui.label("Records"); ui.label("Cache"); ui.end_row();
                for l in self.logs.iter().rev().take(50) {
                    ui.label(&l.ts); ui.label(&l.source); ui.label(&l.status); ui.label(l.records.to_string()); ui.label(&l.cache); ui.end_row();
                }
            });
        });
    }

    fn selected_parameters(&self) -> String {
        self.nasa_form.parameters.iter().filter(|(_, c, _)| *c).map(|(n, _, _)| n.clone()).collect::<Vec<_>>().join(",")
    }

    fn build_nasa_url(&self) -> Result<String> {
        let lat: f64 = self.nasa_form.lat.replace(',', ".").parse()?;
        let lon: f64 = self.nasa_form.lon.replace(',', ".").parse()?;
        if self.nasa_form.start.len() != 8 || self.nasa_form.end.len() != 8 {
            return Err(anyhow!("Dates must be YYYYMMDD"));
        }
        let params = self.selected_parameters();
        if params.is_empty() { return Err(anyhow!("Select at least one parameter")); }
        let temporal = self.nasa_form.time_format.to_lowercase();
        let base = self.nasa_form.custom_endpoint.trim_end_matches('/');
        Ok(format!("{}/{}/point?parameters={}&community={}&longitude={}&latitude={}&start={}&end={}&format=JSON&time-standard=UTC",
            base, temporal, urlencoding::encode(&params), self.nasa_form.community, lon, lat, self.nasa_form.start, self.nasa_form.end))
    }

    fn fetch_nasa(&mut self) -> Result<()> {
        self.last_error = None;
        self.last_success = None;
        let url = self.build_nasa_url()?;
        self.last_request_url = url.clone();
        let start_time = Instant::now();
        let provenance_id = format!("nasa_power_{}", Utc::now().timestamp());
        let cache_key = sanitize_filename(&url);
        let cache_path = self.app_dir.join("cache").join(format!("{}.json", cache_key));
        let mut cache_state = "MISS".to_string();
        let body = if self.cache_enabled && cache_path.exists() {
            cache_state = "HIT".into();
            fs::read_to_string(&cache_path)?
        } else {
            self.rate_counter.push(Instant::now());
            self.rate_counter.retain(|t| t.elapsed() < Duration::from_secs(60));
            let resp = reqwest::blocking::get(&url)?;
            let status = resp.status();
            let text = resp.text()?;
            if !status.is_success() { return Err(anyhow!("NASA request failed: {} — {}", status, text)); }
            if self.cache_enabled { let _ = fs::write(&cache_path, &text); }
            text
        };
        self.last_response_preview = body.chars().take(1000).collect();
        let json: serde_json::Value = serde_json::from_str(&body)?;
        let mut records = Vec::new();
        let params = json.pointer("/properties/parameter").ok_or_else(|| anyhow!("Unexpected NASA response: missing properties.parameter"))?;
        let allsky = params.get("ALLSKY_SFC_SW_DWN");
        let t2m = params.get("T2M");
        let ws2m = params.get("WS2M");
        let mut dates = Vec::new();
        for source in [allsky, t2m, ws2m].iter().flatten() {
            if let Some(obj) = source.as_object() { for k in obj.keys() { if !dates.contains(k) { dates.push(k.clone()); } } }
        }
        dates.sort();
        for d in dates {
            records.push(NasaRecord {
                date: d.clone(),
                allsky_sfc_sw_dwn: allsky.and_then(|v| v.get(&d)).and_then(|v| v.as_f64()),
                t2m: t2m.and_then(|v| v.get(&d)).and_then(|v| v.as_f64()),
                ws2m: ws2m.and_then(|v| v.get(&d)).and_then(|v| v.as_f64()),
                raw: params.clone(),
            });
        }
        self.nasa_records = records;
        let log = RequestLog {
            ts: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            source: "NASA POWER".into(), status: "200".into(), method: "GET".into(), url,
            duration_ms: start_time.elapsed().as_millis(), records: self.nasa_records.len(), cache: cache_state, provenance_id,
        };
        self.logs.push(log);
        self.last_success = Some(format!("NASA data loaded: {} records", self.nasa_records.len()));
        Ok(())
    }

    fn export_csv(&mut self) {
        let path = self.app_dir.join("exports").join(format!("nasa_power_{}.csv", Utc::now().format("%Y%m%d_%H%M%S")));
        match csv::Writer::from_path(&path) {
            Ok(mut w) => {
                let _ = w.write_record(["date", "ALLSKY_SFC_SW_DWN", "T2M", "WS2M"]);
                for r in &self.nasa_records {
                    let _ = w.write_record([&r.date, &fmt_opt(r.allsky_sfc_sw_dwn), &fmt_opt(r.t2m), &fmt_opt(r.ws2m)]);
                }
                let _ = w.flush();
                self.last_success = Some(format!("CSV exported: {}", path.display()));
            }
            Err(e) => self.last_error = Some(e.to_string()),
        }
    }

    fn export_json(&mut self) {
        let path = self.app_dir.join("exports").join(format!("nasa_power_{}.json", Utc::now().format("%Y%m%d_%H%M%S")));
        match serde_json::to_string_pretty(&self.nasa_records).and_then(|s| fs::write(&path, s).map_err(serde_json::Error::io)) {
            Ok(_) => self.last_success = Some(format!("JSON exported: {}", path.display())),
            Err(e) => self.last_error = Some(e.to_string()),
        }
    }
}

fn fmt_opt(v: Option<f64>) -> String { v.map(|x| format!("{:.3}", x)).unwrap_or_else(|| "—".into()) }
fn sanitize_filename(s: &str) -> String { s.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect::<String>().chars().take(180).collect() }

fn store_secret(key: &str, value: &str) -> Result<()> {
    let entry = keyring::Entry::new(APP_NAME, key)?;
    entry.set_password(value)?;
    Ok(())
}
fn read_secret(key: &str) -> Result<String> {
    let entry = keyring::Entry::new(APP_NAME, key)?;
    Ok(entry.get_password()?)
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(APP_NAME, native_options, Box::new(|_cc| Ok(Box::<SatelliteApp>::default())))
}
