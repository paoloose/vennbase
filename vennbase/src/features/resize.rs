use std::io::{BufWriter, IntoInnerError};
use std::{num::NonZeroU32, io::Cursor};

use fast_image_resize as fr;
use fr::{ImageBufferError, MulDivImageError, DifferentTypesOfPixelsError};
use image::error::{UnsupportedError, ImageFormatHint, UnsupportedErrorKind};
use image::{ColorType, ImageEncoder, ImageResult};
use image::{
    io::Reader as ImageReader,
    ImageError,
    ImageFormat
};
use image::codecs::{
    png::PngEncoder,
    jpeg::JpegEncoder,
    bmp::BmpEncoder,
    webp::WebPEncoder
};

use crate::db::types::MimeType;

#[derive(Debug)]
enum Resize {
    Auto,
    Dimension(NonZeroU32),
}

#[derive(Debug)]
pub struct Dimensions(Resize, Resize);

#[derive(Debug)]
pub struct DimensionParsingError;

impl Dimensions {
    pub fn from_dim_str(resize_str: &str) -> Result<Self, DimensionParsingError> {
        let (width, height) = resize_str.split_once('x').ok_or(DimensionParsingError)?;
        let width = match width {
            "auto" => Resize::Auto,
            num => {
                let num = num.parse::<u32>().map(NonZeroU32::new)
                    .map_err(|_| DimensionParsingError)?
                    .ok_or(DimensionParsingError)?;
                Resize::Dimension(num)
            },
        };
        let height = match height {
            "auto" => Resize::Auto,
            num => {
                let num = num.parse::<u32>().map(NonZeroU32::new)
                    .map_err(|_| DimensionParsingError)?
                    .ok_or(DimensionParsingError)?;
                Resize::Dimension(num)
            },
        };

        Ok(Dimensions(width, height))
    }
}

#[derive(Debug)]
pub enum ResizeError {
    BufferError(ImageBufferError),
    IoError(std::io::Error),
    ImageError(ImageError),
    MulDivImageError(MulDivImageError),
    DifferentTypesOfPixelsError(DifferentTypesOfPixelsError),
    BufferFlushError(IntoInnerError<BufWriter<Vec<u8>>>)
}

fn encode_image_with_format(image_buffer: &[u8], dims: (u32, u32), format: ImageFormat) -> ImageResult<BufWriter<Vec<u8>>> {
    let mut result_buf = BufWriter::new(Vec::new());

    match format {
        ImageFormat::Png => {
            // Write destination image as PNG-file
            PngEncoder::new(&mut result_buf)
                .write_image(
                    image_buffer,
                    dims.0,
                    dims.1,
                    ColorType::Rgba8,
                )?;
            Ok(result_buf)
        },
        ImageFormat::Jpeg => {
            JpegEncoder::new(&mut result_buf)
                .write_image(
                    image_buffer,
                    dims.0,
                    dims.1,
                    ColorType::Rgba8,
                )?;
            Ok(result_buf)
        },
        ImageFormat::WebP => {
            WebPEncoder::new(&mut result_buf)
                .write_image(
                    image_buffer,
                    dims.0,
                    dims.1,
                    ColorType::Rgba8,
                )?;
            Ok(result_buf)
        },
        ImageFormat::Bmp => {
            BmpEncoder::new(&mut result_buf)
                .write_image(
                    image_buffer,
                    dims.0,
                    dims.1,
                    ColorType::Rgba8,
                )?;
            Ok(result_buf)
        },
        _ => {
            Err(ImageError::Unsupported(
                UnsupportedError::from_format_and_kind(
                    ImageFormatHint::Unknown,
                    UnsupportedErrorKind::Format(ImageFormatHint::Unknown)
                )
            ))
        },
    }
}

pub fn is_resizeable_format(mimetype: &MimeType) -> bool {
    match ImageFormat::from_mime_type(mimetype.as_str()) {
        Some(format) => matches!(format,
            ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Bmp
        ),
        None => false,
    }
}


/// Interpretes a well-formed set of bytes, guessing its  image by using the fast_image_resize crate
///
/// # Panics
/// If any of the given size is zero
///
pub fn resize_image(data: &Vec<u8>, format: ImageFormat, new_dims: &Dimensions) -> Result<Vec<u8>, ResizeError> {
    // Read source image from file
    let img = ImageReader::with_format(Cursor::new(data), format).decode()?;
    let (width, height) = (
        NonZeroU32::new(img.width()).expect("To be positive"),
        NonZeroU32::new(img.height()).expect("To be positive")
    );

    let mut src_image = fr::Image::from_vec_u8(
        width,
        height,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )?;

    // Multiple RGB channels of source image by alpha channel
    // (not required for the Nearest algorithm)
    let alpha_mul_div = fr::MulDiv::default();
    alpha_mul_div
        .multiply_alpha_inplace(&mut src_image.view_mut())
        .map_err(ResizeError::MulDivImageError)?;

    let aspect_ratio = img.width() as f32 / img.height() as f32;
    // Create container for data of destination image
    let (dst_width, dst_height) = match new_dims.0 {
        Resize::Auto => {
            match new_dims.1 {
                Resize::Auto => (width, height), // 'autoxauto' has no effect
                Resize::Dimension(h) => {
                    let w = NonZeroU32::new((h.get() as f32 * aspect_ratio) as u32).unwrap_or(
                        NonZeroU32::new(1).unwrap()
                    );
                    (w, h)
                },
            }
        },
        Resize::Dimension(h) => {
            let w = match new_dims.1 {
                Resize::Auto => NonZeroU32::new((h.get() as f32 * aspect_ratio) as u32).unwrap_or(
                    NonZeroU32::new(1).unwrap()
                ),
                Resize::Dimension(h2) => h2,
            };
            (w, h)
        },
    };
    let mut dst_image = fr::Image::new(
        dst_width,
        dst_height,
        src_image.pixel_type(),
    );

    // Get mutable view of destination image data
    let mut dst_view = dst_image.view_mut();

    // Create Resizer instance and resize source image
    // into buffer of destination image
    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    resizer.resize(&src_image.view(), &mut dst_view)
        .map_err(ResizeError::DifferentTypesOfPixelsError)?;

    // Divide RGB channels of destination image by alpha
    alpha_mul_div.divide_alpha_inplace(&mut dst_view)
        .map_err(ResizeError::MulDivImageError)?;

    match encode_image_with_format(dst_image.buffer(), (dst_width.into(), dst_height.into()),format) {
        Ok(img_buffer) => Ok(
            img_buffer.into_inner().map_err(ResizeError::BufferFlushError)?
        ),
        Err(e) => Err(ResizeError::ImageError(e)),
    }
}

impl From<ImageBufferError> for ResizeError {
    fn from(err: ImageBufferError) -> Self {
        ResizeError::BufferError(err)
    }
}

impl From<std::io::Error> for ResizeError {
    fn from(err: std::io::Error) -> Self {
        ResizeError::IoError(err)
    }
}

impl From<ImageError> for ResizeError {
    fn from(err: ImageError) -> Self {
        ResizeError::ImageError(err)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, prelude::*, BufReader};
    use std::fs::File;

    use super::*;

    fn parse_png_or_fail(path: &str) -> io::Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        let image = resize_image(
            &data,
            ImageFormat::Png,
            &Dimensions::from_dim_str("200xauto").unwrap()
        ).unwrap();

        assert!(image.len() > 0);
        Ok(())
    }

    #[test]
    fn png_being_parsed_correctly() -> io::Result<()> {
        parse_png_or_fail("../data/blossom.png")?;

        Ok(())
    }
}
