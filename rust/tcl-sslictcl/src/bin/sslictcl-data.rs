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

//! Thin command-line facade for source-data update scripts.

use std::env;
use std::fs;
use std::process::ExitCode;

use tcl_sslictcl::{
    EmbeddedDataset, TrustProgramSnapshot, canonical_dataset_json, compile_trust_snapshots,
    import_testssl_json,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sslictcl-data: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.as_slice() {
        [command, input] if command == "testssl-to-dsl" => {
            testssl_to_dsl(input, "testssl")
        }
        [command, input, name] if command == "testssl-to-dsl" => testssl_to_dsl(input, name),
        [command, file] if command == "check-trust" => check_trust(file),
        [command, output, inputs @ ..] if command == "compile-trust" && !inputs.is_empty() => {
            compile_trust(output, inputs)
        }
        _ => Err(
            "usage: sslictcl-data testssl-to-dsl INPUT [NAME] | check-trust DATASET | compile-trust OUTPUT SNAPSHOT..."
                .to_owned(),
        ),
    }
}

fn testssl_to_dsl(input: &str, name: &str) -> Result<(), String> {
    let source =
        fs::read_to_string(input).map_err(|error| format!("cannot read {input}: {error}"))?;
    let imported = import_testssl_json(&source).map_err(|error| error.to_string())?;
    let dsl = imported.to_sslictcl(name);
    tcl_sslictcl::load(&dsl)
        .map_err(|error| format!("generated SslicTcl failed validation: {error}"))?;
    print!("{dsl}");
    Ok(())
}

fn check_trust(file: &str) -> Result<(), String> {
    let source =
        fs::read_to_string(file).map_err(|error| format!("cannot read {file}: {error}"))?;
    let dataset: EmbeddedDataset =
        serde_json::from_str(&source).map_err(|error| format!("invalid dataset JSON: {error}"))?;
    dataset.trust.validate()?;
    dataset.trust.anchor_certificates()?;
    Ok(())
}

fn compile_trust(output: &str, input_files: &[String]) -> Result<(), String> {
    let snapshots = input_files
        .iter()
        .map(|file| {
            let source =
                fs::read_to_string(file).map_err(|error| format!("cannot read {file}: {error}"))?;
            serde_json::from_str::<TrustProgramSnapshot>(&source)
                .map_err(|error| format!("invalid snapshot {file}: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let dataset = compile_trust_snapshots(&snapshots).map_err(|error| error.to_string())?;
    let canonical = canonical_dataset_json(&dataset).map_err(|error| error.to_string())?;
    fs::write(output, canonical).map_err(|error| format!("cannot write {output}: {error}"))
}
