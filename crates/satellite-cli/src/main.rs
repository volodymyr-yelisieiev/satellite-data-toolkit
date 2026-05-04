use anyhow::Result;
use satellite_core::{fetch_power_dataset, PowerRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let dataset = fetch_power_dataset(PowerRequest {
        latitude: 40.7128,
        longitude: -74.006,
        start_date: "2024-05-01".to_string(),
        end_date: "2024-05-05".to_string(),
        parameters: vec![
            "ALLSKY_SFC_SW_DWN".to_string(),
            "T2M".to_string(),
            "WS2M".to_string(),
        ],
        temporal: "daily".to_string(),
        community: "RE".to_string(),
        time_standard: "LST".to_string(),
    })
    .await?;
    println!("{}", serde_json::to_string_pretty(&dataset)?);
    Ok(())
}
