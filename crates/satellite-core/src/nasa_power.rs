use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum PowerError {
    #[error("latitude must be between -90 and 90")]
    InvalidLatitude,
    #[error("longitude must be between -180 and 180")]
    InvalidLongitude,
    #[error("start date and end date must be YYYY-MM-DD or YYYYMMDD")]
    InvalidDate,
    #[error("start date must be before or equal to end date")]
    InvalidRange,
    #[error("select between 1 and {max} parameters for {temporal} requests")]
    InvalidParameters { max: usize, temporal: String },
    #[error("date range is too large for {temporal} requests; maximum is {max_days} days")]
    DateRangeTooLarge { temporal: String, max_days: i64 },
    #[error("NASA POWER response did not contain any records")]
    EmptyResponse,
    #[error("request timed out after 60 seconds")]
    Timeout,
    #[error("parameter names must be non-empty and contain only letters, numbers, underscore, dash, or slash")]
    InvalidParameterName,
    #[error("temporal must be daily or hourly")]
    InvalidTemporal,
    #[error("NASA POWER returned status {status}: {body}")]
    ApiStatus { status: u16, body: String },
    #[error("failed to build NASA POWER URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PowerRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub start_date: String,
    pub end_date: String,
    pub parameters: Vec<String>,
    pub temporal: String,
    pub community: String,
    pub time_standard: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PowerRecord {
    #[serde(default)]
    pub raw_timestamp: String,
    pub timestamp: String,
    pub values: BTreeMap<String, Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PowerDataset {
    pub request: PowerRequest,
    pub records: Vec<PowerRecord>,
    pub units: BTreeMap<String, String>,
    pub long_names: BTreeMap<String, String>,
    pub status_code: u16,
    pub api_version: String,
    pub time_standard: String,
    pub fill_value: f64,
    pub data_time_seconds: f64,
    pub process_time_seconds: f64,
    pub fetched_at: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    properties: ApiProperties,
    header: ApiHeader,
    parameters: BTreeMap<String, ApiParameter>,
    #[serde(default)]
    times: ApiTimes,
}

#[derive(Debug, Deserialize)]
struct ApiProperties {
    parameter: BTreeMap<String, BTreeMap<String, f64>>,
}

#[derive(Debug, Deserialize)]
struct ApiHeader {
    #[serde(default)]
    api: ApiInfo,
    #[serde(default)]
    fill_value: f64,
    #[serde(default)]
    time_standard: String,
}

#[derive(Debug, Default, Deserialize)]
struct ApiInfo {
    #[serde(default)]
    version: String,
}

#[derive(Debug, Deserialize)]
struct ApiParameter {
    #[serde(default)]
    units: String,
    #[serde(default)]
    longname: String,
}

#[derive(Debug, Default, Deserialize)]
struct ApiTimes {
    #[serde(default)]
    data: f64,
    #[serde(default)]
    process: f64,
}

pub async fn fetch_power_dataset(request: PowerRequest) -> Result<PowerDataset, PowerError> {
    validate_request(&request)?;
    let url = build_url(&request)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let response = client.get(url).send().await.map_err(|error| {
        if error.is_timeout() {
            PowerError::Timeout
        } else {
            PowerError::Request(error)
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(PowerError::ApiStatus {
            status: status.as_u16(),
            body: response.text().await.unwrap_or_default(),
        });
    }
    let parsed = response.json::<ApiResponse>().await?;
    normalize_power_response(request, parsed, status.as_u16())
}

fn build_url(request: &PowerRequest) -> Result<Url, PowerError> {
    let temporal = request.temporal.to_ascii_lowercase();
    let start = compact_date(&request.start_date).ok_or(PowerError::InvalidDate)?;
    let end = compact_date(&request.end_date).ok_or(PowerError::InvalidDate)?;
    let mut url = Url::parse(&format!(
        "https://power.larc.nasa.gov/api/temporal/{}/point",
        temporal
    ))?;
    url.query_pairs_mut()
        .append_pair("parameters", &request.parameters.join(","))
        .append_pair("community", &request.community)
        .append_pair("longitude", &request.longitude.to_string())
        .append_pair("latitude", &request.latitude.to_string())
        .append_pair("start", &start)
        .append_pair("end", &end)
        .append_pair("format", "JSON")
        .append_pair("time-standard", &request.time_standard);
    Ok(url)
}

fn validate_request(request: &PowerRequest) -> Result<(), PowerError> {
    if !(-90.0..=90.0).contains(&request.latitude) {
        return Err(PowerError::InvalidLatitude);
    }
    if !(-180.0..=180.0).contains(&request.longitude) {
        return Err(PowerError::InvalidLongitude);
    }
    let temporal = request.temporal.to_ascii_lowercase();
    if temporal != "daily" && temporal != "hourly" {
        return Err(PowerError::InvalidTemporal);
    }
    let max_parameters = if temporal == "hourly" { 15 } else { 20 };
    if request.parameters.is_empty() || request.parameters.len() > max_parameters {
        return Err(PowerError::InvalidParameters {
            max: max_parameters,
            temporal,
        });
    }
    if request
        .parameters
        .iter()
        .any(|parameter| !is_valid_parameter(parameter))
    {
        return Err(PowerError::InvalidParameterName);
    }
    let start = parse_date(&request.start_date).ok_or(PowerError::InvalidDate)?;
    let end = parse_date(&request.end_date).ok_or(PowerError::InvalidDate)?;
    if start > end {
        return Err(PowerError::InvalidRange);
    }
    let days = (end - start).num_days() + 1;
    let max_days = if temporal == "hourly" { 366 } else { 3650 };
    if days > max_days {
        return Err(PowerError::DateRangeTooLarge { temporal, max_days });
    }
    Ok(())
}

fn is_valid_parameter(parameter: &str) -> bool {
    let trimmed = parameter.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '/')
        })
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(value, "%Y%m%d"))
        .ok()
}

