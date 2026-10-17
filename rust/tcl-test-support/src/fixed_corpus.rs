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

//! Exact external source inputs for fixed conformance controls.

use std::io;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

/// Authored provenance and complete expected bytes for one upstream input.
/// A matching file establishes this input only, never the entire corpus,
/// physical path identity, provider execution or semantic success.
pub struct FixedCorpusFixture {
    /// Upstream release recorded for this original input.
    pub release: &'static str,
    /// Upstream location used when recording the fixed bytes.
    pub origin: &'static str,
    /// Original path relative to the supplied source-tree root.
    pub relative_path: &'static str,
    /// SHA-256 of the complete original byte image.
    pub sha256: [u8; 32],
}

/// The original callback-prefix control from the repository's Tcllib pin.
pub const TCLLIB_2_0_CHANNEL_CAT: FixedCorpusFixture = FixedCorpusFixture {
    release: "Tcllib 2.0 (tcllib-2-0)",
    origin: "https://github.com/tcltk/tcllib/blob/tcllib-2-0/modules/virtchannel_base/cat.tcl",
    relative_path: "modules/virtchannel_base/cat.tcl",
    sha256: fixed_sha256(include_str!("../data/tcllib-2.0-cat.sha256")),
};

// The bootstrap verifier and this typed fixture share one expected byte digest.
const fn fixed_sha256(text: &str) -> [u8; 32] {
    let bytes = text.as_bytes();
    assert!(bytes.len() == 65 && bytes[64] == b'\n');
    let mut digest = [0; 32];
    let mut index = 0;
    while index < 64 {
        let byte = bytes[index];
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("fixed fixture SHA-256 must be lowercase hexadecimal"),
        };
        digest[index / 2] = digest[index / 2] * 16 + nibble;
        index += 1;
    }
    digest
}

/// A successfully read, byte-checked fixed input. Missing or different files
/// are setup failures; neither can be reported as a passing semantic control.
pub struct FixedCorpusSource {
    fixture: &'static FixedCorpusFixture,
    path: PathBuf,
    bytes: Vec<u8>,
}

impl FixedCorpusSource {
    pub fn read(root: &Path, fixture: &'static FixedCorpusFixture) -> io::Result<Self> {
        let relative = Path::new(fixture.relative_path);
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fixed corpus input needs a relative source path",
            ));
        }
        let path = root.join(relative);
        let bytes = std::fs::read(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "{} setup: {}: {error}; fetch with make install-test-deps",
                    fixture.release,
                    path.display()
                ),
            )
        })?;
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        if digest != fixture.sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} setup: {} differs from fixed upstream input {}",
                    fixture.release,
                    path.display(),
                    fixture.origin
                ),
            ));
        }
        Ok(Self {
            fixture,
            path,
            bytes,
        })
    }

    #[must_use]
    pub const fn fixture(&self) -> &'static FixedCorpusFixture {
        self.fixture
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
