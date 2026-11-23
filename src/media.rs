//! Lightweight image header dimension parser (pure Rust, zero dependencies).
//!
//! Reads at most the first 64 KiB of an image file to extract pixel dimensions
//! without decoding pixel buffers. Activated only when `--media-meta` is enabled.

use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_HEADER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn pixels(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    pub fn is_ultrawide(self) -> bool {
        self.width >= 2560
            && self.height >= 1000
            && u64::from(self.width) * 100 >= u64::from(self.height) * 205
    }

    pub fn is_portrait(self) -> bool {
        self.height > self.width
    }

    pub fn is_4k_or_higher(self) -> bool {
        self.width >= 3840 || self.height >= 2160
    }
}

pub fn probe_dimensions(path: &Path, ext: &str) -> Option<Dimensions> {
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; MAX_HEADER_BYTES];
    let n = file.read(&mut buf).ok()?;
    buf.truncate(n);
    parse_dimensions(&buf, ext)
}

pub fn parse_dimensions(data: &[u8], ext: &str) -> Option<Dimensions> {
    let dims = match ext {
        "png" => parse_png(data),
        "gif" => parse_gif(data),
        "bmp" => parse_bmp(data),
        "webp" => parse_webp(data),
        "jpg" | "jpeg" => parse_jpeg(data),
        _ => parse_png(data)
            .or_else(|| parse_jpeg(data))
            .or_else(|| parse_webp(data))
            .or_else(|| parse_gif(data))
            .or_else(|| parse_bmp(data)),
    }?;
    if dims.width == 0 || dims.height == 0 {
        None
    } else {
        Some(dims)
    }
}

fn parse_png(data: &[u8]) -> Option<Dimensions> {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if data.len() < 24 || &data[0..8] != SIG || &data[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some(Dimensions { width, height })
}

fn parse_gif(data: &[u8]) -> Option<Dimensions> {
    if data.len() < 10 || (&data[0..6] != b"GIF87a" && &data[0..6] != b"GIF89a") {
        return None;
    }
    let width = u32::from(u16::from_le_bytes([data[6], data[7]]));
    let height = u32::from(u16::from_le_bytes([data[8], data[9]]));
    Some(Dimensions { width, height })
}

fn parse_bmp(data: &[u8]) -> Option<Dimensions> {
    if data.len() < 26 || &data[0..2] != b"BM" {
        return None;
    }
    let dib_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);
    if dib_size == 12 {
        let width = u32::from(u16::from_le_bytes([data[18], data[19]]));
        let height = u32::from(u16::from_le_bytes([data[20], data[21]]));
        Some(Dimensions { width, height })
    } else if dib_size >= 40 {
        let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
        let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).saturating_abs();
        if width <= 0 || height <= 0 {
            return None;
        }
        Some(Dimensions {
            width: width as u32,
            height: height as u32,
        })
    } else {
        None
    }
}

fn parse_webp(data: &[u8]) -> Option<Dimensions> {
    if data.len() < 25 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return None;
    }
    match &data[12..16] {
        b"VP8 " => {
            if data.len() < 30 || data[23..26] != [0x9d, 0x01, 0x2a] {
                return None;
            }
            let width = u32::from(u16::from_le_bytes([data[26], data[27]]) & 0x3fff);
            let height = u32::from(u16::from_le_bytes([data[28], data[29]]) & 0x3fff);
            Some(Dimensions { width, height })
        }
        b"VP8L" => {
            if data.len() < 25 || data[20] != 0x2f {
                return None;
            }
            let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
            let width = (bits & 0x3fff) + 1;
            let height = ((bits >> 14) & 0x3fff) + 1;
            Some(Dimensions { width, height })
        }
        b"VP8X" => {
            if data.len() < 30 {
                return None;
            }
            let width = (u32::from(data[24])
                | (u32::from(data[25]) << 8)
                | (u32::from(data[26]) << 16))
                + 1;
            let height = (u32::from(data[27])
                | (u32::from(data[28]) << 8)
                | (u32::from(data[29]) << 16))
                + 1;
            Some(Dimensions { width, height })
        }
        _ => None,
    }
}