fn compact_date(value: &str) -> Option<String> {
    parse_date(value).map(|date| date.format("%Y%m%d").to_string())
}

fn normalize_power_response(
    request: PowerRequest,
    response: ApiResponse,
    status_code: u16,
) -> Result<PowerDataset, PowerError> {
    let fill_value = response.header.fill_value;
    let mut keys = BTreeSet::new();
    for values in response.properties.parameter.values() {
        keys.extend(values.keys().cloned());
    }

    if keys.is_empty() {
        return Err(PowerError::EmptyResponse);
    }

    let records = keys
        .into_iter()
        .map(|key| {
            let values = request
                .parameters
                .iter()
                .map(|parameter| {
                    let value = response
                        .properties
                        .parameter
                        .get(parameter)
                        .and_then(|series| series.get(&key))
                        .copied()
                        .filter(|value| (*value - fill_value).abs() > f64::EPSILON);
                    (parameter.clone(), value)
                })
                .collect();
            PowerRecord {
                raw_timestamp: key.clone(),
                timestamp: normalize_timestamp(&key),
                values,
            }
        })
        .collect();

    let units = response
        .parameters
        .iter()
        .map(|(key, value)| (key.clone(), value.units.clone()))
        .collect();
    let long_names = response
        .parameters
        .iter()
        .map(|(key, value)| (key.clone(), value.longname.clone()))
        .collect();

    Ok(PowerDataset {
        request,
        records,
        units,
        long_names,
        status_code,
        api_version: response.header.api.version,
        time_standard: response.header.time_standard,
        fill_value,
        data_time_seconds: response.times.data,
        process_time_seconds: response.times.process,
        fetched_at: Utc::now().to_rfc3339(),
    })
}

fn normalize_timestamp(key: &str) -> String {
    match key.len() {
        8 => format!("{}-{}-{}", &key[0..4], &key[4..6], &key[6..8]),
        10 => format!(
            "{}-{}-{} {:0>2}:00",
            &key[0..4],
            &key[4..6],
            &key[6..8],
            &key[8..10]
        ),
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_daily_response() {
        let response = ApiResponse {
            properties: ApiProperties {
                parameter: BTreeMap::from([
                    (
                        "ALLSKY_SFC_SW_DWN".to_string(),
                        BTreeMap::from([
                            ("20240501".to_string(), 6.3082),
                            ("20240502".to_string(), -999.0),
                        ]),
                    ),
                    (
                        "T2M".to_string(),
                        BTreeMap::from([("20240501".to_string(), 13.13)]),
                    ),
                ]),
            },
            header: ApiHeader {
                api: ApiInfo {
                    version: "v2.8.11".to_string(),
                },
                fill_value: -999.0,
                time_standard: "LST".to_string(),
            },
            parameters: BTreeMap::from([
                (
                    "ALLSKY_SFC_SW_DWN".to_string(),
                    ApiParameter {
                        units: "kWh/m^2/day".to_string(),
                        longname: "All Sky Surface Shortwave Downward Irradiance".to_string(),
                    },
                ),
                (
                    "T2M".to_string(),
                    ApiParameter {
                        units: "C".to_string(),
                        longname: "Temperature at 2 Meters".to_string(),
                    },
                ),
            ]),
            times: ApiTimes {
                data: 0.3,
                process: 0.01,
            },
        };
        let dataset = normalize_power_response(
            PowerRequest {
                latitude: 40.7128,
                longitude: -74.006,
                start_date: "2024-05-01".to_string(),
                end_date: "2024-05-02".to_string(),
                parameters: vec!["ALLSKY_SFC_SW_DWN".to_string(), "T2M".to_string()],
                temporal: "daily".to_string(),
                community: "RE".to_string(),
                time_standard: "LST".to_string(),
            },
            response,
            200,
        )
        .unwrap();
        assert_eq!(dataset.records.len(), 2);
        assert_eq!(dataset.records[0].timestamp, "2024-05-01");
        assert_eq!(dataset.records[0].raw_timestamp, "20240501");
        assert_eq!(dataset.records[1].values["ALLSKY_SFC_SW_DWN"], None);
    }

    #[test]
    fn rejects_hourly_requests_over_15_parameters() {
        let request = PowerRequest {
            latitude: 0.0,
            longitude: 0.0,
            start_date: "2024-05-01".to_string(),
            end_date: "2024-05-02".to_string(),
            parameters: (0..16).map(|index| format!("T2M_{index}")).collect(),
            temporal: "hourly".to_string(),
            community: "RE".to_string(),
            time_standard: "LST".to_string(),
        };
        assert!(matches!(
            validate_request(&request),
            Err(PowerError::InvalidParameters { max: 15, .. })
        ));
    }

    #[test]
    fn normalizes_hourly_timestamp_and_preserves_raw_key() {
        assert_eq!(normalize_timestamp("2024050104"), "2024-05-01 04:00");
    }
}
