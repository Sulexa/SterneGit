use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
};

use flate2::read::DeflateDecoder;

const END_SIGNATURE: u32 = 0x0605_4b50;
const ENTRY_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const END_SIZE: u64 = 22;
const LONGEST_COMMENT: u64 = u16::MAX as u64;
const ENTRY_FIXED_SIZE: usize = 46;
const LOCAL_FIXED_SIZE: usize = 30;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
const ENCRYPTED_FLAG: u16 = 1;
const ZIP64_MARK: u32 = u32::MAX;

struct Entry {
    name: String,
    method: u16,
    packed: u64,
    unpacked: u64,
    local_header: u64,
}

pub(crate) fn unpack(zip: &Path, into: &Path) -> io::Result<()> {
    let mut file = BufReader::new(File::open(zip)?);
    for entry in entries(&mut file)? {
        let Some(relative) = safe_path(&entry.name) else {
            return Err(broken(&format!("{} would land outside the folder", entry.name)));
        };
        let target = into.join(relative);
        if entry.name.ends_with(['/', '\\']) {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder)?;
        }
        write_entry(&mut file, &entry, &target)?;
    }
    Ok(())
}

fn entries(file: &mut (impl Read + Seek)) -> io::Result<Vec<Entry>> {
    let (count, mut offset) = central_directory(file)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        file.seek(SeekFrom::Start(offset))?;
        let fixed = read_bytes(file, ENTRY_FIXED_SIZE)?;
        if u32_at(&fixed, 0) != ENTRY_SIGNATURE {
            return Err(broken("an entry of the central directory is damaged"));
        }
        let flags = u16_at(&fixed, 8);
        let method = u16_at(&fixed, 10);
        let [packed, unpacked] = [u32_at(&fixed, 20), u32_at(&fixed, 24)];
        let [name_length, extra_length, comment_length] =
            [u16_at(&fixed, 28), u16_at(&fixed, 30), u16_at(&fixed, 32)].map(usize::from);
        let local_header = u32_at(&fixed, 42);
        if flags & ENCRYPTED_FLAG != 0 || ![STORED, DEFLATED].contains(&method) {
            return Err(broken("an entry is encrypted or packed in an unknown way"));
        }
        if [packed, unpacked, local_header].contains(&ZIP64_MARK) {
            return Err(broken("the archive is too large"));
        }
        let name = String::from_utf8(read_bytes(file, name_length)?)
            .map_err(|_| broken("an entry's name isn't UTF-8"))?;
        entries.push(Entry {
            name,
            method,
            packed: packed.into(),
            unpacked: unpacked.into(),
            local_header: local_header.into(),
        });
        offset += (ENTRY_FIXED_SIZE + name_length + extra_length + comment_length) as u64;
    }
    Ok(entries)
}

fn central_directory(file: &mut (impl Read + Seek)) -> io::Result<(usize, u64)> {
    let length = file.seek(SeekFrom::End(0))?;
    let searched = length.min(END_SIZE + LONGEST_COMMENT);
    file.seek(SeekFrom::Start(length - searched))?;
    let tail = read_bytes(file, searched as usize)?;
    let end = (0..tail.len().saturating_sub(END_SIZE as usize - 1))
        .rev()
        .find(|&at| u32_at(&tail, at) == END_SIGNATURE)
        .ok_or_else(|| broken("it isn't a zip archive"))?;
    let count = u16_at(&tail, end + 10);
    let offset = u32_at(&tail, end + 16);
    if offset == ZIP64_MARK {
        return Err(broken("the archive is too large"));
    }
    Ok((count.into(), offset.into()))
}

fn write_entry(file: &mut (impl Read + Seek), entry: &Entry, target: &Path) -> io::Result<()> {
    file.seek(SeekFrom::Start(entry.local_header))?;
    let fixed = read_bytes(file, LOCAL_FIXED_SIZE)?;
    if u32_at(&fixed, 0) != LOCAL_SIGNATURE {
        return Err(broken("an entry is damaged"));
    }
    let skipped = u64::from(u16_at(&fixed, 26)) + u64::from(u16_at(&fixed, 28));
    file.seek(SeekFrom::Current(skipped as i64))?;
    let packed = file.take(entry.packed);
    let mut content: Box<dyn Read + '_> = match entry.method {
        DEFLATED => Box::new(DeflateDecoder::new(packed)),
        _ => Box::new(packed),
    };
    let mut written = BufWriter::new(File::create(target)?);
    let copied = io::copy(&mut content, &mut written)?;
    if copied != entry.unpacked {
        return Err(broken(&format!("{} unpacked to the wrong size", entry.name)));
    }
    written.into_inner().map_err(io::IntoInnerError::into_error)?.sync_all()
}

