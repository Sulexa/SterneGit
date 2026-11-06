#[path = "src/app/logo/pixels.rs"]
mod pixels;

use std::{env, fs, path::PathBuf};

const ICON_SIZES: [u32; 4] = [16, 32, 48, 256];
const WINDOW_ICON_ID: u16 = 1;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const ORDINAL_MARK: u16 = 0xffff;
const RESOURCE_HEADER_SIZE: u32 = 32;
const LANGUAGE_NEUTRAL: u16 = 0;
const MOVEABLE_DISCARDABLE: u16 = 0x1010;
const MOVEABLE_PURE_DISCARDABLE: u16 = 0x1030;
const ICON_TYPE: u16 = 1;
const COLOR_PLANES: u16 = 1;
const BITS_PER_PIXEL: u16 = 32;
const SIDE_OF_256_IN_A_BYTE: u8 = 0;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const EIGHT_BIT_RGBA_NOT_INTERLACED: [u8; 5] = [8, 6, 0, 0, 0];
const UNFILTERED_SCANLINE: u8 = 0;
const BEST_COMPRESSION: u8 = 10;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/app/logo/pixels.rs");
    if !links_with_msvc_on_windows() {
        return;
    }
    let resources = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR")).join("icon.res");
    fs::write(&resources, icon_resources()).expect("the build output folder is writable");
    println!("cargo:rustc-link-arg-bins={}", resources.display());
}

fn links_with_msvc_on_windows() -> bool {
    env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows")
        && env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|abi| abi == "msvc")
}

fn icon_resources() -> Vec<u8> {
    let mut file = resource(0, 0, 0, &[]);
    let mut group = [0, ICON_TYPE, ICON_SIZES.len() as u16]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    for (id, size) in (1..).zip(ICON_SIZES) {
        let picture = png(size);
        group.extend(group_entry(size, picture.len(), id));
        file.extend(resource(RT_ICON, id, MOVEABLE_DISCARDABLE, &picture));
    }
    file.extend(resource(RT_GROUP_ICON, WINDOW_ICON_ID, MOVEABLE_PURE_DISCARDABLE, &group));
    file
}

fn resource(kind: u16, id: u16, memory_flags: u16, data: &[u8]) -> Vec<u8> {
    let mut entry = Vec::new();
    entry.extend((data.len() as u32).to_le_bytes());
    entry.extend(RESOURCE_HEADER_SIZE.to_le_bytes());
    for ordinal in [kind, id] {
        entry.extend(ORDINAL_MARK.to_le_bytes());
        entry.extend(ordinal.to_le_bytes());
    }
    entry.extend(0u32.to_le_bytes());
    entry.extend(memory_flags.to_le_bytes());
    entry.extend(LANGUAGE_NEUTRAL.to_le_bytes());
    entry.extend(0u32.to_le_bytes());
    entry.extend(0u32.to_le_bytes());
    entry.extend(data);
    entry.resize(entry.len().next_multiple_of(4), 0);
    entry
}

fn group_entry(size: u32, bytes: usize, id: u16) -> Vec<u8> {
    let side = u8::try_from(size).unwrap_or(SIDE_OF_256_IN_A_BYTE);
    let mut entry = vec![side, side, 0, 0];
    entry.extend(COLOR_PLANES.to_le_bytes());
    entry.extend(BITS_PER_PIXEL.to_le_bytes());
    entry.extend((bytes as u32).to_le_bytes());
    entry.extend(id.to_le_bytes());
    entry
}

fn png(size: u32) -> Vec<u8> {
    let side = size as usize;
    let scale = side / pixels::SIDE;
    let mut scanlines = Vec::with_capacity(side * (side * 4 + 1));
    for y in 0..side {
        scanlines.push(UNFILTERED_SCANLINE);
        let row = pixels::ROWS[y / scale].as_bytes();
        scanlines.extend((0..side).flat_map(|x| rgba(row[x / scale])));
    }
    let mut header = size.to_be_bytes().to_vec();
    header.extend(size.to_be_bytes());
    header.extend(EIGHT_BIT_RGBA_NOT_INTERLACED);
    let mut file = PNG_SIGNATURE.to_vec();
    chunk(&mut file, b"IHDR", &header);
    chunk(&mut file, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&scanlines, BEST_COMPRESSION));
    chunk(&mut file, b"IEND", &[]);
    file
}

fn rgba(pixel: u8) -> [u8; 4] {
    pixels::rgb(pixel).map_or([0; 4], |color| {
        let [_, red, green, blue] = color.to_be_bytes();
        [red, green, blue, u8::MAX]
    })
}

fn chunk(file: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    file.extend((data.len() as u32).to_be_bytes());
    let checked_from = file.len();
    file.extend(kind);
    file.extend(data);
    let checksum = crc32fast::hash(&file[checked_from..]);
    file.extend(checksum.to_be_bytes());
}
