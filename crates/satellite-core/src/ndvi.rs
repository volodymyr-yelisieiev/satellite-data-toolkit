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
    tags::Tag,
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
    Ok("NDVI job is structurally valid. Pure-Rust TIFF execution preserves common GeoTIFF CRS/geotransform tags when they are present on the Red band input.".to_string())
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
    let input_nodata_value = job
        .nodata_value
        .or(red.metadata.nodata_value)
        .or(nir.metadata.nodata_value);
    let mut ndvi = compute_ndvi_arrays_with_nodata(
        &scaled_red,
        &scaled_nir,
        input_nodata_value.map(f64::from),
    )?;
    let output_nodata_value = input_nodata_value.unwrap_or(f32::NAN);
    let mut valid_pixel_count = 0;
    let mut nodata_pixel_count = 0;
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut sum = 0.0_f64;

    for value in &mut ndvi {
        let is_nodata = value.is_nan();
        if is_nodata {
            *value = output_nodata_value;
        }
        if is_nodata {
            nodata_pixel_count += 1;
        } else {
            valid_pixel_count += 1;
            min = min.min(*value);
            max = max.max(*value);
            sum += f64::from(*value);
        }
    }

    let georeferencing_preserved = write_float_tiff(
        Path::new(&job.output_path),
        red.width,
        red.height,
        &ndvi,
        &red.metadata,
        input_nodata_value,
    )?;
    let warnings = if georeferencing_preserved {
        vec![]
    } else {
        vec![
            "Input Red band did not contain GeoTIFF CRS/geotransform tags to preserve.".to_string(),
        ]
    };
    Ok(NdviResult {
        output_path: job.output_path.clone(),
        width: red.width,
        height: red.height,
        valid_pixel_count,
        nodata_pixel_count,
        min: (valid_pixel_count > 0).then_some(min),
        max: (valid_pixel_count > 0).then_some(max),
        mean: (valid_pixel_count > 0).then_some((sum / valid_pixel_count as f64) as f32),
        georeferencing_preserved,
        warnings,
    })
}

pub fn compute_ndvi_arrays(red: &[f64], nir: &[f64]) -> Result<Vec<f32>, NdviError> {
    compute_ndvi_arrays_with_nodata(red, nir, None)
}

fn compute_ndvi_arrays_with_nodata(
    red: &[f64],
    nir: &[f64],
    nodata_value: Option<f64>,
) -> Result<Vec<f32>, NdviError> {
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
            if nodata_value.is_some_and(|nodata| {
                (*red - nodata).abs() <= f64::EPSILON || (*nir - nodata).abs() <= f64::EPSILON
            }) {
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
    metadata: GeoTiffMetadata,
}

#[derive(Debug, Clone, Default)]
struct GeoTiffMetadata {
    model_pixel_scale: Option<Vec<f64>>,
    model_tiepoint: Option<Vec<f64>>,
    model_transformation: Option<Vec<f64>>,
    geo_key_directory: Option<Vec<u16>>,
    geo_double_params: Option<Vec<f64>>,
    geo_ascii_params: Option<String>,
    nodata_value: Option<f32>,
    nodata_ascii: Option<String>,
}

impl GeoTiffMetadata {
    fn has_georeferencing(&self) -> bool {
        self.model_pixel_scale.is_some()
            || self.model_tiepoint.is_some()
            || self.model_transformation.is_some()
            || self.geo_key_directory.is_some()
            || self.geo_double_params.is_some()
            || self.geo_ascii_params.is_some()
    }
}

fn read_single_band_tiff(path: &Path) -> Result<RasterBand, NdviError> {
    let file = File::open(path)?;
    let mut decoder =
        Decoder::new(BufReader::new(file)).map_err(|error| NdviError::Tiff(error.to_string()))?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|error| NdviError::Tiff(error.to_string()))?;
    let metadata = read_geotiff_metadata(&mut decoder)?;
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
        metadata,
    })
}

