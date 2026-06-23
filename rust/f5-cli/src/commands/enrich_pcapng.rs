//! The `enrich-pcapng` (`enrich`) verb — inject NRB / DSB blocks into a PCAPNG.
//!
//! Derives a hostname-style label for every BIG-IP IP via
//! [`tcl_bigip::pcap_enrich::build_merged_name_index`] and injects those
//! mappings into the capture as a PCAPNG Name Resolution Block (plus an
//! optional TLS keylog Decryption Secrets Block).
//!
//! A PCAPNG input is annotated directly (NRB/DSB injection). A libpcap input is
//! auto-converted to PCAPNG by shelling out to `editcap` / `tshark`; the
//! converted bytes depend on the external tool's version. When neither tool is
//! present, the conversion is refused.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use tcl_bigip::parser::{BigipConfig, parse_bigip_conf};
use tcl_bigip::pcap_enrich::{
    EnrichResult, NameIndex, build_merged_name_index, enrich_pcapng_with_index,
};
use tcl_bigip::pcapng::is_pcapng_magic;

/// Load each config path into a `(BigipConfig, source)` pair. `-` reads stdin
/// (only once).
pub fn load_configs(paths: &[PathBuf]) -> Result<Vec<(BigipConfig, String)>, String> {
    let mut pairs = Vec::new();
    let mut saw_stdin = false;
    for path in paths {
        let text = if path.as_os_str() == "-" {
            if saw_stdin {
                return Err("`-` (stdin) can only be supplied once".to_owned());
            }
            saw_stdin = true;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        } else {
            read_text_lossy(path).map_err(|e| format!("{}: {e}", path.display()))?
        };
        let config = parse_bigip_conf(&text, "Common");
        pairs.push((config, text));
    }
    Ok(pairs)
}

/// Read a file as UTF-8, replacing invalid sequences.
fn read_text_lossy(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// `f5 enrich-pcapng`.
#[allow(clippy::unnecessary_wraps)]
pub fn run_enrich_pcapng(
    configs: &[PathBuf],
    input: &Path,
    output: &Path,
    keylog: Option<&Path>,
    include_unobserved: bool,
    dry_run: bool,
) -> anyhow::Result<u8> {
    let configs_with_sources = match load_configs(configs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let name_index = build_merged_name_index(&configs_with_sources);

    if dry_run {
        print_dry_run(&name_index);
        return Ok(0);
    }

    let keylog_text: Option<Vec<u8>> = match keylog {
        Some(p) => match std::fs::read(p) {
            // Decode as text replacing invalid sequences, then re-encode as
            // UTF-8 so the emitted bytes are well-formed.
            Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned().into_bytes()),
            Err(e) => {
                eprintln!("error: cannot read keylog: {e}");
                return Ok(2);
            }
        },
        None => None,
    };

    if !input.is_file() {
        eprintln!("error: not a file: {}", input.display());
        return Ok(2);
    }

    let result = match enrich_capture_file(
        input,
        output,
        &name_index,
        keylog_text.as_deref(),
        include_unobserved,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            let _ = std::fs::remove_file(output);
            return Ok(2);
        }
    };

    let converted_note = if result.converted_from_libpcap {
        " (libpcap input converted to pcapng)"
    } else {
        ""
    };
    let keylog_note = if result.keylog_bytes > 0 {
        format!(", keylog {} byte(s) embedded", result.keylog_bytes)
    } else {
        String::new()
    };
    let observed_note = if include_unobserved {
        " (--all: every inventory entry)".to_owned()
    } else {
        format!(
            " out of {} v4 / {} v6 observed",
            result.observed_ipv4, result.observed_ipv6
        )
    };
    eprintln!(
        "enrich-pcapng: {} v4 / {} v6 record(s), {} label(s){observed_note}{keylog_note}{converted_note}",
        result.ipv4_records, result.ipv6_records, result.names_total
    );
    Ok(0)
}

