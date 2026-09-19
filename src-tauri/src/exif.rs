// EXIF device pool + injected-tag writer (spec §7.3 / §8 / EXIF-internals section).
use crate::models::{ExifConfig, ExifDevice};
use crate::prng::Prng;

pub fn device_list() -> Vec<ExifDevice> {
    let d = |id: &str, label: &str, os: &str, make: &str, model: &str, sw: Option<&str>| ExifDevice {
        id: id.into(), label: label.into(), os: os.into(), make: make.into(),
        model: model.into(), software: sw.map(|s| s.into()),
    };
    vec![
        d("ios_iphone_15_pro_max", "iPhone 15 Pro Max", "ios", "Apple", "iPhone 15 Pro Max", Some("17.5.1")),
        d("ios_iphone_15", "iPhone 15", "ios", "Apple", "iPhone 15", Some("17.4.1")),
        d("ios_iphone_14_pro", "iPhone 14 Pro", "ios", "Apple", "iPhone 14 Pro", Some("17.1")),
        d("ios_iphone_13_pro", "iPhone 13 Pro", "ios", "Apple", "iPhone 13 Pro", Some("16.6")),
        d("ios_iphone_13", "iPhone 13", "ios", "Apple", "iPhone 13", Some("16.3.1")),
        d("ios_iphone_12", "iPhone 12", "ios", "Apple", "iPhone 12", Some("15.7")),
        d("ios_iphone_11", "iPhone 11", "ios", "Apple", "iPhone 11", Some("15.1")),
        d("ios_iphone_se", "iPhone SE", "ios", "Apple", "iPhone SE", Some("16.5")),
        d("and_samsung_s24_ultra", "Samsung Galaxy S24 Ultra", "android", "samsung", "SM-S928B", Some("S928BXXU1AWxx")),
        d("and_samsung_s23", "Samsung Galaxy S23", "android", "samsung", "SM-S911B", Some("S911BXXU3BWxx")),
        d("and_samsung_s21", "Samsung Galaxy S21", "android", "samsung", "SM-G991B", Some("G991BXXU3")),
        d("and_pixel_8_pro", "Google Pixel 8 Pro", "android", "Google", "Pixel 8 Pro", Some("HDR+ 1.0.610268337")),
        d("and_pixel_7", "Google Pixel 7", "android", "Google", "Pixel 7", Some("HDR+ 1.0.540352485")),
        d("and_oneplus_11", "OnePlus 11", "android", "OnePlus", "CPH2449", Some("1.0")),
        d("and_xiaomi_13", "Xiaomi 13", "android", "Xiaomi", "2211133C", None),
    ]
}

// exif::random_datetime — year∈{2022,2023,2024,2025}, "YYYY:MM:DD HH:MM:SS"
pub fn random_datetime(prng: &mut Prng) -> String {
    let years = [2022, 2023, 2024, 2025];
    let year = years[(prng.next_u64() & 3) as usize];
    let month = (prng.next_u64() % 12) + 1;
    let day = (prng.next_u64() % 28) + 1;
    let hour = prng.next_u64() % 24;
    let minute = prng.next_u64() % 60;
    let second = prng.next_u64() % 60;
    format!("{year:04}:{month:02}:{day:02} {hour:02}:{minute:02}:{second:02}")
}

// resolve_device — random pool picks modulo count; ios/android filter; device_id matches.
pub fn resolve_device(cfg: &ExifConfig, prng: &mut Prng) -> Option<ExifDevice> {
    let all = device_list();
    if cfg.mode == "off" {
        return None;
    }
    if cfg.mode == "device" {
        if let Some(id) = &cfg.device_id {
            return all.into_iter().find(|d| &d.id == id);
        }
    }
    let pool: Vec<ExifDevice> = match cfg.pool.as_str() {
        "ios" => all.into_iter().filter(|d| d.os == "ios").collect(),
        "android" => all.into_iter().filter(|d| d.os == "android").collect(),
        _ => all,
    };
    if pool.is_empty() {
        return None;
    }
    let idx = (prng.next_u64() % pool.len() as u64) as usize;
    Some(pool[idx].clone())
}

// ---- TIFF/EXIF APP1 writer (IFD0 7 entries + Exif sub-IFD 7 entries) ----