fn write_float_tiff(
    path: &Path,
    width: u32,
    height: u32,
    values: &[f32],
    metadata: &GeoTiffMetadata,
    nodata_value: Option<f32>,
) -> Result<bool, NdviError> {
    let file = File::create(path)?;
    let mut encoder = TiffEncoder::new(BufWriter::new(file))
        .map_err(|error| NdviError::Tiff(error.to_string()))?;
    let mut image = encoder
        .new_image::<colortype::Gray32Float>(width, height)
        .map_err(|error| NdviError::Tiff(error.to_string()))?;
    write_geotiff_metadata(&mut image, metadata, nodata_value)?;
    image
        .write_data(values)
        .map_err(|error| NdviError::Tiff(error.to_string()))?;
    Ok(metadata.has_georeferencing())
}

fn read_geotiff_metadata<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<GeoTiffMetadata, NdviError> {
    let nodata_ascii = optional_ascii_tag(decoder, Tag::GdalNodata)?;
    let nodata_value = nodata_ascii
        .as_deref()
        .map(str::trim)
        .and_then(|value| value.parse::<f32>().ok());
    Ok(GeoTiffMetadata {
        model_pixel_scale: optional_f64_vec_tag(decoder, Tag::ModelPixelScaleTag)?,
        model_tiepoint: optional_f64_vec_tag(decoder, Tag::ModelTiepointTag)?,
        model_transformation: optional_f64_vec_tag(decoder, Tag::ModelTransformationTag)?,
        geo_key_directory: optional_u16_vec_tag(decoder, Tag::GeoKeyDirectoryTag)?,
        geo_double_params: optional_f64_vec_tag(decoder, Tag::GeoDoubleParamsTag)?,
        geo_ascii_params: optional_ascii_tag(decoder, Tag::GeoAsciiParamsTag)?,
        nodata_value,
        nodata_ascii,
    })
}

fn optional_f64_vec_tag<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<f64>>, NdviError> {
    match decoder.find_tag(tag) {
        Ok(Some(value)) => value
            .into_f64_vec()
            .map(Some)
            .map_err(|error| NdviError::Tiff(error.to_string())),
        Ok(None) => Ok(None),
        Err(error) => Err(NdviError::Tiff(error.to_string())),
    }
}

fn optional_u16_vec_tag<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<u16>>, NdviError> {
    match decoder.find_tag(tag) {
        Ok(Some(value)) => value
            .into_u16_vec()
            .map(Some)
            .map_err(|error| NdviError::Tiff(error.to_string())),
        Ok(None) => Ok(None),
        Err(error) => Err(NdviError::Tiff(error.to_string())),
    }
}

fn optional_ascii_tag<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<String>, NdviError> {
    match decoder.find_tag(tag) {
        Ok(Some(value)) => value
            .into_string()
            .map(Some)
            .map_err(|error| NdviError::Tiff(error.to_string())),
        Ok(None) => Ok(None),
        Err(error) => Err(NdviError::Tiff(error.to_string())),
    }
}