/// Print the address→label mapping (and summary) for `--dry-run`, sorted by
/// address.
fn print_dry_run(name_index: &NameIndex) {
    let mut v4: Vec<&(String, Vec<String>)> = name_index.v4.iter().collect();
    v4.sort_by(|a, b| a.0.cmp(&b.0));
    for (addr, names) in v4 {
        for name in names {
            println!("{addr}\t{name}");
        }
    }
    let mut v6: Vec<&(String, Vec<String>)> = name_index.v6.iter().collect();
    v6.sort_by(|a, b| a.0.cmp(&b.0));
    for (addr, names) in v6 {
        for name in names {
            println!("{addr}\t{name}");
        }
    }
    for (net, name) in &name_index.v4_subnets {
        println!("{}\t{name}", net.text());
    }
    for (net, name) in &name_index.v6_subnets {
        println!("{}\t{name}", net.text());
    }
    eprintln!(
        "enrich-pcapng (dry-run): {} v4 / {} v6 address(es), {} subnet(s), {} label(s)",
        name_index.v4.len(),
        name_index.v6.len(),
        name_index.v4_subnets.len() + name_index.v6_subnets.len(),
        name_index.total_names()
    );
}

/// Enrich a capture file on disk; auto-converts libpcap → pcapng via
/// `editcap` / `tshark`.
fn enrich_capture_file(
    in_path: &Path,
    out_path: &Path,
    name_index: &NameIndex,
    keylog_text: Option<&[u8]>,
    include_unobserved: bool,
) -> Result<EnrichResult, String> {
    let input_bytes = std::fs::read(in_path).map_err(|e| e.to_string())?;
    let magic_is_pcapng = input_bytes.len() >= 4 && is_pcapng_magic(&input_bytes[..4]);

    let (pcapng_bytes, converted) = if magic_is_pcapng {
        (input_bytes, false)
    } else {
        let converted = libpcap_to_pcapng(in_path).ok_or_else(|| {
            let name = in_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!(
                "input {name} is libpcap and neither editcap nor tshark is available \
                 to convert it to pcapng"
            )
        })?;
        (converted, true)
    };

    let (out_bytes, mut result) =
        enrich_pcapng_with_index(&pcapng_bytes, name_index, keylog_text, include_unobserved)
            .map_err(|e| e.to_string())?;

    // Stage in the output's parent and rename into place atomically so an
    // `in.pcapng -> in.pcapng` invocation never truncates the input first.
    write_atomic(out_path, &out_bytes).map_err(|e| e.to_string())?;

    result.converted_from_libpcap = converted;
    Ok(result)
}

/// Convert a libpcap file to PCAPNG via `editcap` then `tshark`. Returns the
/// converted bytes, or `None` if neither tool produced a non-empty output.
fn libpcap_to_pcapng(in_path: &Path) -> Option<Vec<u8>> {
    let tmp = std::env::temp_dir().join(format!("enrich-pcapng-{}.pcapng", std::process::id()));
    for (bin, build) in [
        ("editcap", editcap_args as fn(&Path, &Path) -> Vec<String>),
        ("tshark", tshark_args as fn(&Path, &Path) -> Vec<String>),
    ] {
        let args = build(in_path, &tmp);
        let status = Command::new(bin).args(&args).output();
        if let Ok(out) = status
            && out.status.success()
            && let Ok(meta) = std::fs::metadata(&tmp)
            && meta.len() > 0
            && let Ok(bytes) = std::fs::read(&tmp)
        {
            let _ = std::fs::remove_file(&tmp);
            return Some(bytes);
        }
        let _ = std::fs::remove_file(&tmp);
    }
    None
}

fn editcap_args(in_path: &Path, out_path: &Path) -> Vec<String> {
    vec![
        "-F".to_owned(),
        "pcapng".to_owned(),
        in_path.to_string_lossy().into_owned(),
        out_path.to_string_lossy().into_owned(),
    ]
}

fn tshark_args(in_path: &Path, out_path: &Path) -> Vec<String> {
    vec![
        "-F".to_owned(),
        "pcapng".to_owned(),
        "-r".to_owned(),
        in_path.to_string_lossy().into_owned(),
        "-w".to_owned(),
        out_path.to_string_lossy().into_owned(),
    ]
}

/// Write `bytes` to `path` via a `.partial` staging file + atomic rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = dir {
        std::fs::create_dir_all(dir)?;
    }
    let staging = {
        let mut name = path
            .file_name()
            .map(std::ffi::OsString::from)
            .unwrap_or_default();
        name.push(format!(".{}.partial", std::process::id()));
        path.with_file_name(name)
    };
    std::fs::write(&staging, bytes)?;
    match std::fs::rename(&staging, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&staging);
            Err(e)
        }
    }
}
