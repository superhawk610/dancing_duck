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

    let mut byte_sets = ByteSet::new();
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

fn generate_header(byte_sets: ByteSet) -> String {
    let bytes = byte_sets
        .into_iter()
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

/// A ByteSet contains a variable number of Bytes of variable length.
/// They are laid out in memory as follows:
///
///     const uint8_t frames[] = {
///       /* w: u8 */ 60, /* h: u8 */ 64,
///       /* bytes: u8[] */ 0b01100011,
///       // (repeat 0 or more times)...
///     };
///
struct ByteSet(Vec<Bytes>);

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
}

impl std::iter::IntoIterator for Bytes {
    type Item = u8;
    type IntoIter = <Vec<u8> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ByteSet {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, bytes: Bytes) {
        self.0.push(bytes);
    }
}

impl std::iter::IntoIterator for ByteSet {
    type Item = u8;
    type IntoIter = std::iter::Flatten<<Vec<Bytes> as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().flatten()
    }
}
