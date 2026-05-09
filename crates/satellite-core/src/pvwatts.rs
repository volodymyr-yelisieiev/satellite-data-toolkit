use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

const PVWATTS_ENDPOINT: &str = "https://developer.nlr.gov/api/pvwatts/v8.json";

#[derive(Debug, Error)]
pub enum PvWattsError {
    #[error("PVWatts API key is required")]
    MissingApiKey,
    #[error("latitude must be between -90 and 90")]
    InvalidLatitude,
    #[error("longitude must be between -180 and 180")]
    InvalidLongitude,
    #[error("system capacity must be greater than zero")]
    InvalidCapacity,
    #[error("losses must be between -5 and 100 percent")]
    InvalidLosses,
    #[error("tilt must be between 0 and 90 degrees")]
    InvalidTilt,
    #[error("azimuth must be at least 0 and less than 360 degrees")]
    InvalidAzimuth,
    #[error("PVWatts inverter efficiency must be between 90 and 99.5 percent")]
    InvalidInverterEfficiency,
    #[error("module type must be 0, 1, or 2")]
    InvalidModuleType,
    #[error("array type must be between 0 and 4")]
    InvalidArrayType,
    #[error("timeframe must be monthly or hourly")]
    InvalidTimeframe,
    #[error("PVWatts returned status {status}: {body}")]
    ApiStatus { status: u16, body: String },
    #[error("PVWatts reported errors: {0}")]
    ApiErrors(String),
    #[error("failed to build PVWatts URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvWattsRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub system_capacity_kw: f64,
    pub tilt_degrees: f64,
    pub azimuth_degrees: f64,
    pub losses_percent: f64,
    #[serde(default)]
    pub inverter_efficiency_percent: Option<f64>,
    #[serde(default = "default_module_type")]
    pub module_type: u8,
    #[serde(default = "default_array_type")]
    pub array_type: u8,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvWattsResult {
    pub ac_annual_kwh: f64,
    pub solrad_annual_kwh_per_m2_day: f64,
    pub capacity_factor_percent: f64,
    pub station_info: serde_json::Value,
    pub warnings: Vec<String>,
    pub method: String,
}

#[derive(Debug, Deserialize)]
struct PvWattsApiResponse {
    outputs: PvWattsOutputs,
    #[serde(default)]
    station_info: serde_json::Value,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PvWattsOutputs {
    ac_annual: f64,
    solrad_annual: f64,
    capacity_factor: f64,
}

pub async fn estimate_pvwatts(
    request: PvWattsRequest,
    api_key: &str,
) -> Result<PvWattsResult, PvWattsError> {
    validate_request(&request)?;
    if api_key.trim().is_empty() {
        return Err(PvWattsError::MissingApiKey);
    }
    let url = build_url(&request, api_key)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(PvWattsError::ApiStatus {
            status: status.as_u16(),
            body: response.text().await.unwrap_or_default(),
        });
    }
    let parsed = response.json::<PvWattsApiResponse>().await?;
    if !parsed.errors.is_empty() {
        return Err(PvWattsError::ApiErrors(parsed.errors.join("; ")));
    }
    Ok(PvWattsResult {
        ac_annual_kwh: parsed.outputs.ac_annual,
        solrad_annual_kwh_per_m2_day: parsed.outputs.solrad_annual,
        capacity_factor_percent: parsed.outputs.capacity_factor,
        station_info: parsed.station_info,
        warnings: parsed.warnings,
        method: "PVWatts V8/NLR".to_string(),
    })
}

fn validate_request(request: &PvWattsRequest) -> Result<(), PvWattsError> {
    if !(-90.0..=90.0).contains(&request.latitude) {
        return Err(PvWattsError::InvalidLatitude);
    }
    if !(-180.0..=180.0).contains(&request.longitude) {
        return Err(PvWattsError::InvalidLongitude);
    }
    if request.system_capacity_kw <= 0.0 {
        return Err(PvWattsError::InvalidCapacity);
    }
    if !(-5.0..100.0).contains(&request.losses_percent) {
        return Err(PvWattsError::InvalidLosses);
    }
    if !(0.0..=90.0).contains(&request.tilt_degrees) {
        return Err(PvWattsError::InvalidTilt);
    }
    if !(0.0..360.0).contains(&request.azimuth_degrees) {
        return Err(PvWattsError::InvalidAzimuth);
    }
    if request
        .inverter_efficiency_percent
        .is_some_and(|value| !(90.0..=99.5).contains(&value))
    {
        return Err(PvWattsError::InvalidInverterEfficiency);
    }
    if request.module_type > 2 {
        return Err(PvWattsError::InvalidModuleType);
    }
    if request.array_type > 4 {
        return Err(PvWattsError::InvalidArrayType);
    }
    if !matches!(request.timeframe.as_str(), "monthly" | "hourly") {
        return Err(PvWattsError::InvalidTimeframe);
    }
    Ok(())
}

fn build_url(request: &PvWattsRequest, api_key: &str) -> Result<Url, PvWattsError> {
    let mut url = Url::parse(PVWATTS_ENDPOINT)?;
    let mut query = url.query_pairs_mut();
    query
        .append_pair("api_key", api_key)
        .append_pair("lat", &request.latitude.to_string())
        .append_pair("lon", &request.longitude.to_string())
        .append_pair("system_capacity", &request.system_capacity_kw.to_string())
        .append_pair("module_type", &request.module_type.to_string())
        .append_pair("losses", &request.losses_percent.to_string())
        .append_pair("array_type", &request.array_type.to_string())
        .append_pair("tilt", &request.tilt_degrees.to_string())
        .append_pair("azimuth", &request.azimuth_degrees.to_string())
        .append_pair("timeframe", &request.timeframe);
    if let Some(inverter_efficiency) = request.inverter_efficiency_percent {
        query.append_pair("inv_eff", &inverter_efficiency.to_string());
    }
    drop(query);
    Ok(url)
}

fn default_module_type() -> u8 {
    0
}

fn default_array_type() -> u8 {
    1
}

fn default_timeframe() -> String {
    "monthly".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_missing_api_key() {
        let request = valid_request();
        let result = estimate_pvwatts(request, "").await;
        assert!(matches!(result, Err(PvWattsError::MissingApiKey)));
    }

    #[test]
    fn builds_expected_url_shape() {
        let url = build_url(&valid_request(), "secret").unwrap();
        let query = url.query().unwrap();
        assert!(query.contains("api_key=secret"));
        assert!(query.contains("system_capacity=10"));
        assert!(query.contains("inv_eff=96"));
        assert!(query.contains("timeframe=monthly"));
    }

    #[test]
    fn validation_matches_pvwatts_parameter_ranges() {
        let mut request = valid_request();
        request.losses_percent = -1.0;
        validate_request(&request).unwrap();

        request.azimuth_degrees = 360.0;
        assert!(matches!(
            validate_request(&request),
            Err(PvWattsError::InvalidAzimuth)
        ));

        request = valid_request();
        request.inverter_efficiency_percent = Some(99.5);
        validate_request(&request).unwrap();

        request.inverter_efficiency_percent = Some(89.9);
        assert!(matches!(
            validate_request(&request),
            Err(PvWattsError::InvalidInverterEfficiency)
        ));

        request = valid_request();
        request.inverter_efficiency_percent = Some(99.6);
        assert!(matches!(
            validate_request(&request),
            Err(PvWattsError::InvalidInverterEfficiency)
        ));

        request = valid_request();
        request.module_type = 3;
        assert!(matches!(
            validate_request(&request),
            Err(PvWattsError::InvalidModuleType)
        ));

        request = valid_request();
        request.array_type = 5;
        assert!(matches!(
            validate_request(&request),
            Err(PvWattsError::InvalidArrayType)
        ));

        request = valid_request();
        request.timeframe = "daily".to_string();
        assert!(matches!(
            validate_request(&request),
            Err(PvWattsError::InvalidTimeframe)
        ));
    }

    fn valid_request() -> PvWattsRequest {
        PvWattsRequest {
            latitude: 40.7128,
            longitude: -74.006,
            system_capacity_kw: 10.0,
            tilt_degrees: 30.0,
            azimuth_degrees: 180.0,
            losses_percent: 14.0,
            inverter_efficiency_percent: Some(96.0),
            module_type: 0,
            array_type: 1,
            timeframe: "monthly".to_string(),
        }
    }
}
