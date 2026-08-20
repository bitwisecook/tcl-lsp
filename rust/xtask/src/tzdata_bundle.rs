// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bundled tzdata blob builder.
//!
//! Packs a curated set of IANA zones from a host `zoneinfo` directory into
//! the `TZBL` bundle the WASM runtime falls back on when it can't preopen
//! `/usr/share/zoneinfo`. The output format is stable and canonical
//! (verified by `cmp` on the produced artifact):
//!
//! ```text
//! magic     "TZBL"   version u8=1   pad 3   n_entries u32 LE
//! repeat:   name_len u8   name N   blob_off u32 LE   blob_len u32 LE
//! pad to 4-byte boundary
//! payload   concatenated TZif blobs
//! ```
//!
//! Entries are sorted ascending by name so the resolver can binary-search
//! the index. Aliases share one deduplicated payload. With `--trim-from` /
//! `--trim-to` the `TZif` transition list is trimmed to the window
//! (`v1`-only, leap tables dropped), matching `_trim_tzif`.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

/// `(name, relative-path)` for every curated zone. Aliases (e.g.
/// `US/Eastern` → `America/New_York`) share a payload. Transcribed from
/// `_CURATED_ZONES`; output is invariant to this ordering
/// (entries are sorted by name before packing).
const CURATED_ZONES: &[(&str, &str)] = &[
    ("UTC", "Etc/UTC"),
    ("GMT", "Etc/GMT"),
    ("Etc/UTC", "Etc/UTC"),
    ("Etc/GMT", "Etc/GMT"),
    ("Etc/GMT-12", "Etc/GMT-12"),
    ("Etc/GMT-11", "Etc/GMT-11"),
    ("Etc/GMT-10", "Etc/GMT-10"),
    ("Etc/GMT-9", "Etc/GMT-9"),
    ("Etc/GMT-8", "Etc/GMT-8"),
    ("Etc/GMT-7", "Etc/GMT-7"),
    ("Etc/GMT-6", "Etc/GMT-6"),
    ("Etc/GMT-5", "Etc/GMT-5"),
    ("Etc/GMT-4", "Etc/GMT-4"),
    ("Etc/GMT-3", "Etc/GMT-3"),
    ("Etc/GMT-2", "Etc/GMT-2"),
    ("Etc/GMT-1", "Etc/GMT-1"),
    ("Etc/GMT+1", "Etc/GMT+1"),
    ("Etc/GMT+2", "Etc/GMT+2"),
    ("Etc/GMT+3", "Etc/GMT+3"),
    ("Etc/GMT+4", "Etc/GMT+4"),
    ("Etc/GMT+5", "Etc/GMT+5"),
    ("Etc/GMT+6", "Etc/GMT+6"),
    ("Etc/GMT+7", "Etc/GMT+7"),
    ("Etc/GMT+8", "Etc/GMT+8"),
    ("Etc/GMT+9", "Etc/GMT+9"),
    ("Etc/GMT+10", "Etc/GMT+10"),
    ("Etc/GMT+11", "Etc/GMT+11"),
    ("Etc/GMT+12", "Etc/GMT+12"),
    ("America/New_York", "America/New_York"),
    ("America/Chicago", "America/Chicago"),
    ("America/Denver", "America/Denver"),
    ("America/Detroit", "America/Detroit"),
    ("America/Indianapolis", "America/Indiana/Indianapolis"),
    (
        "America/Indiana/Indianapolis",
        "America/Indiana/Indianapolis",
    ),
    ("America/Los_Angeles", "America/Los_Angeles"),
    ("America/Phoenix", "America/Phoenix"),
    ("America/Anchorage", "America/Anchorage"),
    ("America/Halifax", "America/Halifax"),
    ("America/St_Johns", "America/St_Johns"),
    ("America/Toronto", "America/Toronto"),
    ("America/Vancouver", "America/Vancouver"),
    ("America/Mexico_City", "America/Mexico_City"),
    ("America/Sao_Paulo", "America/Sao_Paulo"),
    ("America/Buenos_Aires", "America/Argentina/Buenos_Aires"),
    (
        "America/Argentina/Buenos_Aires",
        "America/Argentina/Buenos_Aires",
    ),
    ("America/Santiago", "America/Santiago"),
    ("America/Bogota", "America/Bogota"),
    ("America/Lima", "America/Lima"),
    ("America/Caracas", "America/Caracas"),
    ("Europe/London", "Europe/London"),
    ("Europe/Paris", "Europe/Paris"),
    ("Europe/Berlin", "Europe/Berlin"),
    ("Europe/Amsterdam", "Europe/Amsterdam"),
    ("Europe/Brussels", "Europe/Brussels"),
    ("Europe/Madrid", "Europe/Madrid"),
    ("Europe/Rome", "Europe/Rome"),
    ("Europe/Vienna", "Europe/Vienna"),
    ("Europe/Zurich", "Europe/Zurich"),
    ("Europe/Dublin", "Europe/Dublin"),
    ("Europe/Lisbon", "Europe/Lisbon"),
    ("Europe/Stockholm", "Europe/Stockholm"),
    ("Europe/Oslo", "Europe/Oslo"),
    ("Europe/Copenhagen", "Europe/Copenhagen"),
    ("Europe/Helsinki", "Europe/Helsinki"),
    ("Europe/Warsaw", "Europe/Warsaw"),
    ("Europe/Prague", "Europe/Prague"),
    ("Europe/Budapest", "Europe/Budapest"),
    ("Europe/Athens", "Europe/Athens"),
    ("Europe/Istanbul", "Europe/Istanbul"),
    ("Europe/Moscow", "Europe/Moscow"),
    ("Europe/Kiev", "Europe/Kyiv"),
    ("Europe/Kyiv", "Europe/Kyiv"),
    ("Africa/Cairo", "Africa/Cairo"),
    ("Africa/Johannesburg", "Africa/Johannesburg"),
    ("Africa/Lagos", "Africa/Lagos"),
    ("Africa/Nairobi", "Africa/Nairobi"),
    ("Africa/Casablanca", "Africa/Casablanca"),
    ("Asia/Tokyo", "Asia/Tokyo"),
    ("Asia/Shanghai", "Asia/Shanghai"),
    ("Asia/Hong_Kong", "Asia/Hong_Kong"),
    ("Asia/Singapore", "Asia/Singapore"),
    ("Asia/Seoul", "Asia/Seoul"),
    ("Asia/Taipei", "Asia/Taipei"),
    ("Asia/Bangkok", "Asia/Bangkok"),
    ("Asia/Jakarta", "Asia/Jakarta"),
    ("Asia/Manila", "Asia/Manila"),
    ("Asia/Kolkata", "Asia/Kolkata"),
    ("Asia/Calcutta", "Asia/Kolkata"),
    ("Asia/Karachi", "Asia/Karachi"),
    ("Asia/Dhaka", "Asia/Dhaka"),
    ("Asia/Dubai", "Asia/Dubai"),
    ("Asia/Riyadh", "Asia/Riyadh"),
    ("Asia/Tehran", "Asia/Tehran"),
    ("Asia/Jerusalem", "Asia/Jerusalem"),
    ("Asia/Tel_Aviv", "Asia/Jerusalem"),
    ("Asia/Baghdad", "Asia/Baghdad"),
    ("Australia/Sydney", "Australia/Sydney"),
    ("Australia/Melbourne", "Australia/Melbourne"),
    ("Australia/Brisbane", "Australia/Brisbane"),
    ("Australia/Perth", "Australia/Perth"),
    ("Australia/Adelaide", "Australia/Adelaide"),
    ("Pacific/Auckland", "Pacific/Auckland"),
    ("Pacific/Honolulu", "Pacific/Honolulu"),
    ("Pacific/Fiji", "Pacific/Fiji"),
    ("Pacific/Guam", "Pacific/Guam"),
    ("US/Eastern", "America/New_York"),
    ("US/Central", "America/Chicago"),
    ("US/Mountain", "America/Denver"),
    ("US/Pacific", "America/Los_Angeles"),
    ("US/Hawaii", "Pacific/Honolulu"),
    ("US/Alaska", "America/Anchorage"),
    ("EST", "EST"),
    ("EST5EDT", "EST5EDT"),
    ("CST6CDT", "CST6CDT"),
    ("MST", "MST"),
    ("MST7MDT", "MST7MDT"),
    ("PST8PDT", "PST8PDT"),
    ("HST", "HST"),
];