fn safe_path(name: &str) -> Option<PathBuf> {
    let relative = PathBuf::from(name.replace('\\', "/"));
    let plain = relative
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    let names_something = relative.components().any(|component| matches!(component, Component::Normal(_)));
    (plain && names_something && !name.contains(':')).then_some(relative)
}

fn read_bytes(file: &mut impl Read, length: usize) -> io::Result<Vec<u8>> {
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn broken(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("The update couldn't be unpacked: {why}"))
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{io::Write as _, path::PathBuf};

    use flate2::{Compression, write::DeflateEncoder};

    use super::{DEFLATED, STORED, safe_path, unpack};

    pub(crate) fn zip(files: &[(&str, &[u8])], method: u16) -> Vec<u8> {
        let mut archive = Vec::new();
        let mut directory = Vec::new();
        for (name, content) in files {
            let packed = match method {
                DEFLATED => {
                    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(content).unwrap();
                    encoder.finish().unwrap()
                }
                _ => content.to_vec(),
            };
            let offset = archive.len() as u32;
            let crc = crc32fast::hash(content);
            let sizes = [packed.len() as u32, content.len() as u32];
            archive.extend(0x0403_4b50u32.to_le_bytes());
            archive.extend([20u16, 0, method, 0, 0].iter().flat_map(|value| value.to_le_bytes()));
            archive.extend(crc.to_le_bytes());
            archive.extend(sizes.iter().flat_map(|size| size.to_le_bytes()));
            archive.extend([name.len() as u16, 0].iter().flat_map(|value| value.to_le_bytes()));
            archive.extend(name.as_bytes());
            archive.extend(&packed);
            directory.extend(0x0201_4b50u32.to_le_bytes());
            directory.extend([20u16, 20, 0, method, 0, 0].iter().flat_map(|value| value.to_le_bytes()));
            directory.extend(crc.to_le_bytes());
            directory.extend(sizes.iter().flat_map(|size| size.to_le_bytes()));
            directory.extend([name.len() as u16, 0, 0, 0, 0].iter().flat_map(|value| value.to_le_bytes()));
            directory.extend(0u32.to_le_bytes());
            directory.extend(offset.to_le_bytes());
            directory.extend(name.as_bytes());
        }
        let directory_offset = archive.len() as u32;
        archive.extend(&directory);
        archive.extend(0x0605_4b50u32.to_le_bytes());
        archive.extend([0u16, 0, files.len() as u16, files.len() as u16].iter().flat_map(|value| value.to_le_bytes()));
        archive.extend((directory.len() as u32).to_le_bytes());
        archive.extend(directory_offset.to_le_bytes());
        archive.extend(0u16.to_le_bytes());
        archive
    }

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-archive-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn deflated_and_stored_entries_unpack_into_their_folders() {
        for (method, name) in [(DEFLATED, "deflated"), (STORED, "stored")] {
            let folder = scratch(name);
            let manifest = br#"{"id":"github"}"#.repeat(40);
            std::fs::write(
                folder.join("portable.zip"),
                zip(
                    &[
                        ("sternegit.exe", b"MZ new exe"),
                        ("connectors/", b""),
                        ("connectors\\github\\connector.json", &manifest),
                    ],
                    method,
                ),
            )
            .unwrap();
            let into = folder.join("unpacked");
            unpack(&folder.join("portable.zip"), &into).unwrap();
            let exe = std::fs::read(into.join("sternegit.exe")).unwrap();
            let read_manifest = std::fs::read(into.join("connectors").join("github").join("connector.json")).unwrap();
            let _ = std::fs::remove_dir_all(&folder);
            assert_eq!(exe, b"MZ new exe");
            assert_eq!(read_manifest, manifest);
        }
    }

    #[test]
    fn an_entry_reaching_outside_the_folder_stops_the_unpacking() {
        let folder = scratch("escape");
        std::fs::write(folder.join("evil.zip"), zip(&[("../outside.exe", b"MZ")], STORED)).unwrap();
        let result = unpack(&folder.join("evil.zip"), &folder.join("unpacked"));
        let escaped = folder.join("outside.exe").exists();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(result.is_err());
        assert!(!escaped);
    }

    #[test]
    fn only_plain_relative_names_are_unpacked() {
        assert_eq!(safe_path("connectors/github/x.wasm"), Some(PathBuf::from("connectors/github/x.wasm")));
        for name in ["../x", "a/../../x", "/etc/x", "C:/x", "C:x", "\\\\server\\share\\x", "", "./"] {
            assert_eq!(safe_path(name), None, "{name}");
        }
    }

    #[test]
    fn a_file_that_is_not_a_zip_is_refused() {
        let folder = scratch("not-zip");
        std::fs::write(folder.join("page.zip"), "<html>Not Found</html>").unwrap();
        let result = unpack(&folder.join("page.zip"), &folder.join("unpacked"));
        let _ = std::fs::remove_dir_all(&folder);
        assert!(result.is_err());
    }
}