fn parse_jpeg(data: &[u8]) -> Option<Dimensions> {
    if data.len() < 4 || data[0] != 0xff || data[1] != 0xd8 {
        return None;
    }
    let mut pos = 2usize;
    while pos + 1 < data.len() {
        while pos < data.len() && data[pos] != 0xff {
            pos += 1;
        }
        while pos < data.len() && data[pos] == 0xff {
            pos += 1;
        }
        if pos >= data.len() {
            break;
        }
        let marker = data[pos];
        pos += 1;

        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x00 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if pos + 2 > data.len() {
            break;
        }
        let len = usize::from(u16::from_be_bytes([data[pos], data[pos + 1]]));
        if len < 2 || pos + len > data.len() {
            break;
        }
        let is_sof = matches!(
            marker,
            0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf
        );
        if is_sof && len >= 7 {
            let height = u32::from(u16::from_be_bytes([data[pos + 3], data[pos + 4]]));
            let width = u32::from(u16::from_be_bytes([data[pos + 5], data[pos + 6]]));
            return Some(Dimensions { width, height });
        }
        pos += len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_png_dimensions() {
        let mut data = Vec::new();
        data.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        data.extend_from_slice(&13u32.to_be_bytes());
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&3840u32.to_be_bytes());
        data.extend_from_slice(&2160u32.to_be_bytes());
        let dims = parse_dimensions(&data, "png").unwrap();
        assert_eq!(dims, Dimensions { width: 3840, height: 2160 });
        assert!(dims.is_4k_or_higher());
        assert!(!dims.is_portrait());
    }

    #[test]
    fn parses_gif_dimensions() {
        let mut data = b"GIF89a".to_vec();
        data.extend_from_slice(&1920u16.to_le_bytes());
        data.extend_from_slice(&1080u16.to_le_bytes());
        assert_eq!(
            parse_dimensions(&data, "gif"),
            Some(Dimensions { width: 1920, height: 1080 })
        );
    }

    #[test]
    fn parses_webp_variants() {
        let mut vp8x = Vec::new();
        vp8x.extend_from_slice(b"RIFF");
        vp8x.extend_from_slice(&0u32.to_le_bytes());
        vp8x.extend_from_slice(b"WEBP");
        vp8x.extend_from_slice(b"VP8X");
        vp8x.extend_from_slice(&10u32.to_le_bytes());
        vp8x.extend_from_slice(&[0u8; 4]);
        let w = 3440u32 - 1;
        let h = 1440u32 - 1;
        vp8x.extend_from_slice(&[w as u8, (w >> 8) as u8, (w >> 16) as u8]);
        vp8x.extend_from_slice(&[h as u8, (h >> 8) as u8, (h >> 16) as u8]);
        let dims = parse_dimensions(&vp8x, "webp").unwrap();
        assert_eq!(dims, Dimensions { width: 3440, height: 1440 });
        assert!(dims.is_ultrawide());

        let mut vp8l = Vec::new();
        vp8l.extend_from_slice(b"RIFF\x00\x00\x00\x00WEBPVP8L");
        vp8l.extend_from_slice(&5u32.to_le_bytes());
        vp8l.push(0x2f);
        let bits: u32 = (1080 - 1) | ((1920 - 1) << 14);
        vp8l.extend_from_slice(&bits.to_le_bytes());
        assert_eq!(
            parse_dimensions(&vp8l, "webp"),
            Some(Dimensions { width: 1080, height: 1920 })
        );
    }

    #[test]
    fn parses_jpeg_sof0() {
        let mut jpg = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00];
        jpg.extend_from_slice(&[0xff, 0xc0, 0x00, 0x0b, 0x08]);
        jpg.extend_from_slice(&4512u16.to_be_bytes());
        jpg.extend_from_slice(&6106u16.to_be_bytes());
        jpg.extend_from_slice(&[0x03, 0x01, 0x11, 0x00]);
        assert_eq!(
            parse_dimensions(&jpg, "jpg"),
            Some(Dimensions { width: 6106, height: 4512 })
        );
    }
}