const MAGIC: &[u8; 4] = b"TZBL";
const VERSION: u8 = 1;
const TZIF_MAGIC: &[u8; 4] = b"TZif";

/// Build the bundle from `zoneinfo` and write it to `output`.
pub fn run(
    zoneinfo: &Path,
    output: &Path,
    trim_from: Option<i64>,
    trim_to: Option<i64>,
) -> Result<ExitCode> {
    let blob = match build_bundle(zoneinfo, trim_from, trim_to) {
        Ok(blob) => blob,
        Err(message) => {
            eprintln!("{message}");
            return Ok(ExitCode::from(1));
        }
    };
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(output, &blob).with_context(|| format!("writing {}", output.display()))?;
    eprintln!(
        "build_tzdata_bundle: wrote {} bytes ({} zones) to {}",
        group_thousands(blob.len()),
        count_entries(&blob),
        output.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// Read the curated zones from `zoneinfo` and pack them into a bundle.
/// Returns `Err(message)` for `SystemExit(msg)` cases.
fn build_bundle(
    zoneinfo: &Path,
    trim_from: Option<i64>,
    trim_to: Option<i64>,
) -> std::result::Result<Vec<u8>, String> {
    if !zoneinfo.is_dir() {
        return Err(format!(
            "{} is not a directory; pass --zoneinfo /usr/share/zoneinfo",
            zoneinfo.display()
        ));
    }

    // Deduplicate payloads — multiple aliases share one TZif blob.
    let mut payloads: HashMap<&str, Vec<u8>> = HashMap::new();
    let mut entries: Vec<(&str, &str)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for &(alias, rel) in CURATED_ZONES {
        let path = zoneinfo.join(rel);
        if !path.is_file() {
            skipped.push(format!("{alias} → {rel}"));
            continue;
        }
        if !payloads.contains_key(rel) {
            let raw =
                std::fs::read(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
            payloads.insert(rel, trim_tzif(&raw, trim_from, trim_to));
        }
        entries.push((alias, rel));
    }

    if !skipped.is_empty() {
        eprintln!(
            "build_tzdata_bundle: skipped (not present on host):\n  {}",
            skipped.join("\n  ")
        );
    }
    if entries.is_empty() {
        return Err(format!(
            "no zones resolved under {}; tzdata package missing?",
            zoneinfo.display()
        ));
    }

    // Sorted index — the resolver binary-searches it. Zone names are
    // ASCII, so byte order matches the code-point sort.
    entries.sort_by(|a, b| a.0.cmp(b.0));

    // First pass: lay out payload addresses (first-occurrence order over
    // the sorted entries).
    let mut payload_offsets: HashMap<&str, usize> = HashMap::new();
    let mut payload_block: Vec<u8> = Vec::new();
    for &(_alias, rel) in &entries {
        if payload_offsets.contains_key(rel) {
            continue;
        }
        payload_offsets.insert(rel, payload_block.len());
        payload_block.extend_from_slice(&payloads[rel]);
    }

    // Index record size: name_len(1) + name + blob_off(4) + blob_len(4).
    let index_size: usize = entries.iter().map(|(alias, _)| 1 + alias.len() + 8).sum();

    let header_size = 12; // magic(4) + version(1) + pad(3) + n_entries(4)
    let mut payload_start = header_size + index_size;
    let pad = (4 - payload_start % 4) % 4;
    payload_start += pad;

    let mut index = Vec::with_capacity(index_size);
    for &(alias, rel) in &entries {
        let name = alias.as_bytes();
        if name.len() > 255 {
            return Err(format!("zone alias too long: {alias:?}"));
        }
        let blob_off = payload_start + payload_offsets[rel];
        let blob_len = payloads[rel].len();
        index.push(u8::try_from(name.len()).expect("checked <= 255"));
        index.extend_from_slice(name);
        index.extend_from_slice(&u32::try_from(blob_off).unwrap_or(0).to_le_bytes());
        index.extend_from_slice(&u32::try_from(blob_len).unwrap_or(0).to_le_bytes());
    }

    let mut out = Vec::with_capacity(payload_start + payload_block.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(&u32::try_from(entries.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&index);
    out.extend(std::iter::repeat_n(0u8, pad));
    out.extend_from_slice(&payload_block);
    Ok(out)
}

/// Drop transitions outside `[trim_from, trim_to]` from a `TZif` blob,
/// re-emitting a `v1`-only blob (leap tables stripped). Malformed input is
/// returned unchanged.
fn trim_tzif(blob: &[u8], trim_from: Option<i64>, trim_to: Option<i64>) -> Vec<u8> {
    if trim_from.is_none() && trim_to.is_none() {
        return blob.to_vec();
    }
    if !blob.starts_with(TZIF_MAGIC) || blob.len() < 44 {
        return blob.to_vec();
    }

    let u32_be = |at: usize| -> u32 {
        u32::from_be_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]])
    };
    // v1 header: magic(4) + version(1) + reserved(15) + 6×u32be counts @20.
    // Kept as `u32` so they re-emit without a narrowing cast; widened to
    // `usize` only for slicing.
    let ttisutcnt = u32_be(20);
    let ttisstdcnt = u32_be(24);
    let leapcnt = u32_be(28);
    let timecnt = u32_be(32);
    let typecnt = u32_be(36);
    let charcnt = u32_be(40);

    let mut pos = 44;
    let take = |pos: &mut usize, n: usize| -> Option<&[u8]> {
        let end = pos.checked_add(n)?;
        let slice = blob.get(*pos..end)?;
        *pos = end;
        Some(slice)
    };
    let Some(times_raw) = take(&mut pos, 4 * timecnt as usize) else {
        return blob.to_vec();
    };
    let Some(type_idx_raw) = take(&mut pos, timecnt as usize) else {
        return blob.to_vec();
    };
    let Some(ttinfo_raw) = take(&mut pos, 6 * typecnt as usize) else {
        return blob.to_vec();
    };
    let Some(abbr_raw) = take(&mut pos, charcnt as usize) else {
        return blob.to_vec();
    };
    if take(&mut pos, 8 * leapcnt as usize).is_none() {
        return blob.to_vec();
    }
    let Some(std_raw) = take(&mut pos, ttisstdcnt as usize) else {
        return blob.to_vec();
    };
    let Some(utc_raw) = take(&mut pos, ttisutcnt as usize) else {
        return blob.to_vec();
    };

    let times: Vec<i32> = times_raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let mut keep_times: Vec<i32> = Vec::new();
    let mut keep_indices: Vec<u8> = Vec::new();
    let mut last_pre: Option<(i32, u8)> = None;
    for (&t, &ti) in times.iter().zip(type_idx_raw.iter()) {
        if trim_from.is_some_and(|from| i64::from(t) < from) {
            last_pre = Some((t, ti));
            continue;
        }
        if trim_to.is_some_and(|to| i64::from(t) > to) {
            continue;
        }
        keep_times.push(t);
        keep_indices.push(ti);
    }

    // Anchor the start with the last dropped pre-window transition.
    if let Some((pre_t, pre_idx)) = last_pre
        && (keep_times.first() != Some(&pre_t))
    {
        keep_times.insert(0, pre_t);
        keep_indices.insert(0, pre_idx);
    }

    let new_timecnt = u32::try_from(keep_times.len()).unwrap_or(0);
    let mut out = Vec::new();
    out.extend_from_slice(TZIF_MAGIC);
    out.push(0); // version 0 (v1)
    out.extend_from_slice(&[0u8; 15]);
    for count in [ttisutcnt, ttisstdcnt, 0, new_timecnt, typecnt, charcnt] {
        out.extend_from_slice(&count.to_be_bytes());
    }
    if new_timecnt > 0 {
        for t in &keep_times {
            out.extend_from_slice(&t.to_be_bytes());
        }
        out.extend_from_slice(&keep_indices);
    }
    out.extend_from_slice(ttinfo_raw);
    out.extend_from_slice(abbr_raw);
    // leap section omitted (leapcnt = 0)
    out.extend_from_slice(std_raw);
    out.extend_from_slice(utc_raw);
    out
}

/// `n_entries` from a bundle header (0 if the magic is wrong).
fn count_entries(blob: &[u8]) -> u32 {
    if blob.len() < 12 || &blob[..4] != MAGIC {
        return 0;
    }
    u32::from_le_bytes([blob[8], blob[9], blob[10], blob[11]])
}

/// Format `n` with `,` thousands separators.
fn group_thousands(n: usize) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, byte) in digits.bytes().enumerate() {
        if i != 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(char::from(byte));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_grouping_inserts_commas() {
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(42), "42");
        assert_eq!(group_thousands(1_000), "1,000");
        assert_eq!(group_thousands(12_345), "12,345");
        assert_eq!(group_thousands(150_000), "150,000");
    }

    #[test]
    fn untrimmed_tzif_is_verbatim() {
        let blob = b"TZif2\x00\x00\x00 arbitrary bytes".to_vec();
        assert_eq!(trim_tzif(&blob, None, None), blob);
    }

    #[test]
    fn non_tzif_input_is_returned_unchanged_even_with_window() {
        let blob = b"not a tzif file".to_vec();
        assert_eq!(trim_tzif(&blob, Some(0), Some(100)), blob);
    }
}
