use std::io::BufWriter;
use std::{num::NonZeroU32, io::Cursor};

use fast_image_resize as fr;
use fr::ImageBufferError;
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
    Dimension(u32),
}

#[derive(Debug)]
pub struct Dimensions(Resize, Resize);

impl Dimensions {
    pub fn from_str(resize_str: &str) -> Result<Self, ()> {
        let (width, height) = resize_str.split_once('x').ok_or(())?;
        let width = match width {
            "auto" => Resize::Auto,
            num => {
                Resize::Dimension(num.parse::<u32>().map_err(|_| ())?)
            },
        };
        let height = match height {
            "auto" => Resize::Auto,
            num => {
                Resize::Dimension(num.parse::<u32>().map_err(|_| ())?)
            },
        };

        Ok(Dimensions(width, height))
    }
}

#[derive(Debug)]
pub enum ResizeError {
    BufferError(ImageBufferError),
    IoError(std::io::Error),
    ImageError(ImageError)
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
        Some(format) => match format {
            ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Bmp => true,
            _ => false,
        },
        None => false,
    }
}

/// Interpretes a well-formed set of bytes, guessing its  image by using the fast_image_resize crate
///
/// # Panics
/// If any of the given size is zero
///
pub fn resize_image<'a>(data: &Vec<u8>, format: ImageFormat, new_dims: &Dimensions) -> Result<Vec<u8>, ResizeError> {
    // Read source image from file
    let img = ImageReader::with_format(Cursor::new(data), format).decode()?;
    let (width, height) = (img.width(), img.height());

    let mut src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )?;

    // Multiple RGB channels of source image by alpha channel
    // (not required for the Nearest algorithm)
    let alpha_mul_div = fr::MulDiv::default();
    alpha_mul_div
        .multiply_alpha_inplace(&mut src_image.view_mut())
        .unwrap();

    let aspect_ratio = img.width() as f32 / img.height() as f32;
    // Create container for data of destination image
    let (dst_width, dst_height) = match new_dims.0 {
        Resize::Auto => {
            match new_dims.1 {
                Resize::Auto => {
                    (NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap())
                },
                Resize::Dimension(h) => {
                    let w = (h as f32 * aspect_ratio) as u32;
                    (NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
                },
            }
        },
        Resize::Dimension(h) => {
            let w = match new_dims.1 {
                Resize::Auto => (h as f32 * aspect_ratio) as u32,
                Resize::Dimension(h2) => h2,
            };
            (NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
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
    let mut resizer = fr::Resizer::new(
        fr::ResizeAlg::Convolution(fr::FilterType::Box),
    );
    resizer.resize(&src_image.view(), &mut dst_view).unwrap();

    // Divide RGB channels of destination image by alpha
    alpha_mul_div.divide_alpha_inplace(&mut dst_view).unwrap();

    match encode_image_with_format(dst_image.buffer(), (dst_width.into(), dst_height.into()),format) {
        Ok(img_buffer) => Ok(img_buffer.into_inner().unwrap()),
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
            &Dimensions::from_str("200xauto").unwrap()
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
