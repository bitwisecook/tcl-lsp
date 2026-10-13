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

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct UcsFixture {
    pub path: PathBuf,
    directory: PathBuf,
}

impl Drop for UcsFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).expect("remove temporary UCS material");
    }
}

pub fn ucs(template: &Path, chain: bool) -> (UcsFixture, String) {
    let directory = std::env::temp_dir().join(format!(
        "tcl-certificate-ucs-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&directory)
        .expect("create private UCS directory");
    let fixture = UcsFixture {
        path: directory.join("fixture.ucs"),
        directory,
    };
    let path = fixture.path.clone();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|candidate| {
            candidate
                .join("scripts/dev/generate-certificate-ucs.py")
                .is_file()
        })
        .expect("repository certificate generator");
    let mut command = Command::new("python3");
    command
        .arg(root.join("scripts/dev/generate-certificate-ucs.py"))
        .arg(template)
        .arg(&path);
    if chain {
        command.arg("--chain");
    }
    let output = command.output().expect("run certificate generator");
    assert!(
        output.status.success(),
        "certificate generator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("certificate metadata JSON");
    let fingerprint = metadata["fingerprint_sha256"]
        .as_str()
        .expect("certificate fingerprint")
        .to_owned();
    (fixture, fingerprint)
}
