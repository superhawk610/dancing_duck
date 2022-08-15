use glob::glob;
use image::io::Reader as ImageReader;
use itertools::Itertools;
use std::env;
use std::fs;
use std::path::Path;

/// Convert a .bmp image to an Arduino .h file that contains
/// a byte array representing the black/white pixels in the
/// image in PROGMEM that can be used with displayBitmap()
/// from the Adafruit GFX library.

const IMAGE_PATH: &str = "C:\\\\Users\\Aaron Ross\\Downloads\\downscale\\out-*.bmp";

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let output = args.get(1).expect("output file must be provided as arg");

    let mut byte_sets = Vec::new();
    for file in glob(IMAGE_PATH)
        .expect("failed to find input files")
        .take(30)
    {
        let file = file.expect("failed to read input file");
        let bytes = Bytes::try_from_image(file).unwrap();
        byte_sets.push(bytes);
    }

    let header = generate_header(byte_sets);
    fs::write(output, header).unwrap();
}

// TODO: compress runs of zeroes
// TODO: reduce width/height to minimum possible
fn generate_header(byte_sets: Vec<Bytes>) -> String {
    let bytes = byte_sets
        .into_iter()
        .flatten()
        // .map(|x| format!("{x:#010b}"))
        .map(|x| format!("{x:#04x}"))
        .chunks(16)
        .into_iter()
        .map(|mut chunk| chunk.join(", "))
        .join(",\n  ");
    let bytes = bytes.trim_end();
    format!("const uint8_t PROGMEM image_frames[] =\n{{ {bytes} }};")
}

/// Bytes encodes the width, height, and pixel data for a black
/// and white image. The first byte represents the width, the
/// second byte represents the height, and all remaining bytes
/// represent the pixels (where a 1 is a white pixel and a 0 is
/// a black pixel).
///
/// This data may be displayed with the Adafruit GFX library using
/// something like the following:
///
///     // draw into the top left of the screen
///     display.drawBitmap(
///       /* x */ 0, /* y */ 0, /* bmp */ image_bytes + 2,
///       /* w */ image_bytes[0], /* h */ image_bytes[1],
///       /* color */ 1
///     );
///     display.display()
///
///     // draw into the center of the screen
///     uint8_t w = image_bytes[0];
///     uint8_t h = image_bytes[1];
///     display.drawBitmap(
///       (display.width() - w) / 2,
///       (display.height() - h) / 2,
///       image_bytes + 2, w, h, 1
///     );
///     display.display();
///
struct Bytes(Vec<u8>);

impl Bytes {
    fn try_from_image<P>(path: P) -> Result<Self, String>
    where
        P: AsRef<Path>,
    {
        let img = ImageReader::open(path)
            .map_err(|err| format!("unable to open image: {}", err))?
            .decode()
            .map_err(|err| format!("failed to decode image: {}", err))?
            .into_luma8();

        let w: u8 = img.width().try_into().map_err(|_| "image is too wide")?;
        let h: u8 = img.height().try_into().map_err(|_| "image is too tall")?;
        let mut inner = Vec::with_capacity(2 + w as usize * h as usize);
        inner.push(w);
        inner.push(h);

        // iterate over x,y coords instead of pixels so that rows will be
        // right-padded with 0's to full bytes
        for y in 0..h {
            for chunk in &(0..w).chunks(8) {
                let mut byte: u8 = 0;
                for (i, x) in chunk.enumerate() {
                    if x >= w {
                        break;
                    }

                    let luma = img.get_pixel(x as _, y as _).0[0];
                    let pixel = if luma > 0 { 1 } else { 0 };
                    byte = byte | (pixel << (7 - i));
                }
                inner.push(byte);
            }
        }

        // this method won't work since for images that aren't perfect power-of-8
        // dimensions, since pixels from the next row end up stuck on the end of
        // the previous row
        //
        // for chunk in &img.pixels().into_iter().chunks(8) {
        //     let mut byte: u8 = 0;
        //     for (i, luma) in chunk.enumerate() {
        //         let pixel = if luma.0[0] > 0 { 1 } else { 0 };
        //         byte = byte | (pixel << (7 - i));
        //     }
        //     inner.push(byte);
        // }

        Ok(Self(inner))
    }

    /// Return an iterator over bits (pixels) contained in the bytes.
    /// Row padding is included, so the iterator will always output
    /// a multiple of 8 values.
    fn bits(&self) -> impl Iterator<Item = u8> {
        let mut bits = Vec::with_capacity((self.0.len() - 2) * 8);

        // skip the width and height
        for byte in self.0.iter().skip(2) {
            for bit_index in 0..8 {
                bits.push(if byte & (1 << bit_index) > 0 { 1 } else { 0 });
            }
        }

        bits.into_iter()
    }
}

impl std::iter::IntoIterator for Bytes {
    type Item = u8;
    type IntoIter = <Vec<u8> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        // Sadly, smaller Arduino boards are too memory-constrained to
        // decompress images into SRAM at runtime, so any sufficiently-
        // complex (read: larger than 24x24 or so) images must be stored
        // in flash memory at program burn time.
        return self.0.into_iter();

        let uncompressed_size = self.0.len();

        let mut bytes = vec![self.0[0], self.0[1]];
        if self.0.len() == 2 {
            // length is 0u32 (read more below)
            bytes.extend(vec![0, 0, 0, 0]);
            let compressed_size = bytes.len();
            eprintln!("uncompressed: {uncompressed_size} bytes");
            eprintln!("compressed: {compressed_size} bytes");
            eprintln!(
                "size reduction: {}%",
                (uncompressed_size - compressed_size) as f32 / uncompressed_size as f32 * 100.0
            );
            return bytes.into_iter();
        }

        // We're going to use run-length encoding to compress consecutive
        // pixels of the same value; run length will be stored in the low
        // 7 bits of the byte, and the high bit will determine the value;
        // since 0-length runs can't exist, the low 7 bits represent the
        // values [1, 128] (to calculate, add 1 to their normal value)
        //
        //     1 0 0 0 1 0 0 1
        //     | |-|-|-|-|-|-|- 9 in binary, so length is 10
        //     |- run of 1s
        //
        // There may be a more efficient option where runs always alternate,
        // which would allow us to use the high bit and thus store runs up
        // to 256 per byte. However, this would require some sort of special
        // marker value for runs > 256, which isn't required with the above
        // layout, so we'll put that off for now for the sake of simplicity.
        let mut runs = Vec::new();
        let mut bits = self.bits();
        let mut bit_val = bits.next().unwrap();
        let mut run_len = 1;
        let max_len = 128;
        for bit in bits {
            if bit == bit_val {
                run_len += 1;
                if run_len == max_len {
                    runs.push((bit_val << 7) | (run_len - 1));
                    run_len = 0;
                }
            } else {
                runs.push((bit_val << 7) | (run_len - 1));
                bit_val = bit;
                run_len = 1;
            }
        }

        // Since the number of bytes will no longer be directly related to
        // the image dimensions, we need to output the actual length. This
        // is stored as a u32, split over 4 u8 values.
        let len = runs.len();
        for byte_index in (0..4).into_iter().rev() {
            bytes.push((len >> (byte_index * 8) & 0xff) as u8);
        }

        bytes.extend(runs);
        let compressed_size = bytes.len();
        eprintln!("uncompressed: {uncompressed_size} bytes");
        eprintln!("compressed: {compressed_size} bytes");
        eprintln!(
            "size reduction: {}%",
            (uncompressed_size - compressed_size) as f32 / uncompressed_size as f32 * 100.0
        );
        bytes.into_iter()
    }
}