fn build_tiff(make: &str, model: &str, software: &str, datetime: &str, width: u32, height: u32) -> Vec<u8> {
    const N0: usize = 7;
    const N1: usize = 7;
    let subifd_off: u32 = (8 + 2 + 12 * N0 + 4) as u32; // 98
    let data_start: usize = (subifd_off as usize) + 2 + 12 * N1 + 4; // 188

    let mut data: Vec<u8> = Vec::new();
    let mut ascii = |s: &str| -> (u32, [u8; 4]) {
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0);
        let count = bytes.len() as u32;
        if bytes.len() <= 4 {
            let mut v = [0u8; 4];
            v[..bytes.len()].copy_from_slice(&bytes);
            (count, v)
        } else {
            let off = data_start + data.len();
            data.extend_from_slice(&bytes);
            if data.len() % 2 == 1 {
                data.push(0);
            }
            (count, (off as u32).to_le_bytes())
        }
    };

    fn entry(tag: u16, typ: u16, count: u32, value: [u8; 4]) -> [u8; 12] {
        let mut e = [0u8; 12];
        e[0..2].copy_from_slice(&tag.to_le_bytes());
        e[2..4].copy_from_slice(&typ.to_le_bytes());
        e[4..8].copy_from_slice(&count.to_le_bytes());
        e[8..12].copy_from_slice(&value);
        e
    }
    let short_val = |v: u16| -> [u8; 4] { let b = v.to_le_bytes(); [b[0], b[1], 0, 0] };
    let long_val = |v: u32| -> [u8; 4] { v.to_le_bytes() };

    let (mk_c, mk_v) = ascii(make);
    let (mo_c, mo_v) = ascii(model);
    let (sw_c, sw_v) = ascii(software);
    let (dt_c, dt_v) = ascii(datetime);
    let (dto_c, dto_v) = ascii(datetime);
    let (dtd_c, dtd_v) = ascii(datetime);

    let ifd0: [[u8; 12]; N0] = [
        entry(0x010F, 2, mk_c, mk_v),                  // Make
        entry(0x0110, 2, mo_c, mo_v),                  // Model
        entry(0x0112, 3, 1, short_val(1)),             // Orientation
        entry(0x0131, 2, sw_c, sw_v),                  // Software
        entry(0x0132, 2, dt_c, dt_v),                  // DateTime
        entry(0x0213, 3, 1, short_val(1)),             // YCbCrPositioning
        entry(0x8769, 4, 1, long_val(subifd_off)),     // ExifIFDPointer
    ];
    let sub: [[u8; 12]; N1] = [
        entry(0x9000, 7, 4, *b"0230"),                 // ExifVersion
        entry(0x9003, 2, dto_c, dto_v),                // DateTimeOriginal
        entry(0x9004, 2, dtd_c, dtd_v),                // DateTimeDigitized
        entry(0x9101, 7, 4, [1, 2, 3, 0]),             // ComponentsConfiguration
        entry(0xA001, 3, 1, short_val(1)),             // ColorSpace
        entry(0xA002, 4, 1, long_val(width)),          // PixelXDimension
        entry(0xA003, 4, 1, long_val(height)),         // PixelYDimension
    ];

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&(N0 as u16).to_le_bytes());
    for e in &ifd0 { out.extend_from_slice(e); }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(N1 as u16).to_le_bytes());
    for e in &sub { out.extend_from_slice(e); }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

// inject_exif — JPEG only, insert APP1 after SOI. Best-effort (non-fatal).
pub fn inject_exif(path: &std::path::Path, device: &ExifDevice, datetime: &str, width: u32, height: u32) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err("output is not a JPEG (missing SOI)".into());
    }
    let sw = device.software.clone().unwrap_or_default();
    let tiff = build_tiff(&device.make, &device.model, &sw, datetime, width, height);

    // APP1 = FFE1 + len(2, BE, includes len bytes) + "Exif\0\0" + tiff
    let mut seg: Vec<u8> = Vec::new();
    seg.extend_from_slice(b"Exif\0\0");
    seg.extend_from_slice(&tiff);
    let seg_len = seg.len() + 2;
    if seg_len > 0xFFFF {
        return Err("exif segment too large".into());
    }
    let mut app1: Vec<u8> = vec![0xFF, 0xE1, (seg_len >> 8) as u8, (seg_len & 0xFF) as u8];
    app1.extend_from_slice(&seg);

    let mut out = Vec::with_capacity(bytes.len() + app1.len());
    out.extend_from_slice(&bytes[..2]); // SOI
    out.extend_from_slice(&app1);
    out.extend_from_slice(&bytes[2..]);
    std::fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(())
}
