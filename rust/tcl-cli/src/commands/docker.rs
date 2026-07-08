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

//! `tcl docker` verb group — Dockerfile generation for Tcl projects.
//!
//! Thin wrappers over `tcl_pkg::docker`: parse CLI args, call the library,
//! format output. Library errors print
//! `error: <msg>` to stderr and return exit code 1.

// Handlers return `anyhow::Result<u8>` for a uniform dispatch signature even
// when a given verb cannot fail; the wrap is the interface contract.
#![allow(clippy::unnecessary_wraps)]

use std::collections::BTreeMap;

use serde_json::json;
use tcl_pkg::docker::{
    DockerfileSpec, SUPPORTED_TCL_VERSIONS, available_recipes, generate_dockerfile,
    tcl_install_recipe, write_dockerfile,
};
use tcl_pkg::ui;

use crate::cli::DockerCommand;

/// Dispatch a `tcl docker` sub-action.
pub fn run(action: &DockerCommand) -> anyhow::Result<u8> {
    match action {
        DockerCommand::Create { .. } => run_create(action),
        DockerCommand::Recipe {
            image,
            tcl_version,
            json,
        } => run_recipe(image, tcl_version, *json),
        DockerCommand::Info { json } => run_info(*json),
    }
}

fn run_create(action: &DockerCommand) -> anyhow::Result<u8> {
    let DockerCommand::Create {
        image,
        tcl_version,
        output,
        workdir,
        entrypoint,
        venv,
        no_copy,
        no_packages,
        extra_package,
        label,
        env,
        cli_version,
        force,
        json,
    } = action
    else {
        unreachable!("run_create only called for Create");
    };

    let colour = ui::use_colour(Some(!*json));

    let mut spec = DockerfileSpec {
        base_image: image.clone(),
        tcl_version: tcl_version.clone(),
        workdir: workdir.clone(),
        copy_project: !*no_copy,
        install_packages: !*no_packages,
        create_venv: *venv,
        entrypoint: entrypoint.clone(),
        cli_version: cli_version.clone(),
        extra_packages: extra_package.clone(),
        ..Default::default()
    };

    let mut labels = BTreeMap::new();
    for item in label {
        let Some((key, value)) = item.split_once('=') else {
            eprintln!("error: --label must be key=value, got: {item}");
            return Ok(1);
        };
        labels.insert(key.to_string(), value.to_string());
    }
    spec.labels = labels;

    let mut envs = BTreeMap::new();
    for item in env {
        let Some((key, value)) = item.split_once('=') else {
            eprintln!("error: --env must be key=value, got: {item}");
            return Ok(1);
        };
        envs.insert(key.to_string(), value.to_string());
    }
    spec.env = envs;

    let written = match write_dockerfile(output, &spec, *force) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };

    if *json {
        let content = match generate_dockerfile(&spec) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(1);
            }
        };
        println!(
            "{}",
            ui::json_output(&json!({
                "path": written.to_string_lossy(),
                "base_image": spec.base_image,
                "tcl_version": spec.tcl_version,
                "content": content,
            }))
        );
    } else {
        println!(
            "{}",
            ui::ok(&format!("wrote {}", written.display()), colour)
        );
        println!(
            "{}",
            ui::dim(
                &format!("  base: {}  tcl: {}", spec.base_image, spec.tcl_version),
                colour
            )
        );
        println!("{}", ui::dim("  docker build -t myapp .", colour));
    }
    Ok(0)
}

fn run_recipe(image: &str, tcl_version: &str, json: bool) -> anyhow::Result<u8> {
    let recipe = match tcl_install_recipe(image, tcl_version) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if json {
        println!(
            "{}",
            ui::json_output(&json!({
                "image": image,
                "tcl_version": tcl_version,
                "recipe": recipe,
            }))
        );
    } else {
        println!("{recipe}");
    }
    Ok(0)
}

fn run_info(json: bool) -> anyhow::Result<u8> {
    let recipes = available_recipes();
    if json {
        let families: BTreeMap<String, Vec<String>> = recipes.clone();
        println!(
            "{}",
            ui::json_output(&json!({
                "supported_tcl_versions": SUPPORTED_TCL_VERSIONS.to_vec(),
                "families": families,
            }))
        );
    } else {
        let colour = ui::use_colour(None);
        println!("{}", ui::bold("Supported Tcl versions:", colour));
        for v in SUPPORTED_TCL_VERSIONS {
            println!("  {v}");
        }
        println!();
        println!("{}", ui::bold("Install recipe families:", colour));
        for (family, versions) in &recipes {
            println!("  {family:12}  {}", versions.join(", "));
        }
    }
    Ok(0)
}
