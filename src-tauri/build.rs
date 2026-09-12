use std::{env, fs, path::Path};

struct ZipEntry<'a> {
    archive_name: &'a str,
    source: &'a str,
    unix_mode: u32,
}

struct CentralEntry {
    name: Vec<u8>,
    crc32: u32,
    size: u32,
    offset: u32,
    unix_mode: u32,
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn create_zip(manifest: &Path, output_path: &Path, entries: &[ZipEntry<'_>]) {
    let mut output = Vec::new();
    let mut central = Vec::new();
    const FLAGS: u16 = 0x0800;
    const DOS_DATE_2026_01_01: u16 = ((2026 - 1980) << 9) | (1 << 5) | 1;

    for entry in entries {
        let source = manifest.join(entry.source);
        println!("cargo:rerun-if-changed={}", source.display());
        let data = fs::read(&source)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
        let name = entry.archive_name.as_bytes();
        let size = u32::try_from(data.len()).expect("player tool is too large");
        let offset = u32::try_from(output.len()).expect("player tool archive is too large");
        let checksum = crc32(&data);

        write_u32(&mut output, 0x0403_4b50);
        write_u16(&mut output, 20);
        write_u16(&mut output, FLAGS);
        write_u16(&mut output, 0);
        write_u16(&mut output, 0);
        write_u16(&mut output, DOS_DATE_2026_01_01);
        write_u32(&mut output, checksum);
        write_u32(&mut output, size);
        write_u32(&mut output, size);
        write_u16(&mut output, name.len() as u16);
        write_u16(&mut output, 0);
        output.extend_from_slice(name);
        output.extend_from_slice(&data);

        central.push(CentralEntry {
            name: name.to_vec(),
            crc32: checksum,
            size,
            offset,
            unix_mode: entry.unix_mode,
        });
    }

    let central_offset = u32::try_from(output.len()).expect("player tool archive is too large");
    for entry in &central {
        write_u32(&mut output, 0x0201_4b50);
        write_u16(&mut output, 0x0314);
        write_u16(&mut output, 20);
        write_u16(&mut output, FLAGS);
        write_u16(&mut output, 0);
        write_u16(&mut output, 0);
        write_u16(&mut output, DOS_DATE_2026_01_01);
        write_u32(&mut output, entry.crc32);
        write_u32(&mut output, entry.size);
        write_u32(&mut output, entry.size);
        write_u16(&mut output, entry.name.len() as u16);
        write_u16(&mut output, 0);
        write_u16(&mut output, 0);
        write_u16(&mut output, 0);
        write_u16(&mut output, 0);
        write_u32(&mut output, entry.unix_mode << 16);
        write_u32(&mut output, entry.offset);
        output.extend_from_slice(&entry.name);
    }

    let central_size =
        u32::try_from(output.len()).expect("player tool archive is too large") - central_offset;
    let entry_count = u16::try_from(central.len()).expect("too many player tool files");
    write_u32(&mut output, 0x0605_4b50);
    write_u16(&mut output, 0);
    write_u16(&mut output, 0);
    write_u16(&mut output, entry_count);
    write_u16(&mut output, entry_count);
    write_u32(&mut output, central_size);
    write_u32(&mut output, central_offset);
    write_u16(&mut output, 0);

    fs::write(output_path, output)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", output_path.display()));
}

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is missing");
    let output = env::var("OUT_DIR").expect("OUT_DIR is missing");
    let manifest = Path::new(&manifest);
    let output = Path::new(&output);

    create_zip(
        manifest,
        &output.join("stickplay-player-tools-windows.zip"),
        &[
            ZipEntry {
                archive_name: "install.cmd",
                source: "player-tools/windows/install.cmd",
                unix_mode: 0o100644,
            },
            ZipEntry {
                archive_name: "install.ps1",
                source: "player-tools/windows/install.ps1",
                unix_mode: 0o100644,
            },
            ZipEntry {
                archive_name: "launch-player.ps1",
                source: "player-tools/windows/launch-player.ps1",
                unix_mode: 0o100644,
            },
            ZipEntry {
                archive_name: "uninstall.cmd",
                source: "player-tools/windows/uninstall.cmd",
                unix_mode: 0o100644,
            },
            ZipEntry {
                archive_name: "uninstall.ps1",
                source: "player-tools/windows/uninstall.ps1",
                unix_mode: 0o100644,
            },
        ],
    );
    create_zip(
        manifest,
        &output.join("stickplay-player-tools-macos.zip"),
        &[
            ZipEntry {
                archive_name: "Install StickPlay VLC.command",
                source: "player-tools/macos/install.command",
                unix_mode: 0o100755,
            },
            ZipEntry {
                archive_name: "Uninstall StickPlay VLC.command",
                source: "player-tools/macos/uninstall.command",
                unix_mode: 0o100755,
            },
        ],
    );
}
