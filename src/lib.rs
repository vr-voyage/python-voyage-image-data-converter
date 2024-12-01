use pyo3::prelude::*;

use ddsfile::{AlphaMode, Caps2, D3D10ResourceDimension, Dds, DxgiFormat, NewDxgiParams};
use image::ImageReader;
use std::io::Cursor;
use intel_tex_2::bc7;
use intel_tex_2::bc3;

fn convert_to_bc7(buffer: &[u8]) -> (u32, u32, Vec<u8>)
{
    let img = ImageReader::new(Cursor::new(buffer)).with_guessed_format().unwrap().decode().unwrap();
    let rgba8_image = img.to_rgba8();
    let width = rgba8_image.width();
    let height = rgba8_image.height();

    let rgba8_content = &rgba8_image.into_raw()[..];
    /*for b in 0..rgba8_content.len()
    {
        if (b & 7) == 0 { print!("\n"); }
        print!("0x{:x} ", rgba8_content[b]);
    }
    print!("\n");*/

    let block_count = intel_tex_2::divide_up_by_multiple(width * height, 16);
    println!("Block count: {}", block_count);
    println!("width {} - height {}", width, height);
    let dds_defaults = NewDxgiParams {
        height,
        width,
        depth: Some(1),
        format: DxgiFormat::BC7_UNorm,
        mipmap_levels: Some(1),
        array_layers: Some(1),
        caps2: Some(Caps2::empty()),
        is_cubemap: false,
        resource_dimension: D3D10ResourceDimension::Texture2D,
        alpha_mode: AlphaMode::Straight,
    };
    // BC7
    {
        let mut dds = Dds::new_dxgi(NewDxgiParams {
            format: DxgiFormat::BC7_UNorm,
            ..dds_defaults
        })
        .unwrap();
        let surface = intel_tex_2::RgbaSurface {
            width,
            height,
            stride: width * 4,
            data: rgba8_content,
        };

        println!("Compressing to BC7...");
        bc7::compress_blocks_into(
            &bc7::alpha_ultra_fast_settings(),
            &surface,
            dds.get_mut_data(0 /* layer */).unwrap(),
        );
        println!("  Done!");

        //dds.write(&mut OpenOptions::new().write(true).create(true).open("a.dds").unwrap());
        let dds_data = dds.get_data(0).unwrap();
        return (width, height, dds_data.to_vec());
    }
}

fn convert_to_dxt5(buffer: &[u8]) -> (u32, u32, Vec<u8>)
{
    let img = ImageReader::new(Cursor::new(buffer)).with_guessed_format().unwrap().decode().unwrap();
    let rgba8_image = img.to_rgba8();
    let width = rgba8_image.width();
    let height = rgba8_image.height();

    let rgba8_content = &rgba8_image.into_raw()[..];
    /*for b in 0..rgba8_content.len()
    {
        if (b & 7) == 0 { print!("\n"); }
        print!("0x{:x} ", rgba8_content[b]);
    }
    print!("\n");*/

    let block_count = intel_tex_2::divide_up_by_multiple(width * height, 16);
    println!("Block count: {}", block_count);
    println!("width {} - height {}", width, height);
    let dds_defaults = NewDxgiParams {
        height,
        width,
        depth: Some(1),
        format: DxgiFormat::BC3_Typeless,
        mipmap_levels: Some(1),
        array_layers: Some(1),
        caps2: Some(Caps2::empty()),
        is_cubemap: false,
        resource_dimension: D3D10ResourceDimension::Texture2D,
        alpha_mode: AlphaMode::Straight,
    };
    // BC7
    {
        let mut dds = Dds::new_dxgi(NewDxgiParams {
            format: DxgiFormat::BC3_Typeless,
            ..dds_defaults
        })
        .unwrap();
        let surface = intel_tex_2::RgbaSurface {
            width,
            height,
            stride: width * 4,
            data: rgba8_content,
        };

        println!("Compressing to DXT5...");
        bc3::compress_blocks_into(
            &surface,
            dds.get_mut_data(0 /* layer */).unwrap(),
        );
        println!("  Done!");

        //dds.write(&mut OpenOptions::new().write(true).create(true).open("a.dds").unwrap());
        let dds_data = dds.get_data(0).unwrap();
        return (width, height, dds_data.to_vec());
    }
}

/// Formats the sum of two numbers as string.
/**
 * Convert the image provided to DDS, compressed using the BC7 algorithm
 * @param buffer The buffer containing the image data to convert
 * @returns (width, height, buffer_with_dds_data)
 * @note The returned buffer has no header
 */
#[pyfunction]
fn convert_image_content_in(buffer: &[u8], dxt: bool) -> PyResult<(u32, u32, Vec<u8>)>
{
    if dxt { return Ok(convert_to_dxt5(buffer)); }
    return Ok(convert_to_bc7(buffer));
}

/// A Python module implemented in Rust.
#[pymodule]
fn voyage_texture_converter(python_module: &Bound<'_, PyModule>) -> PyResult<()> {
    python_module.add_function(wrap_pyfunction!(convert_image_content_in, python_module)?)?;
    Ok(())
}
