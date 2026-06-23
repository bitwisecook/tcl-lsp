//! The `enrich-wireshark` (`ws-profile`) verb — emit a Wireshark profile dir.
//!
//! Builds a [`tcl_bigip::wireshark_profile::WiresharkProfile`] from one or more
//! configs and writes the `hosts` / `subnets` / `vlans` / `dfilters` /
//! `services` / `ethers` / `colorfilters` / `preferences` / `README.md` files
//! into the output directory.

use std::path::{Path, PathBuf};

use tcl_bigip::wireshark_profile::build_wireshark_profile;

use super::enrich_pcapng::load_configs;

/// `f5 enrich-wireshark`.
#[allow(clippy::unnecessary_wraps)]
pub fn run_enrich_wireshark(configs: &[PathBuf], output: &Path, force: bool) -> anyhow::Result<u8> {
    let configs_with_sources = match load_configs(configs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    if output.exists() && !output.is_dir() {
        eprintln!("error: not a directory: {}", output.display());
        return Ok(2);
    }

    if output.exists() && !force {
        let non_empty = std::fs::read_dir(output).is_ok_and(|mut it| it.next().is_some());
        if non_empty {
            eprintln!(
                "error: {} already exists and is not empty; pass --force to overwrite",
                output.display()
            );
            return Ok(2);
        }
    }

    let profile = build_wireshark_profile(&configs_with_sources);

    let profile_name = output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let files = profile.render_files(&profile_name);

    if let Err(e) = std::fs::create_dir_all(output) {
        eprintln!("error: cannot write profile: {e}");
        return Ok(2);
    }
    for (name, content) in &files {
        if let Err(e) = std::fs::write(output.join(name), content) {
            eprintln!("error: cannot write profile: {e}");
            return Ok(2);
        }
    }

    eprintln!(
        "enrich-wireshark: {} host(s), {} subnet(s), {} vlan(s), {} display-filter(s), \
         {} service(s), {} ether(s), {} colour rule(s) written to {}",
        profile.hosts.len(),
        profile.subnets.len(),
        profile.vlans.len(),
        profile.dfilters.len(),
        profile.services.len(),
        profile.ethers.len(),
        profile.colorfilters.len(),
        output.display(),
    );
    Ok(0)
}
