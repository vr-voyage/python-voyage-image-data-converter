use pyo3::prelude::*;

use ddsfile::{AlphaMode, Caps2, D3D10ResourceDimension, Dds, DxgiFormat, NewDxgiParams};
use intel_tex_2::bc5;
use intel_tex_2::bc7;
use intel_tex_2::bc3;
use std::fmt;
use std::io::Cursor;

#[pyclass]
pub enum CompressionFormat
{
    Rgba8,
    Rgba8Unorm,
    Dxt5,
    Bc7
}

struct MyDxgiFormat(DxgiFormat);

impl MyDxgiFormat {
    fn to_string(&self) -> String {
        match self.0 {
            DxgiFormat::BC7_Typeless => "BC7_Typeless".to_string(),
            DxgiFormat::BC7_UNorm => "BC7_UNorm".to_string(),
            DxgiFormat::BC7_UNorm_sRGB => "BC7_UNorm_sRGB".to_string(),
            DxgiFormat::BC3_Typeless => "BC3_Typeless".to_string(),
            DxgiFormat::BC3_UNorm_sRGB => "BC3_UNorm_sRGB".to_string(),
            _ => "Unknown format".to_string()
        }
    }
}

impl fmt::Display for CompressionFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompressionFormat::Rgba8 => write!(f, "RGBA8"),
            CompressionFormat::Dxt5 => write!(f, "DXT5"),
            CompressionFormat::Bc7 => write!(f, "BC7"),
            CompressionFormat::Rgba8Unorm => write!(f, "RGBA8_UNORM")
        }
    }
}

fn compresion_format_to_dxgi_format(compression_format:&CompressionFormat) -> DxgiFormat
{
    return match compression_format {
        CompressionFormat::Bc7 => DxgiFormat::BC7_UNorm_sRGB,
        CompressionFormat::Rgba8 => DxgiFormat::R8G8B8A8_UInt,
        CompressionFormat::Rgba8Unorm => DxgiFormat::R8G8B8A8_UNorm,
        CompressionFormat::Dxt5 => DxgiFormat::BC3_UNorm_sRGB,
    }
}

fn convert_image_as_raw_rgba8(width: u32, height: u32, rgba8_content: &[u8]) -> (u32, u32, Vec<u8>, CompressionFormat)
{
    return (width, height, rgba8_content.to_vec(), CompressionFormat::Rgba8);
}

type SurfaceHandler = fn(&intel_tex_2::RgbaSurface, &mut [u8]);

fn convert_image_as(width: u32, height: u32, rgba8_content: &[u8], format: DxgiFormat, surface_handler: SurfaceHandler) -> (u32, u32, Vec<u8>, CompressionFormat)
{
    let block_count = intel_tex_2::divide_up_by_multiple(width * height, 16);
    println!("Block count: {}", block_count);
    println!("width {} - height {}", width, height);
    let dds_defaults = NewDxgiParams {
        height,
        width,
        depth: Some(1),
        format: format,
        mipmap_levels: Some(1),
        array_layers: Some(1),
        caps2: Some(Caps2::empty()),
        is_cubemap: false,
        resource_dimension: D3D10ResourceDimension::Texture2D,
        alpha_mode: AlphaMode::Straight,
    };
    // BC7
    let mut dds = Dds::new_dxgi(NewDxgiParams {
        format: format,
        ..dds_defaults
    })
    .unwrap();
    let surface = intel_tex_2::RgbaSurface {
        width,
        height,
        stride: width * 4,
        data: rgba8_content,
    };
    println!("Compressing to {}...", MyDxgiFormat(format).to_string());
    surface_handler(&surface, dds.get_mut_data(0 /* layer */).unwrap());
    println!("  Done!");

    //dds.write(&mut OpenOptions::new().write(true).create(true).open("a.dds").unwrap());
    let dds_data = dds.get_data(0).unwrap();
    return (width, height, dds_data.to_vec(), CompressionFormat::Bc7);
}

fn align_on(pow2_value: u32, val: u32) -> u32
{
    let mask: u32 = pow2_value - 1;
    return (val+mask) & (!mask);
}

fn surface_treatment_none(_surface: &intel_tex_2::RgbaSurface, _blocks: &mut [u8])
{

}

fn surface_treatment_dxt5(surface: &intel_tex_2::RgbaSurface, blocks: &mut [u8])
{
    println!("BC3 Compression...");
    bc3::compress_blocks_into(surface, blocks);
    println!("Compression Done !");
}

fn surface_treatment_bc7(surface: &intel_tex_2::RgbaSurface, blocks: &mut [u8])
{
    println!("BC7 Compression...");
    bc7::compress_blocks_into(
        &bc7::alpha_ultra_fast_settings(),
        &surface,
        blocks // dds.get_mut_data(0 /* layer */).unwrap(),
    );
    println!("Compression Done !");
}

fn surface_handler_for(compression_format:&CompressionFormat) -> SurfaceHandler
{
    return match compression_format {
        CompressionFormat::Bc7 => surface_treatment_bc7,
        CompressionFormat::Rgba8 => surface_treatment_none,
        CompressionFormat::Rgba8Unorm => surface_treatment_none,
        CompressionFormat::Dxt5 => surface_treatment_dxt5,
    }
}

fn compression_format_from_name(name:&str) -> CompressionFormat
{
    
    return match name.to_lowercase().as_str().into() {
        "bc7" => CompressionFormat::Bc7,
        "rgba8" => CompressionFormat::Rgba8,
        "rgba8_unorm" => CompressionFormat::Rgba8Unorm,
        "dxt5" => CompressionFormat::Dxt5,
        _ => CompressionFormat::Bc7
    };
}

/**
 * Convert the image provided to DDS, compressed using the BC7 algorithm
 * @param buffer The buffer containing the image data to convert
 * @returns (width, height, buffer_with_dds_data)
 * @note The returned buffer has no header
 */
#[pyfunction]
pub fn convert_image_content_in(buffer: &[u8], preferred_compression_format:&str) -> PyResult<(u32, u32, Vec<u8>, CompressionFormat)>
{
    let img = image::ImageReader::new(Cursor::new(buffer)).with_guessed_format().unwrap().decode().unwrap();
    
    let width = img.width();
    let height = img.height();

    if (width * height) < (256*256)
    {
        let rgba8_image = img.to_rgba8();
        let rgba8_content = &rgba8_image.into_raw()[..];
        return Ok(convert_image_as_raw_rgba8(width, height, rgba8_content));
    }
    else
    {
        let compression_format = compression_format_from_name(preferred_compression_format);
        let dxgi_format = compresion_format_to_dxgi_format(&compression_format);
        let surface_handler = surface_handler_for(&compression_format);

        let used_width = align_on(4, width);
        let used_height  = align_on(4, height);
        let resize_filter:image::imageops::FilterType = image::imageops::FilterType::Lanczos3;

        let needs_resize :bool = (used_width != width) | (used_height != height);
        let used_img: image::DynamicImage = if needs_resize { img.resize_exact(used_width, used_height, resize_filter) } else { img };

        let rgba8_image = used_img.to_rgba8();
        let rgba8_content = &rgba8_image.into_raw()[..];
        return Ok(convert_image_as(used_width, used_height, rgba8_content, dxgi_format, surface_handler));
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn voyage_texture_converter(python_module: &Bound<'_, PyModule>) -> PyResult<()> {
    python_module.add_function(wrap_pyfunction!(convert_image_content_in, python_module)?)?;
    Ok(())
}
