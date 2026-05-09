use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::nasa_power::PowerDataset;

#[derive(Debug, Error)]
pub enum PvError {
    #[error("capacity must be greater than zero")]
    InvalidCapacity,
    #[error("losses must be between 0 and 100 percent")]
    InvalidLosses,
    #[error("inverter efficiency must be between 0 and 100 percent")]
    InvalidInverter,
    #[error("irradiance parameter is missing from the dataset")]
    MissingIrradiance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvEstimateInput {
    pub dataset: PowerDataset,
    pub capacity_kw: f64,
    pub irradiance_parameter: String,
    pub losses_percent: f64,
    pub inverter_efficiency_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PvEstimate {
    pub energy_kwh: f64,
    pub average_power_kw: f64,
    pub capacity_factor_percent: f64,
    pub performance_ratio: f64,
    pub record_count: usize,
    pub used_record_count: usize,
    pub missing_record_count: usize,
    pub unit_mode: String,
    pub method: String,
    pub assumptions: Vec<String>,
}

pub fn estimate_pv(input: PvEstimateInput) -> Result<PvEstimate, PvError> {
    if !input.capacity_kw.is_finite() || input.capacity_kw <= 0.0 {
        return Err(PvError::InvalidCapacity);
    }
    if !input.losses_percent.is_finite() || !(0.0..100.0).contains(&input.losses_percent) {
        return Err(PvError::InvalidLosses);
    }
    if !input.inverter_efficiency_percent.is_finite()
        || !(0.0..=100.0).contains(&input.inverter_efficiency_percent)
    {
        return Err(PvError::InvalidInverter);
    }

    let unit = input
        .dataset
        .units
        .get(&input.irradiance_parameter)
        .cloned()
        .ok_or(PvError::MissingIrradiance)?;
    let unit_mode = irradiance_unit_mode(&unit).ok_or(PvError::MissingIrradiance)?;
    let performance_ratio =
        (1.0 - input.losses_percent / 100.0) * (input.inverter_efficiency_percent / 100.0);
    let mut energy_kwh = 0.0;
    let mut used_record_count = 0;
    let hours_per_record = match unit_mode {
        IrradianceUnitMode::DailyKwhPerM2 => 24.0,
        IrradianceUnitMode::HourlyWhPerM2 => 1.0,
    };
    let total_period_hours = input.dataset.records.len() as f64 * hours_per_record;

    for record in &input.dataset.records {
        let Some(Some(irradiance)) = record.values.get(&input.irradiance_parameter) else {
            continue;
        };
        if *irradiance < 0.0 {
            continue;
        }
        used_record_count += 1;
        match unit_mode {
            IrradianceUnitMode::DailyKwhPerM2 => {
                energy_kwh += input.capacity_kw * irradiance * performance_ratio;
            }
            IrradianceUnitMode::HourlyWhPerM2 => {
                energy_kwh += input.capacity_kw * (irradiance / 1000.0) * performance_ratio;
            }
        }
    }

    let average_power_kw = if total_period_hours > 0.0 {
        energy_kwh / total_period_hours
    } else {
        0.0
    };
    let capacity_factor_percent = if input.capacity_kw > 0.0 && total_period_hours > 0.0 {
        (average_power_kw / input.capacity_kw) * 100.0
    } else {
        0.0
    };
    let missing_record_count = input
        .dataset
        .records
        .len()
        .saturating_sub(used_record_count);

    Ok(PvEstimate {
        energy_kwh,
        average_power_kw,
        capacity_factor_percent,
        performance_ratio,
        record_count: input.dataset.records.len(),
        used_record_count,
        missing_record_count,
        unit_mode: unit_mode.label().to_string(),
        method: "Local quick estimate".to_string(),
        assumptions: vec![
            "Approximate PV model: NASA POWER horizontal irradiance is treated as peak-sun-hours against nameplate capacity.".to_string(),
            "Plane-of-array transposition, module temperature, wind, tilt, azimuth, shading, clipping, and degradation are not modeled.".to_string(),
            "Missing irradiance records are counted and treated as zero production in average power and capacity-factor calculations.".to_string(),
            "Use PVWatts V8/NLR mode for higher-accuracy estimates when an API key is available.".to_string(),
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IrradianceUnitMode {
    DailyKwhPerM2,
    HourlyWhPerM2,
}

impl IrradianceUnitMode {
    fn label(self) -> &'static str {
        match self {
            Self::DailyKwhPerM2 => "daily_kwh_per_m2",
            Self::HourlyWhPerM2 => "hourly_wh_per_m2",
        }
    }
}

fn irradiance_unit_mode(unit: &str) -> Option<IrradianceUnitMode> {
    let normalized = unit.to_ascii_lowercase();
    if normalized.contains("kw-hr") || normalized.contains("kwh") {
        Some(IrradianceUnitMode::DailyKwhPerM2)
    } else if normalized.contains("wh/m") || normalized == "wh" {
        Some(IrradianceUnitMode::HourlyWhPerM2)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::nasa_power::{PowerDataset, PowerRecord, PowerRequest};

    use super::*;

    #[test]
    fn estimates_daily_energy_with_performance_ratio() {
        let dataset = PowerDataset {
            request: PowerRequest {
                latitude: 0.0,
                longitude: 0.0,
                start_date: "2024-05-01".to_string(),
                end_date: "2024-05-01".to_string(),
                parameters: vec!["ALLSKY_SFC_SW_DWN".to_string()],
                temporal: "daily".to_string(),
                community: "RE".to_string(),
                time_standard: "LST".to_string(),
            },
            records: vec![PowerRecord {
                raw_timestamp: "20240501".to_string(),
                timestamp: "2024-05-01".to_string(),
                values: BTreeMap::from([("ALLSKY_SFC_SW_DWN".to_string(), Some(5.0))]),
            }],
            units: BTreeMap::from([("ALLSKY_SFC_SW_DWN".to_string(), "kWh/m^2/day".to_string())]),
            long_names: BTreeMap::new(),
            status_code: 200,
            api_version: "test".to_string(),
            time_standard: "LST".to_string(),
            fill_value: -999.0,
            data_time_seconds: 0.0,
            process_time_seconds: 0.0,
            fetched_at: "now".to_string(),
        };
        let estimate = estimate_pv(PvEstimateInput {
            dataset,
            capacity_kw: 10.0,
            irradiance_parameter: "ALLSKY_SFC_SW_DWN".to_string(),
            losses_percent: 10.0,
            inverter_efficiency_percent: 95.0,
        })
        .unwrap();
        assert!((estimate.energy_kwh - 42.75).abs() < 0.001);
        assert_eq!(estimate.used_record_count, 1);
        assert_eq!(estimate.missing_record_count, 0);
        assert_eq!(estimate.unit_mode, "daily_kwh_per_m2");
    }

    #[test]
    fn treats_missing_irradiance_as_zero_for_capacity_factor() {
        let mut dataset = test_dataset("Wh/m^2");
        dataset.records = vec![
            PowerRecord {
                raw_timestamp: "2024050100".to_string(),
                timestamp: "2024-05-01 00:00".to_string(),
                values: BTreeMap::from([("ALLSKY_SFC_SW_DWN".to_string(), Some(500.0))]),
            },
            PowerRecord {
                raw_timestamp: "2024050101".to_string(),
                timestamp: "2024-05-01 01:00".to_string(),
                values: BTreeMap::from([("ALLSKY_SFC_SW_DWN".to_string(), None)]),
            },
        ];
        let estimate = estimate_pv(PvEstimateInput {
            dataset,
            capacity_kw: 10.0,
            irradiance_parameter: "ALLSKY_SFC_SW_DWN".to_string(),
            losses_percent: 0.0,
            inverter_efficiency_percent: 100.0,
        })
        .unwrap();
        assert!((estimate.energy_kwh - 5.0).abs() < 0.001);
        assert!((estimate.average_power_kw - 2.5).abs() < 0.001);
        assert_eq!(estimate.used_record_count, 1);
        assert_eq!(estimate.missing_record_count, 1);
    }

    #[test]
    fn rejects_non_finite_numeric_inputs() {
        let base = PvEstimateInput {
            dataset: test_dataset("kWh/m^2/day"),
            capacity_kw: 10.0,
            irradiance_parameter: "ALLSKY_SFC_SW_DWN".to_string(),
            losses_percent: 0.0,
            inverter_efficiency_percent: 100.0,
        };

        assert!(matches!(
            estimate_pv(PvEstimateInput {
                capacity_kw: f64::NAN,
                ..base.clone()
            }),
            Err(PvError::InvalidCapacity)
        ));
        assert!(matches!(
            estimate_pv(PvEstimateInput {
                capacity_kw: f64::INFINITY,
                ..base.clone()
            }),
            Err(PvError::InvalidCapacity)
        ));
        assert!(matches!(
            estimate_pv(PvEstimateInput {
                losses_percent: f64::NAN,
                ..base.clone()
            }),
            Err(PvError::InvalidLosses)
        ));
        assert!(matches!(
            estimate_pv(PvEstimateInput {
                inverter_efficiency_percent: f64::INFINITY,
                ..base
            }),
            Err(PvError::InvalidInverter)
        ));
    }

    fn test_dataset(unit: &str) -> PowerDataset {
        PowerDataset {
            request: PowerRequest {
                latitude: 0.0,
                longitude: 0.0,
                start_date: "2024-05-01".to_string(),
                end_date: "2024-05-01".to_string(),
                parameters: vec!["ALLSKY_SFC_SW_DWN".to_string()],
                temporal: "hourly".to_string(),
                community: "RE".to_string(),
                time_standard: "LST".to_string(),
            },
            records: vec![],
            units: BTreeMap::from([("ALLSKY_SFC_SW_DWN".to_string(), unit.to_string())]),
            long_names: BTreeMap::new(),
            status_code: 200,
            api_version: "test".to_string(),
            time_standard: "LST".to_string(),
            fill_value: -999.0,
            data_time_seconds: 0.0,
            process_time_seconds: 0.0,
            fetched_at: "now".to_string(),
        }
    }
}
