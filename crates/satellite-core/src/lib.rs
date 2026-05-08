pub mod nasa_power;
pub mod ndvi;
pub mod pv;
pub mod pvwatts;

pub const DEFAULT_HTTP_TIMEOUT_SECONDS: u64 = 60;
pub const MIN_HTTP_TIMEOUT_SECONDS: u64 = 10;
pub const MAX_HTTP_TIMEOUT_SECONDS: u64 = 300;

pub fn normalize_http_timeout_seconds(seconds: u64) -> u64 {
    seconds.clamp(MIN_HTTP_TIMEOUT_SECONDS, MAX_HTTP_TIMEOUT_SECONDS)
}

pub use nasa_power::{
    fetch_power_dataset, fetch_power_dataset_with_timeout, PowerDataset, PowerRecord, PowerRequest,
};
pub use ndvi::{compute_ndvi_arrays, run_ndvi, validate_ndvi_inputs, NdviJob, NdviResult};
pub use pv::{estimate_pv, PvEstimate, PvEstimateInput};
pub use pvwatts::{estimate_pvwatts, estimate_pvwatts_with_timeout, PvWattsRequest, PvWattsResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_http_timeout_seconds() {
        assert_eq!(normalize_http_timeout_seconds(0), MIN_HTTP_TIMEOUT_SECONDS);
        assert_eq!(normalize_http_timeout_seconds(60), 60);
        assert_eq!(
            normalize_http_timeout_seconds(999),
            MAX_HTTP_TIMEOUT_SECONDS
        );
    }
}
