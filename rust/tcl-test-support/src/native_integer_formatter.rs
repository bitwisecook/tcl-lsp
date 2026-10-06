// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit formatter admission from the independently pinned reference build.
//! This does not turn a release/profile into formatter authority: the source
//! header and actual archive are checked, the recorded native linker operation
//! produces a new image, and its separate digest is supplied to the ABI loader.

use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::{path::Path, process::Command};
use tcl_host_c_abi::LoadedNativeIntegerFormatter;

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const ARCHIVE: &str = "d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011";
const HEADER: &str = "824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf";

fn digest(path: &Path) -> Result<[u8; 32], String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Sha256::digest(bytes).into())
}
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to String");
    }
    out
}

/// Load the actual pinned C84 build for a positive original-object comparison.
/// Missing source/build/capability is an error, never a skipped positive test.
/// The live loader owns its opened image after the temporary pathname retires.
pub fn load_pinned_c84_integer_formatter(
    repo_root: &Path,
) -> Result<LoadedNativeIntegerFormatter, String> {
    let tree = crate::locate_source_tree(repo_root, tcl_dialect::TclVersion::V8_4, None)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "pinned C84 source tree unavailable".to_owned())?;
    let archive = tree.root.join("unix/libtcl8.4.a");
    let header = tree.root.join("generic/tcl.h");
    if hex(&digest(&archive)?) != ARCHIVE || hex(&digest(&header)?) != HEADER {
        return Err("C84 native formatter reference build identity mismatch".to_owned());
    }
    let directory = std::env::temp_dir().join(format!(
        "tcl-native-integer-formatter-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    std::fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let temporary = super::FixtureDirectory(directory);
    let image = temporary.0.join("libtcl8.4-observed.so");
    let mut command = Command::new("cc");
    command
        .args(["-shared", "-Wl,--whole-archive"])
        .arg(&archive)
        .args([
            "-Wl,--no-whole-archive",
            "-lm",
            "-ldl",
            "-lpthread",
            "-lz",
            "-o",
        ])
        .arg(&image);
    let output = command
        .output()
        .map_err(|error| format!("native archive linker: {error}"))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(format!(
            "native archive linker {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    // Recheck the inputs after the recorded producer, independently of the
    // output image identity subsequently verified by the loader.
    if hex(&digest(&archive)?) != ARCHIVE || hex(&digest(&header)?) != HEADER {
        return Err("C84 native formatter build inputs changed during linking".to_owned());
    }
    let image_sha = digest(&image)?;
    if std::env::var_os("TCL_LSP_ORACLE_PROGRESS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        eprintln!(
            "native-formatter archive_sha={ARCHIVE} header_sha={HEADER} image_sha={} command={command:?}",
            hex(&image_sha)
        );
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    #[cfg(test)]
    {
        let mut mismatched = image_sha;
        mismatched[0] ^= 1;
        assert!(
            matches!(
                LoadedNativeIntegerFormatter::load(&image, mismatched, &executable),
                Err(tcl_platform::NativeIntegerFormatterUnavailable::Build)
            ),
            "an equal-version library cannot donate an unverified build"
        );
    }
    let formatter = LoadedNativeIntegerFormatter::load(&image, image_sha, &executable)
        .map_err(|error| format!("loaded native formatter: {error:?}"))?;
    drop(temporary);
    Ok(formatter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_platform::{NativeIntegerFormatter, NativeIntegerKind, NumericEnvironment};

    #[test]
    fn loaded_c84_updater_matches_812_original_archive_and_image_windows() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let formatter =
            load_pinned_c84_integer_formatter(&root).expect("explicit native build capability");
        assert_eq!(formatter.build().version[..3], [8, 4, 20]);
        let fixture =
            include_str!("../../tcl-host-c-abi/testdata/native_integer_formatter/observations.tsv");
        let mut count = 0;
        for row in fixture.lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let kind = match fields[1] {
                "long" => NativeIntegerKind::Long,
                "wide" => NativeIntegerKind::Wide,
                _ => panic!("native kind"),
            };
            assert_eq!(fields[3], "0", "native original starts without string");
            assert_eq!(fields[5], "1", "native updater installs resident bytes");
            assert_eq!(
                fields[2], fields[4],
                "updater preserves actual numeric type"
            );
            assert_eq!(fields[6], fields[7], "native updater errno observation");
            let expected = fields[8]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| {
                    let digit = |byte: u8| match byte {
                        b'0'..=b'9' => byte - b'0',
                        b'a'..=b'f' => byte - b'a' + 10,
                        _ => panic!("native hex"),
                    };
                    digit(pair[0]) * 16 + digit(pair[1])
                })
                .collect::<Vec<_>>();
            let value = fields[0].parse().unwrap();
            let environment = tcl_host_c_abi::NativeNumericEnvironment;
            let before = environment.state().unwrap();
            let observed = formatter.format(kind, value).unwrap();
            let after = environment.state().unwrap();
            assert_eq!(
                before, after,
                "native updater preserves reached thread errno: {row}"
            );
            assert_eq!(observed, expected, "{row}");
            count += 1;
        }
        assert_eq!(count, 812);
    }
}
