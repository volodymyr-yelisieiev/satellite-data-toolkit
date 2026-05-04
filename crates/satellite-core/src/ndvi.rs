use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tiff::{
    decoder::{Decoder, DecodingResult},
    encoder::{colortype, TiffEncoder},
};

#[derive(Debug, Error)]
pub enum NdviError {
    #[error("red and NIR arrays must have equal non-zero length")]
    ShapeMismatch,
    #[error("input and output paths are required")]
    MissingPath,
    #[error("red and NIR scale factors must be finite positive numbers")]
    InvalidScale,
    #[error("red and NIR rasters must have the same dimensions")]
    DimensionMismatch,
    #[error("TIFF read/write failed: {0}")]
    Tiff(String),
    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NdviJob {
    pub red_path: String,
    pub nir_path: String,
    pub output_path: String,
    pub red_scale: f64,
    pub nir_scale: f64,
    #[serde(default)]
    pub nodata_value: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NdviResult {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub valid_pixel_count: usize,
    pub nodata_pixel_count: usize,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub mean: Option<f32>,
    pub georeferencing_preserved: bool,
    pub warnings: Vec<String>,
}

pub fn validate_ndvi_inputs(job: &NdviJob) -> Result<String, NdviError> {
    validate_job(job)?;
    Ok("NDVI job is structurally valid. Pure-Rust TIFF execution is available; full CRS/GeoTIFF tag preservation requires the GDAL production path.".to_string())
}

pub fn run_ndvi(job: &NdviJob) -> Result<NdviResult, NdviError> {
    validate_job(job)?;
    let red = read_single_band_tiff(Path::new(&job.red_path))?;
    let nir = read_single_band_tiff(Path::new(&job.nir_path))?;
    if red.width != nir.width || red.height != nir.height {
        return Err(NdviError::DimensionMismatch);
    }

    let scaled_red = red
        .values
        .iter()
        .map(|value| f64::from(*value) * job.red_scale)
        .collect::<Vec<_>>();
    let scaled_nir = nir
        .values
        .iter()
        .map(|value| f64::from(*value) * job.nir_scale)
        .collect::<Vec<_>>();
    let mut ndvi = compute_ndvi_arrays(&scaled_red, &scaled_nir)?;
    let nodata_value = job.nodata_value.unwrap_or(f32::NAN);
    let mut valid_pixel_count = 0;
    let mut nodata_pixel_count = 0;
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut sum = 0.0_f64;

    for value in &mut ndvi {
        if value.is_nan() {
            *value = nodata_value;
        }
        if value.is_nan() || Some(*value) == job.nodata_value {
            nodata_pixel_count += 1;
        } else {
            valid_pixel_count += 1;
            min = min.min(*value);
            max = max.max(*value);
            sum += f64::from(*value);
        }
    }

    write_float_tiff(Path::new(&job.output_path), red.width, red.height, &ndvi)?;
    Ok(NdviResult {
        output_path: job.output_path.clone(),
        width: red.width,
        height: red.height,
        valid_pixel_count,
        nodata_pixel_count,
        min: (valid_pixel_count > 0).then_some(min),
        max: (valid_pixel_count > 0).then_some(max),
        mean: (valid_pixel_count > 0).then_some((sum / valid_pixel_count as f64) as f32),
        georeferencing_preserved: false,
        warnings: vec![
            "Pure-Rust TIFF path writes a Float32 TIFF and does not preserve CRS/GeoTIFF tags yet."
                .to_string(),
            "Use the planned GDAL production path before relying on georeferenced outputs."
                .to_string(),
        ],
    })
}

pub fn compute_ndvi_arrays(red: &[f64], nir: &[f64]) -> Result<Vec<f32>, NdviError> {
    if red.is_empty() || red.len() != nir.len() {
        return Err(NdviError::ShapeMismatch);
    }
    Ok(red
        .iter()
        .zip(nir.iter())
        .map(|(red, nir)| {
            if !red.is_finite() || !nir.is_finite() {
                return f32::NAN;
            }
            let denominator = nir + red;
            if denominator.abs() <= f64::EPSILON {
                f32::NAN
            } else {
                ((nir - red) / denominator).clamp(-1.0, 1.0) as f32
            }
        })
        .collect())
}

fn validate_job(job: &NdviJob) -> Result<(), NdviError> {
    if job.red_path.trim().is_empty()
        || job.nir_path.trim().is_empty()
        || job.output_path.trim().is_empty()
    {
        return Err(NdviError::MissingPath);
    }
    if !job.red_scale.is_finite()
        || !job.nir_scale.is_finite()
        || job.red_scale <= 0.0
        || job.nir_scale <= 0.0
    {
        return Err(NdviError::InvalidScale);
    }
    Ok(())
}

struct RasterBand {
    width: u32,
    height: u32,
    values: Vec<f32>,
}

fn read_single_band_tiff(path: &Path) -> Result<RasterBand, NdviError> {
    let file = File::open(path)?;
    let mut decoder =
        Decoder::new(BufReader::new(file)).map_err(|error| NdviError::Tiff(error.to_string()))?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|error| NdviError::Tiff(error.to_string()))?;
    let values = match decoder
        .read_image()
        .map_err(|error| NdviError::Tiff(error.to_string()))?
    {
        DecodingResult::U8(values) => values.into_iter().map(f32::from).collect(),
        DecodingResult::U16(values) => values.into_iter().map(f32::from).collect(),
        DecodingResult::U32(values) => values.into_iter().map(|value| value as f32).collect(),
        DecodingResult::I8(values) => values.into_iter().map(f32::from).collect(),
        DecodingResult::I16(values) => values.into_iter().map(f32::from).collect(),
        DecodingResult::I32(values) => values.into_iter().map(|value| value as f32).collect(),
        DecodingResult::F32(values) => values,
        DecodingResult::F64(values) => values.into_iter().map(|value| value as f32).collect(),
        _ => return Err(NdviError::Tiff("unsupported TIFF sample type".to_string())),
    };
    Ok(RasterBand {
        width,
        height,
        values,
    })
}

fn write_float_tiff(path: &Path, width: u32, height: u32, values: &[f32]) -> Result<(), NdviError> {
    let file = File::create(path)?;
    let mut encoder = TiffEncoder::new(BufWriter::new(file))
        .map_err(|error| NdviError::Tiff(error.to_string()))?;
    encoder
        .write_image::<colortype::Gray32Float>(width, height, values)
        .map_err(|error| NdviError::Tiff(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_ndvi_and_marks_zero_sum_as_nodata() {
        let values = compute_ndvi_arrays(&[0.2, 0.0], &[0.6, 0.0]).unwrap();
        assert!((values[0] - 0.5).abs() < 0.001);
        assert!(values[1].is_nan());
    }

    #[test]
    fn rejects_mismatched_arrays() {
        assert!(matches!(
            compute_ndvi_arrays(&[0.2], &[0.6, 0.3]),
            Err(NdviError::ShapeMismatch)
        ));
    }
}
