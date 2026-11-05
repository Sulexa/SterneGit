use std::io::Cursor;

use anyhow::{Context as _, Result, bail};
use image::{
    ColorType, DynamicImage, ImageFormat,
    codecs::{
        bmp::BmpDecoder, gif::GifDecoder, ico::IcoDecoder, jpeg::JpegDecoder, png::PngDecoder,
        qoi::QoiDecoder, tga::TgaDecoder, webp::WebPDecoder,
    },
};

pub(crate) struct Picture {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<u8>,
    pub(crate) format: &'static str,
    pub(crate) color: &'static str,
    pub(crate) bytes: usize,
    pub(crate) lossy: bool,
}

impl Picture {
    pub(crate) fn decode(bytes: &[u8], extension: Option<&str>) -> Result<Self> {
        let format = image::guess_format(bytes)
            .ok()
            .or_else(|| extension.and_then(ImageFormat::from_extension))
            .context("FastGit can't tell what kind of picture this is")?;
        let decoded = decoded(bytes, format)
            .with_context(|| format!("Couldn't read this {} picture", format_name(format)))?;
        let color = color_name(decoded.color());
        let rgba = decoded.to_rgba8();
        Ok(Self {
            width: rgba.width(),
            height: rgba.height(),
            pixels: rgba.into_raw(),
            format: format_name(format),
            color,
            bytes: bytes.len(),
            lossy: matches!(format, ImageFormat::Jpeg | ImageFormat::WebP),
        })
    }

    pub(crate) fn from_frame(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
            format: "Frame",
            color: "RGBA, 8 bits",
            bytes: 0,
            lossy: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        Self {
            width,
            height,
            pixels,
            format: "PNG",
            color: "RGBA, 8 bits",
            bytes: 0,
            lossy: false,
        }
    }

    pub(crate) fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[at],
            self.pixels[at + 1],
            self.pixels[at + 2],
            self.pixels[at + 3],
        ]
    }

    pub(crate) fn size(&self) -> String {
        format!("{} × {} px", self.width, self.height)
    }
}

fn decoded(bytes: &[u8], format: ImageFormat) -> Result<DynamicImage> {
    let reader = Cursor::new(bytes);
    Ok(match format {
        ImageFormat::Png => DynamicImage::from_decoder(PngDecoder::new(reader)?)?,
        ImageFormat::Jpeg => DynamicImage::from_decoder(JpegDecoder::new(reader)?)?,
        ImageFormat::Gif => DynamicImage::from_decoder(GifDecoder::new(reader)?)?,
        ImageFormat::WebP => DynamicImage::from_decoder(WebPDecoder::new(reader)?)?,
        ImageFormat::Bmp => DynamicImage::from_decoder(BmpDecoder::new(reader)?)?,
        ImageFormat::Tga => DynamicImage::from_decoder(TgaDecoder::new(reader)?)?,
        ImageFormat::Ico => DynamicImage::from_decoder(IcoDecoder::new(reader)?)?,
        ImageFormat::Qoi => DynamicImage::from_decoder(QoiDecoder::new(reader)?)?,
        _ => bail!("FastGit doesn't draw {} pictures", format_name(format)),
    })
}

fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Gif => "GIF",
        ImageFormat::WebP => "WebP",
        ImageFormat::Bmp => "BMP",
        ImageFormat::Ico => "ICO",
        ImageFormat::Tga => "TGA",
        ImageFormat::Tiff => "TIFF",
        ImageFormat::OpenExr => "EXR",
        ImageFormat::Qoi => "QOI",
        ImageFormat::Pnm => "PNM",
        ImageFormat::Hdr => "HDR",
        ImageFormat::Dds => "DDS",
        ImageFormat::Farbfeld => "Farbfeld",
        _ => "Picture",
    }
}

fn color_name(color: ColorType) -> &'static str {
    match color {
        ColorType::L8 => "Grayscale, 8 bits",
        ColorType::La8 => "Grayscale with alpha, 8 bits",
        ColorType::Rgb8 => "RGB, 8 bits",
        ColorType::Rgba8 => "RGBA, 8 bits",
        ColorType::L16 => "Grayscale, 16 bits",
        ColorType::La16 => "Grayscale with alpha, 16 bits",
        ColorType::Rgb16 => "RGB, 16 bits",
        ColorType::Rgba16 => "RGBA, 16 bits",
        ColorType::Rgb32F => "RGB, 32-bit float",
        ColorType::Rgba32F => "RGBA, 32-bit float",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, RgbaImage};

    use super::Picture;

    #[test]
    fn a_png_decodes_into_its_size_format_and_pixels() {
        let mut png = Vec::new();
        RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128])
            .unwrap()
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        let picture = Picture::decode(&png, Some("png")).unwrap();
        assert_eq!((picture.width, picture.height), (2, 1));
        assert_eq!(picture.format, "PNG");
        assert_eq!(picture.color, "RGBA, 8 bits");
        assert_eq!(picture.pixel(1, 0), [0, 0, 255, 128]);
        assert_eq!(picture.bytes, png.len());
    }

    #[test]
    fn bytes_that_are_no_picture_say_so() {
        assert!(Picture::decode(b"not a picture", Some("txt")).is_err());
    }
}