fn write_geotiff_metadata<W: std::io::Write + std::io::Seek, K>(
    image: &mut tiff::encoder::ImageEncoder<W, colortype::Gray32Float, K>,
    metadata: &GeoTiffMetadata,
    nodata_value: Option<f32>,
) -> Result<(), NdviError>
where
    K: tiff::encoder::TiffKind,
{
    let encoder = image.encoder();
    if let Some(values) = &metadata.model_pixel_scale {
        encoder
            .write_tag(Tag::ModelPixelScaleTag, values.as_slice())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    if let Some(values) = &metadata.model_tiepoint {
        encoder
            .write_tag(Tag::ModelTiepointTag, values.as_slice())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    if let Some(values) = &metadata.model_transformation {
        encoder
            .write_tag(Tag::ModelTransformationTag, values.as_slice())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    if let Some(values) = &metadata.geo_key_directory {
        encoder
            .write_tag(Tag::GeoKeyDirectoryTag, values.as_slice())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    if let Some(values) = &metadata.geo_double_params {
        encoder
            .write_tag(Tag::GeoDoubleParamsTag, values.as_slice())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    if let Some(value) = &metadata.geo_ascii_params {
        encoder
            .write_tag(Tag::GeoAsciiParamsTag, value.as_str())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    let nodata_ascii = nodata_value
        .map(|value| value.to_string())
        .or_else(|| metadata.nodata_ascii.clone());
    if let Some(value) = nodata_ascii {
        encoder
            .write_tag(Tag::GdalNodata, value.as_str())
            .map_err(|error| NdviError::Tiff(error.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn run_ndvi_preserves_geotiff_tags_and_uses_input_nodata() {
        let red_path = temp_tiff_path("red_geotiff");
        let nir_path = temp_tiff_path("nir_geotiff");
        let output_path = temp_tiff_path("ndvi_geotiff");

        write_test_tiff(
            &red_path,
            &[-9999.0, 0.2],
            Some(TestGeoTags {
                model_pixel_scale: vec![30.0, 30.0, 0.0],
                model_tiepoint: vec![0.0, 0.0, 0.0, 440_720.0, 3_751_320.0, 0.0],
                geo_key_directory: vec![1, 1, 0, 1, 1024, 0, 1, 1],
                nodata: "-9999".to_string(),
            }),
        );
        write_test_tiff(&nir_path, &[0.5, 0.6], None);

        let result = run_ndvi(&NdviJob {
            red_path: red_path.to_string_lossy().to_string(),
            nir_path: nir_path.to_string_lossy().to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            red_scale: 1.0,
            nir_scale: 1.0,
            nodata_value: None,
        })
        .unwrap();

        assert!(result.georeferencing_preserved);
        assert_eq!(result.valid_pixel_count, 1);
        assert_eq!(result.nodata_pixel_count, 1);
        assert!(result.warnings.is_empty());

        let file = File::open(&output_path).unwrap();
        let mut decoder = Decoder::new(BufReader::new(file)).unwrap();
        assert_eq!(decoder.dimensions().unwrap(), (2, 1));
        assert_eq!(
            decoder.get_tag_f64_vec(Tag::ModelPixelScaleTag).unwrap(),
            vec![30.0, 30.0, 0.0]
        );
        assert_eq!(
            decoder.get_tag_u16_vec(Tag::GeoKeyDirectoryTag).unwrap(),
            vec![1, 1, 0, 1, 1024, 0, 1, 1]
        );
        assert_eq!(
            decoder.get_tag_ascii_string(Tag::GdalNodata).unwrap(),
            "-9999"
        );

        let DecodingResult::F32(values) = decoder.read_image().unwrap() else {
            panic!("expected Float32 NDVI output");
        };
        assert_eq!(values[0], -9999.0);
        assert!((values[1] - 0.5).abs() < 0.001);

        let _ = std::fs::remove_file(red_path);
        let _ = std::fs::remove_file(nir_path);
        let _ = std::fs::remove_file(output_path);
    }

    struct TestGeoTags {
        model_pixel_scale: Vec<f64>,
        model_tiepoint: Vec<f64>,
        geo_key_directory: Vec<u16>,
        nodata: String,
    }

    fn write_test_tiff(path: &Path, values: &[f32], tags: Option<TestGeoTags>) {
        let file = File::create(path).unwrap();
        let mut encoder = TiffEncoder::new(BufWriter::new(file)).unwrap();
        let mut image = encoder.new_image::<colortype::Gray32Float>(2, 1).unwrap();
        if let Some(tags) = tags {
            image
                .encoder()
                .write_tag(Tag::ModelPixelScaleTag, tags.model_pixel_scale.as_slice())
                .unwrap();
            image
                .encoder()
                .write_tag(Tag::ModelTiepointTag, tags.model_tiepoint.as_slice())
                .unwrap();
            image
                .encoder()
                .write_tag(Tag::GeoKeyDirectoryTag, tags.geo_key_directory.as_slice())
                .unwrap();
            image
                .encoder()
                .write_tag(Tag::GdalNodata, tags.nodata.as_str())
                .unwrap();
        }
        image.write_data(values).unwrap();
    }

    fn temp_tiff_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "satellite_data_toolkit_{name}_{}_{}.tif",
            std::process::id(),
            nanos
        ))
    }
}
