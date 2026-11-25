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

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use tcl_dialect::{TclVersion, validate_version_for};
use tcl_syntax::native_string::NativeStringProtocol;

use super::{PackageInfo, PackagePrefer, PackageResolver, candidate_package_infos, sorted_subdirs};
use crate::vfs::SourceStore;

/// Module file advertisements keyed by the release's package name bytes.
///
/// The filename is the registrar in Tcl's module unknown handler. It is not
/// a source operand or evidence that the module was loaded.
#[derive(Debug, Default)]
pub(super) struct ModuleCatalog {
    providers: HashMap<TclVersion, HashMap<Vec<u8>, Vec<PackageInfo>>>,
    roots: HashSet<(TclVersion, PathBuf)>,
}

impl ModuleCatalog {
    pub(super) fn entries(&self) -> impl Iterator<Item = (TclVersion, &PackageInfo)> {
        self.providers
            .iter()
            .flat_map(|(&release, names)| names.values().flatten().map(move |info| (release, info)))
    }

    pub(super) fn provider(
        &self,
        name: &[u8],
        requirements: &[&str],
        exact: bool,
        prefer: PackagePrefer,
        release: TclVersion,
    ) -> Option<&PackageInfo> {
        let packages = self.providers.get(&release)?.get(name)?;
        candidate_package_infos(
            &packages.iter().collect::<Vec<_>>(),
            requirements,
            exact,
            prefer,
            Some(release),
        )
        .into_iter()
        .next()
    }

    pub(super) fn provides(&self, name: &[u8], release: TclVersion) -> bool {
        self.providers
            .get(&release)
            .is_some_and(|names| names.contains_key(name))
    }

    pub(super) fn provides_text(&self, name: &str) -> bool {
        self.providers.keys().any(|&release| {
            self.provides(
                NativeStringProtocol::for_tcl_version(release)
                    .host_text_bytes(name)
                    .as_ref(),
                release,
            )
        })
    }
}

impl PackageResolver {
    /// Index one Tcl module search root in its priority order, under the
    /// selected interpreter release. The bounded walk collects filenames
    /// for later queries without evaluating module bodies.
    pub fn scan_module_path_in(
        &mut self,
        store: &dyn SourceStore,
        root: &Path,
        release: TclVersion,
        max_dirs: usize,
    ) {
        self.index_module_path(store, root, release, max_dirs, false);
    }

    /// Index a source-declared module root ahead of the default roots.
    pub fn prepend_module_path_in(
        &mut self,
        store: &dyn SourceStore,
        root: &Path,
        release: TclVersion,
        max_dirs: usize,
    ) {
        self.index_module_path(store, root, release, max_dirs, true);
    }

    fn index_module_path(
        &mut self,
        store: &dyn SourceStore,
        root: &Path,
        release: TclVersion,
        max_dirs: usize,
        prepend: bool,
    ) {
        if release < TclVersion::V8_5
            || max_dirs == 0
            || !store.is_dir(root)
            || !self.modules.roots.insert((release, root.to_path_buf()))
        {
            return;
        }
        let mut pending = vec![root.to_path_buf()];
        let mut found = Vec::new();
        for _ in 0..max_dirs {
            let Some(dir) = pending.pop() else { break };
            if let Ok(entries) = store.read_dir(&dir) {
                let mut paths: Vec<_> = entries
                    .into_iter()
                    .map(|entry| entry.path)
                    .filter(|path| {
                        path.extension().is_some_and(|ext| ext == "tm") && store.is_file(path)
                    })
                    .collect();
                paths.sort();
                found.extend(paths.into_iter().filter_map(|path| {
                    let (name, version) =
                        module_advertisement(path.strip_prefix(root).ok()?, release)?;
                    Some(PackageInfo {
                        name,
                        version,
                        source_files: vec![path.clone()],
                        pkg_index_path: path,
                        conditions: Default::default(),
                    })
                }));
            }
            pending.extend(
                sorted_subdirs(store, &dir)
                    .into_iter()
                    .rev()
                    .filter(|path| {
                        path.file_name()
                            .and_then(std::ffi::OsStr::to_str)
                            .is_some_and(|name| !name.starts_with('.'))
                    }),
            );
        }
        if !found.is_empty() {
            self.invalidate();
        }
        let policy = NativeStringProtocol::for_tcl_version(release);
        let names = self.modules.providers.entry(release).or_default();
        if prepend {
            found.reverse();
        }
        for provider in found {
            let key = policy.host_text_bytes(&provider.name).into_owned();
            let providers = names.entry(key).or_default();
            if prepend {
                providers.insert(0, provider);
            } else {
                providers.push(provider);
            }
        }
    }
}

fn module_advertisement(relative: &Path, release: TclVersion) -> Option<(String, String)> {
    // Use the Tcl library's pattern and its regexp owner. Tcl tests the
    // module version separately with package vcompare.
    static PATTERN: OnceLock<tcl_regex::Regex> = OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        tcl_regex::Regex::compile_str(
            r"^([_[:alpha:]][:_[:alnum:]]*)-([[:digit:]].*)[.]tm$",
            tcl_regex::defs::REG_ADVANCED,
        )
        .expect("Tcl's module advertisement pattern")
    });
    let filename = relative
        .iter()
        .map(std::ffi::OsStr::to_str)
        .collect::<Option<Vec<_>>>()?
        .join("::");
    let chars: Vec<_> = filename.chars().collect();
    let units: Vec<_> = chars.iter().copied().map(u32::from).collect();
    let tcl_regex::ExecOutcome::Matched(groups) = pattern.exec(&units, 0, 0) else {
        return None;
    };
    let name = groups.get(1)?.as_ref()?;
    let version = groups.get(2)?.as_ref()?;
    let version: String = chars.get(version.start..version.end)?.iter().collect();
    validate_version_for(&version, release)
        .then(|| (chars[name.start..name.end].iter().collect(), version))
}
