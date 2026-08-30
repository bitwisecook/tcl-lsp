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

//! Per-module tcllib package identity (redesign §3.2, phase P5).
//!
//! tcllib is not one package. It is a distribution of ~140 independently
//! versioned modules, and the redesign's package layer takes that
//! literally: each module is its own [`Provider::Package`] with its own
//! version axis, its own **trains**, and its own Tcl-core requirement.
//! This table is the evidence layer for that claim — every row is read
//! out of the bundled `tmp/tcllib-2.0` sources, and the sweep at the
//! bottom of this module keeps the table well-formed.
//!
//! Three facts per module, and each is load-bearing somewhere:
//!
//! - **[`TcllibModule::trains`]** — the versions the module's own
//!   `pkgIndex.tcl` offers, newest first. A module with two rows is a
//!   genuine parallel-train case (`md5` 1.4.6 **and** 2.0.9 in the same
//!   index; `struct::tree` 1.2.3 and 2.1.3; `snit` 1.4.3 and 2.3.4), and
//!   the design's "multi-train truth" bullet is exactly this data. Each
//!   train spells a `package require` requirement, so
//!   [`module_version_set`] reads it with the shipped requirement
//!   algebra — a bare `V` is `[V, major+1)` — and the union of the trains
//!   is the module's applicability. A row is therefore never "pinned to
//!   one version": even a single-train module spans its whole major line.
//! - **[`TcllibModule::core_floor`]** — the module's own
//!   `package require Tcl` / `package vsatisfies [package provide Tcl]`
//!   head guard. tcllib 2.0 raised nearly the whole distribution to Tcl
//!   8.5, and a dozen modules to 8.6; `commands::tcllib` turns this into
//!   the ladder bits a module's commands are gated out of.
//! - **[`TcllibModule::evidence`]** — the file the two facts were read
//!   from, relative to `tmp/tcllib-2.0/modules/`.
//!
//! **Every tcllib module is hosted, never ambient** (the deliverable's
//! placement half). No compiled environment ships a tcllib module as part
//! of its own runtime, so no module is
//! [`is_closed_world_package`](crate::model::surface::is_closed_world_package)
//! or
//! [`is_placement_gated_package`](crate::model::surface::is_placement_gated_package):
//! a module's commands stay leniently visible under §5.3's `open` policy
//! with W120 owning the nag, and the **floor** comes from the document's
//! own `package require`, never from a platform pin. `hosted_modules_are_never_ambient`
//! pins that, so a future environment that tried to place one ambient
//! would fail the build rather than silently become a closed world.
//!
//! [`Provider::Package`]: crate::model::surface::Provider::Package

use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::{VersionAxisId, VersionSet};

/// One tcllib module's package identity, as the bundled tcllib 2.0
/// sources state it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcllibModule {
    /// The name a `package require` writes — **not** the module directory.
    /// The two differ for `oo::util` (directory `ooutil`) and `sha256`
    /// (directory `sha1`), which is why this is data rather than a path
    /// convention.
    pub package: &'static str,
    /// The version trains this module's `pkgIndex.tcl` offers, newest
    /// first. Each entry is a `package require` requirement spelling, so
    /// a bare `2.1.3` means `[2.1.3, 3)`.
    pub trains: &'static [&'static str],
    /// The module's declared Tcl-core floor as a ladder release
    /// (`"8.5"`, `"8.6"`), or `None` when the sources declare none.
    pub core_floor: Option<&'static str>,
    /// The source file the row was read from, under
    /// `tmp/tcllib-2.0/modules/`.
    pub evidence: &'static str,
}

/// Every tcllib module the compiled catalogue models, with the identity
/// facts read out of `tmp/tcllib-2.0`.
///
/// Sorted by package name so the table reads as a census; the lookup is a
/// binary search over it, which `table_is_sorted_and_unique` pins.
pub const TCLLIB_MODULES: &[TcllibModule] = &[
    TcllibModule {
        package: "Markdown",
        trains: &["1.2.4"],
        core_floor: Some("8.5"),
        evidence: "markdown/pkgIndex.tcl",
    },
    TcllibModule {
        package: "SASL",
        trains: &["1.3.4"],
        core_floor: Some("8.5"),
        evidence: "sasl/pkgIndex.tcl",
    },
    TcllibModule {
        package: "aes",
        trains: &["1.2.2"],
        core_floor: Some("8.5"),
        evidence: "aes/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ascii85",
        trains: &["1.1.1"],
        core_floor: Some("8.5"),
        evidence: "base64/pkgIndex.tcl",
    },
    TcllibModule {
        package: "asn",
        trains: &["0.8.5"],
        core_floor: Some("8.5"),
        evidence: "asn/pkgIndex.tcl",
    },
    TcllibModule {
        package: "autoproxy",
        trains: &["1.8.1"],
        core_floor: Some("8.5"),
        evidence: "http/pkgIndex.tcl",
    },
    TcllibModule {
        package: "base32",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "base32/pkgIndex.tcl",
    },
    TcllibModule {
        package: "base32::core",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "base32/pkgIndex.tcl",
    },
    TcllibModule {
        package: "base32::hex",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "base32/pkgIndex.tcl",
    },
    TcllibModule {
        package: "base64",
        trains: &["2.6.1"],
        core_floor: Some("8.5"),
        evidence: "base64/pkgIndex.tcl",
    },
    TcllibModule {
        package: "bee",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "bee/pkgIndex.tcl",
    },
    TcllibModule {
        package: "bench",
        trains: &["0.6"],
        core_floor: Some("8.5"),
        evidence: "bench/pkgIndex.tcl",
    },
    TcllibModule {
        package: "bench::in",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "bench/pkgIndex.tcl",
    },
    TcllibModule {
        package: "bibtex",
        trains: &["0.8"],
        core_floor: Some("8.5"),
        evidence: "bibtex/pkgIndex.tcl",
    },
    TcllibModule {
        package: "blowfish",
        trains: &["1.0.6"],
        core_floor: Some("8.5"),
        evidence: "blowfish/pkgIndex.tcl",
    },
    TcllibModule {
        package: "cksum",
        trains: &["1.1.5"],
        core_floor: Some("8.5"),
        evidence: "crc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "cmdline",
        trains: &["1.5.3"],
        core_floor: Some("8.5"),
        evidence: "cmdline/pkgIndex.tcl",
    },
    TcllibModule {
        package: "comm",
        trains: &["4.7.3"],
        core_floor: Some("8.5"),
        evidence: "comm/pkgIndex.tcl",
    },
    TcllibModule {
        package: "control",
        trains: &["0.1.4"],
        core_floor: Some("8.5"),
        evidence: "control/pkgIndex.tcl",
    },
    TcllibModule {
        package: "counter",
        trains: &["2.0.6"],
        core_floor: Some("8.5"),
        evidence: "counter/pkgIndex.tcl",
    },
    TcllibModule {
        package: "crc16",
        trains: &["1.1.5"],
        core_floor: Some("8.5"),
        evidence: "crc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "crc32",
        trains: &["1.3.4"],
        core_floor: Some("8.5"),
        evidence: "crc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "cron",
        trains: &["2.2"],
        core_floor: Some("8.6"),
        evidence: "cron/pkgIndex.tcl",
    },
    TcllibModule {
        package: "csv",
        trains: &["0.10"],
        core_floor: Some("8.5"),
        evidence: "csv/pkgIndex.tcl",
    },
    TcllibModule {
        // The one guard that reads `package require Tcl` rather than
        // `package provide Tcl`; the floor it states is the same.
        package: "debug",
        trains: &["1.0.7"],
        core_floor: Some("8.5"),
        evidence: "debug/pkgIndex.tcl",
    },
    TcllibModule {
        package: "defer",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "defer/pkgIndex.tcl",
    },
    TcllibModule {
        package: "des",
        trains: &["1.2"],
        core_floor: Some("8.5"),
        evidence: "des/pkgIndex.tcl",
    },
    TcllibModule {
        package: "dns",
        trains: &["1.6.1"],
        core_floor: Some("8.5"),
        evidence: "dns/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools",
        trains: &["1.6.1"],
        core_floor: Some("8.5"),
        evidence: "doctools/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::changelog",
        trains: &["1.2"],
        core_floor: Some("8.5"),
        evidence: "doctools/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::cvs",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "doctools/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::html::cssdefaults",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "doctools2base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::idx",
        trains: &["2.1", "1.2.1"],
        core_floor: Some("8.5"),
        evidence: "doctools/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::msgcat",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "doctools2base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::nroff::man_macros",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "doctools2base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "doctools::toc",
        trains: &["2", "1.3.1"],
        core_floor: Some("8.5"),
        evidence: "doctools/pkgIndex.tcl",
    },
    TcllibModule {
        package: "fileutil",
        trains: &["1.16.3"],
        core_floor: Some("8.5"),
        evidence: "fileutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "fileutil::magic::cgen",
        trains: &["1.3.1"],
        core_floor: Some("8.6"),
        evidence: "fumagic/pkgIndex.tcl",
    },
    TcllibModule {
        package: "fileutil::magic::rt",
        trains: &["3.1"],
        core_floor: Some("8.6"),
        evidence: "fumagic/pkgIndex.tcl",
    },
    TcllibModule {
        // P5's third named hostile shape: a `snit::type` factory whose
        // object carries three command-prefix options and a real
        // looping `foreach` method.
        package: "fileutil::traverse",
        trains: &["0.7"],
        core_floor: Some("8.5"),
        evidence: "fileutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ftp",
        trains: &["2.4.14"],
        core_floor: Some("8.5"),
        evidence: "ftp/pkgIndex.tcl",
    },
    TcllibModule {
        package: "generator",
        trains: &["0.3"],
        core_floor: Some("8.6"),
        evidence: "generator/pkgIndex.tcl",
    },
    TcllibModule {
        package: "gpx",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "gpx/pkgIndex.tcl",
    },
    TcllibModule {
        package: "grammar::fa::op",
        trains: &["0.4.2"],
        core_floor: Some("8.5"),
        evidence: "grammar_fa/pkgIndex.tcl",
    },
    TcllibModule {
        package: "grammar::me::cpu::gasm",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "grammar_me/pkgIndex.tcl",
    },
    TcllibModule {
        package: "grammar::me::tcl",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "grammar_me/pkgIndex.tcl",
    },
    TcllibModule {
        package: "grammar::peg::interp",
        trains: &["0.1.2"],
        core_floor: None,
        evidence: "grammar_peg/pkgIndex.tcl",
    },
    TcllibModule {
        package: "hook",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "hook/pkgIndex.tcl",
    },
    TcllibModule {
        package: "html",
        trains: &["1.6"],
        core_floor: Some("8.5"),
        evidence: "html/pkgIndex.tcl",
    },
    TcllibModule {
        package: "htmlparse",
        trains: &["1.2.3"],
        core_floor: Some("8.5"),
        evidence: "htmlparse/pkgIndex.tcl",
    },
    TcllibModule {
        package: "imap4",
        trains: &["0.5.5"],
        core_floor: Some("8.5"),
        evidence: "imap4/pkgIndex.tcl",
    },
    TcllibModule {
        package: "inifile",
        trains: &["0.3.3"],
        core_floor: Some("8.5"),
        evidence: "inifile/pkgIndex.tcl",
    },
    TcllibModule {
        package: "interp",
        trains: &["0.1.3"],
        core_floor: Some("8.5"),
        evidence: "interp/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ip",
        trains: &["1.5.1"],
        core_floor: Some("8.5"),
        evidence: "dns/pkgIndex.tcl",
    },
    TcllibModule {
        package: "irc",
        trains: &["0.8.0"],
        core_floor: Some("8.6"),
        evidence: "irc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "javascript",
        trains: &["1.0.3"],
        core_floor: Some("8.5"),
        evidence: "javascript/pkgIndex.tcl",
    },
    TcllibModule {
        package: "jpeg",
        trains: &["0.7"],
        core_floor: Some("8.5"),
        evidence: "jpeg/pkgIndex.tcl",
    },
    TcllibModule {
        package: "json",
        trains: &["1.3.6"],
        core_floor: Some("8.5"),
        evidence: "json/pkgIndex.tcl",
    },
    TcllibModule {
        package: "lambda",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "lambda/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ldap",
        trains: &["1.10.2"],
        core_floor: Some("8.5"),
        evidence: "ldap/pkgIndex.tcl",
    },
    TcllibModule {
        package: "log",
        trains: &["1.5"],
        core_floor: Some("8.5"),
        evidence: "log/pkgIndex.tcl",
    },
    TcllibModule {
        package: "logger",
        trains: &["0.9.5"],
        core_floor: Some("8.5"),
        evidence: "log/pkgIndex.tcl",
    },
    TcllibModule {
        package: "logger::utils",
        trains: &["1.3.2"],
        core_floor: Some("8.5"),
        evidence: "log/pkgIndex.tcl",
    },
    TcllibModule {
        package: "map::slippy",
        trains: &["0.10"],
        core_floor: Some("8.6"),
        evidence: "map/pkgIndex.tcl",
    },
    TcllibModule {
        package: "mapproj",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "mapproj/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math",
        trains: &["1.2.6"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::PCA",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::bignum",
        trains: &["3.1.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::calculus",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::combinatorics",
        trains: &["2.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::complexnumbers",
        trains: &["1.0.3"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::constants",
        trains: &["1.0.4"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::decimal",
        trains: &["1.0.5"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::exact",
        trains: &["1.0.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::figurate",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::filters",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::fourier",
        trains: &["1.0.3"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::fuzzy",
        trains: &["0.2.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::geometry",
        trains: &["1.4.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::interpolate",
        trains: &["1.1.4"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::linearalgebra",
        trains: &["1.1.7"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::optimize",
        trains: &["1.0.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::polynomials",
        trains: &["1.0.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::probopt",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::rationalfunctions",
        trains: &["1.0.2"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::roman",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::special",
        trains: &["0.5.4"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::statistics",
        trains: &["1.6.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "math::trig",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "math/pkgIndex.tcl",
    },
    TcllibModule {
        package: "md4",
        trains: &["1.0.8"],
        core_floor: Some("8.5"),
        evidence: "md4/pkgIndex.tcl",
    },
    TcllibModule {
        // Two trains in one index: `md5x.tcl` (2.0.9, the Trf-accelerated
        // rewrite) and `md5.tcl` (1.4.6, the pure-Tcl original).
        package: "md5",
        trains: &["2.0.9", "1.4.6"],
        core_floor: Some("8.5"),
        evidence: "md5/pkgIndex.tcl",
    },
    TcllibModule {
        package: "md5crypt",
        trains: &["1.2.0"],
        core_floor: Some("8.5"),
        evidence: "md5crypt/pkgIndex.tcl",
    },
    TcllibModule {
        package: "mime",
        trains: &["1.7.2"],
        core_floor: Some("8.5"),
        evidence: "mime/pkgIndex.tcl",
    },
    TcllibModule {
        package: "mkdoc",
        trains: &["0.7.2"],
        core_floor: Some("8.6"),
        evidence: "mkdoc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "multiplexer",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "multiplexer/pkgIndex.tcl",
    },
    TcllibModule {
        package: "nameserv",
        trains: &["0.4.3"],
        core_floor: Some("8.5"),
        evidence: "nns/pkgIndex.tcl",
    },
    TcllibModule {
        package: "nameserv::common",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "nns/pkgIndex.tcl",
    },
    TcllibModule {
        package: "nameserv::server",
        trains: &["0.3.3"],
        core_floor: Some("8.5"),
        evidence: "nns/pkgIndex.tcl",
    },
    TcllibModule {
        package: "namespacex",
        trains: &["0.4"],
        core_floor: Some("8.5"),
        evidence: "namespacex/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ncgi",
        trains: &["1.4.6"],
        core_floor: Some("8.5"),
        evidence: "ncgi/pkgIndex.tcl",
    },
    TcllibModule {
        package: "nettool",
        trains: &["0.5.4"],
        core_floor: Some("8.5"),
        evidence: "nettool/pkgIndex.tcl",
    },
    TcllibModule {
        package: "nmea",
        trains: &["1.1.0"],
        core_floor: Some("8.5"),
        evidence: "nmea/pkgIndex.tcl",
    },
    TcllibModule {
        package: "oauth",
        trains: &["1.0.4"],
        core_floor: Some("8.5"),
        evidence: "oauth/pkgIndex.tcl",
    },
    TcllibModule {
        // The `ooutil` **directory** provides the `oo::util` package —
        // one of the two identity mismatches this table exists to record.
        // Its three commands (`link`, `mymethod`, `classvariable`) are
        // filed under `commands::tcl`, because each also has a core 9.0
        // route.
        package: "oo::util",
        trains: &["1.2.3"],
        core_floor: Some("8.5"),
        evidence: "ooutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "otp",
        trains: &["1.1.0"],
        core_floor: Some("8.5"),
        evidence: "otp/pkgIndex.tcl",
    },
    TcllibModule {
        package: "page::pluginmgr",
        trains: &["0.3"],
        core_floor: None,
        evidence: "page/pkgIndex.tcl",
    },
    TcllibModule {
        package: "page::util::peg",
        trains: &["0.2"],
        core_floor: None,
        evidence: "page/pkgIndex.tcl",
    },
    TcllibModule {
        package: "page::util::quote",
        trains: &["0.2"],
        core_floor: None,
        evidence: "page/pkgIndex.tcl",
    },
    TcllibModule {
        package: "picoirc",
        trains: &["0.14.0"],
        core_floor: Some("8.6"),
        evidence: "irc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "pki",
        trains: &["0.22"],
        core_floor: Some("8.6"),
        evidence: "pki/pkgIndex.tcl",
    },
    TcllibModule {
        package: "png",
        trains: &["0.4.1"],
        core_floor: Some("8.5"),
        evidence: "png/pkgIndex.tcl",
    },
    TcllibModule {
        package: "pop3",
        trains: &["1.11"],
        core_floor: Some("8.5"),
        evidence: "pop3/pkgIndex.tcl",
    },
    TcllibModule {
        // No head guard in the index; `processman.tcl` states the floor
        // itself. Its `cron 2.0` dependency needs 8.6 transitively — a
        // floor the model has no field for (P5's recorded limit).
        package: "processman",
        trains: &["0.8"],
        core_floor: Some("8.5"),
        evidence: "processman/processman.tcl",
    },
    TcllibModule {
        package: "profiler",
        trains: &["0.7"],
        core_floor: Some("8.5"),
        evidence: "profiler/pkgIndex.tcl",
    },
    TcllibModule {
        package: "pt::peg",
        trains: &["1.1.1"],
        core_floor: Some("8.5"),
        evidence: "pt/pkgIndex.tcl",
    },
    TcllibModule {
        package: "rc4",
        trains: &["1.2.0"],
        core_floor: Some("8.5"),
        evidence: "rc4/pkgIndex.tcl",
    },
    TcllibModule {
        package: "rcs",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "rcs/pkgIndex.tcl",
    },
    TcllibModule {
        package: "report",
        trains: &["0.5"],
        core_floor: Some("8.5"),
        evidence: "report/pkgIndex.tcl",
    },
    TcllibModule {
        package: "rest",
        trains: &["1.7"],
        core_floor: Some("8.5"),
        evidence: "rest/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ripemd128",
        trains: &["1.0.6"],
        core_floor: Some("8.5"),
        evidence: "ripemd/pkgIndex.tcl",
    },
    TcllibModule {
        package: "ripemd160",
        trains: &["1.0.7"],
        core_floor: Some("8.5"),
        evidence: "ripemd/pkgIndex.tcl",
    },
    TcllibModule {
        package: "sha1",
        trains: &["2.0.5", "1.1.2"],
        core_floor: Some("8.5"),
        evidence: "sha1/pkgIndex.tcl",
    },
    TcllibModule {
        // The `sha1` **directory** provides `sha256`, whose commands live
        // in the `::sha2` namespace. Neither the directory nor the
        // namespace is the `package require` name.
        package: "sha256",
        trains: &["1.0.6"],
        core_floor: Some("8.5"),
        evidence: "sha1/pkgIndex.tcl",
    },
    TcllibModule {
        package: "simulation::annealing",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "simulation/annealing.tcl",
    },
    TcllibModule {
        package: "simulation::montecarlo",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "simulation/montecarlo.tcl",
    },
    TcllibModule {
        package: "simulation::random",
        trains: &["0.5.0"],
        core_floor: Some("8.5"),
        evidence: "simulation/random.tcl",
    },
    TcllibModule {
        package: "smtp",
        trains: &["1.5.2"],
        core_floor: Some("8.5"),
        evidence: "mime/pkgIndex.tcl",
    },
    TcllibModule {
        package: "smtpd",
        trains: &["1.6"],
        core_floor: Some("8.5"),
        evidence: "smtpd/pkgIndex.tcl",
    },
    TcllibModule {
        // Two trains, and the 2.x one is itself guarded: the index offers
        // snit 2.3.4 only when the core satisfies `8.5 9`, while 1.4.3 is
        // offered unconditionally.
        package: "snit",
        trains: &["2.3.4", "1.4.3"],
        core_floor: Some("8.5"),
        evidence: "snit/pkgIndex.tcl",
    },
    TcllibModule {
        package: "soundex",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "soundex/pkgIndex.tcl",
    },
    TcllibModule {
        package: "stooop",
        trains: &["4.4.2"],
        core_floor: Some("8.5"),
        evidence: "stooop/pkgIndex.tcl",
    },
    TcllibModule {
        // No `package require Tcl` anywhere in the module — the sources
        // state no floor, so neither does this row.
        package: "stringprep",
        trains: &["1.0.3"],
        core_floor: None,
        evidence: "stringprep/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::disjointset",
        trains: &["1.2"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        // The second multi-train `struct` member; `struct::graph::op`
        // 0.11.4 requires the 2.x train specifically.
        package: "struct::graph",
        trains: &["2.4.4", "1.2.2"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::list",
        trains: &["1.9"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::map",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::matrix",
        trains: &["2.2"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::pool",
        trains: &["1.2.4"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::prioqueue",
        trains: &["1.5"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::queue",
        trains: &["1.4.6"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::set",
        trains: &["2.2.5"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "struct::stack",
        trains: &["1.5.4"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        // P5's flagship adversarial module: two trains whose walker APIs
        // are incompatible — 1.x takes `-command` with `%n`/`%a`/`%t`
        // placeholders, 2.x takes `loopvar script` and adds `walkproc`.
        package: "struct::tree",
        trains: &["2.1.3", "1.2.3"],
        core_floor: Some("8.5"),
        evidence: "struct/pkgIndex.tcl",
    },
    TcllibModule {
        package: "sum",
        trains: &["1.1.3"],
        core_floor: Some("8.5"),
        evidence: "crc/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tar",
        trains: &["0.13"],
        core_floor: Some("8.5"),
        evidence: "tar/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::cat",
        trains: &["1.0.4"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::facade",
        trains: &["1.0.2"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::fifo",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::fifo2",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::halfpipe",
        trains: &["1.0.3"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::memchan",
        trains: &["1.0.5"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::null",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::nullzero",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::random",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::std",
        trains: &["1.0.2"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::string",
        trains: &["1.0.4"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::textwindow",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::variable",
        trains: &["1.0.5"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::chan::zero",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::randomseed",
        trains: &["1.1"],
        core_floor: Some("8.5"),
        evidence: "virtchannel_base/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::adler32",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::base64",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::counter",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::crc32",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::hex",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::identity",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::limitsize",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::observe",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::otp",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::rot",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::spacer",
        trains: &["1.1"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tcl::transform::zlib",
        trains: &["1.0.2"],
        core_floor: Some("8.6"),
        evidence: "virtchannel_transform/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::ansi::code",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::ansi::code::attr",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::ansi::code::ctrl",
        trains: &["0.4"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::ansi::code::macros",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::ansi::ctrl::unix",
        trains: &["0.1.2"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::ansi::send",
        trains: &["0.3"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "term::send",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "term/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil",
        trains: &["0.10"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::adjust",
        trains: &["0.7.4"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::patch",
        trains: &["0.2"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::repeat",
        trains: &["0.8"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::split",
        trains: &["0.9"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::string",
        trains: &["0.9"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::tabify",
        trains: &["0.8"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::trim",
        trains: &["0.8"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "textutil::wcswidth",
        trains: &["35.3"],
        core_floor: Some("8.5"),
        evidence: "textutil/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tie",
        trains: &["1.3"],
        core_floor: Some("8.5"),
        evidence: "tie/pkgIndex.tcl",
    },
    TcllibModule {
        package: "tiff",
        trains: &["0.2.3"],
        core_floor: Some("8.5"),
        evidence: "tiff/pkgIndex.tcl",
    },
    TcllibModule {
        package: "time",
        trains: &["1.2.2"],
        core_floor: Some("8.5"),
        evidence: "ntp/pkgIndex.tcl",
    },
    TcllibModule {
        package: "uevent",
        trains: &["0.3.2"],
        core_floor: Some("8.5"),
        evidence: "uev/pkgIndex.tcl",
    },
    TcllibModule {
        // Shares `stringprep`'s index, and states no floor either.
        package: "unicode",
        trains: &["1.1.1"],
        core_floor: None,
        evidence: "stringprep/pkgIndex.tcl",
    },
    TcllibModule {
        package: "units",
        trains: &["2.2.3"],
        core_floor: Some("8.5"),
        evidence: "units/pkgIndex.tcl",
    },
    TcllibModule {
        package: "uri",
        trains: &["1.2.8"],
        core_floor: Some("8.5"),
        evidence: "uri/pkgIndex.tcl",
    },
    TcllibModule {
        package: "uuencode",
        trains: &["1.1.6"],
        core_floor: Some("8.5"),
        evidence: "base64/pkgIndex.tcl",
    },
    TcllibModule {
        package: "uuid",
        trains: &["1.0.9"],
        core_floor: Some("8.5"),
        evidence: "uuid/pkgIndex.tcl",
    },
    TcllibModule {
        package: "websocket",
        trains: &["1.6"],
        core_floor: Some("8.6"),
        evidence: "websocket/pkgIndex.tcl",
    },
    TcllibModule {
        package: "yaml",
        trains: &["0.4.2"],
        core_floor: Some("8.5"),
        evidence: "yaml/pkgIndex.tcl",
    },
    TcllibModule {
        package: "yencode",
        trains: &["1.1.4"],
        core_floor: Some("8.5"),
        evidence: "base64/pkgIndex.tcl",
    },
    TcllibModule {
        package: "zipfile::decode",
        trains: &["0.10.1"],
        core_floor: Some("8.5"),
        evidence: "zip/pkgIndex.tcl",
    },
];

/// The names the compiled catalogue uses as a tcllib "package" that
/// tcllib 2.0 **does not provide** — the identity census's residue,
/// recorded rather than silently renamed.
///
/// Each row is `(catalogue name, what the sources actually say)`. Two
/// kinds live here:
///
/// - **Namespace prefixes.** `tcl::chan`, `tcl::transform`,
///   `fileutil::magic`, `grammar::me`, `page::util`, `page::util::norm`,
///   `bench::out` and `pt` are the *common prefix* of a family of real
///   packages (`tcl::chan::fifo` 1.1, `fileutil::magic::rt` 3.1, …), not
///   packages themselves. A `package require tcl::chan` fails; the
///   catalogue nonetheless files those commands under the prefix, so
///   W120 currently nags for a require that cannot be written.
/// - **Documentation identities.** `pt_export_api` and `pt_import_api`
///   are doctools *manpage* names for plugin interfaces
///   (`pt/pt_to_api.man`, `pt/pt_from_api.man`), never `package provide`d
///   at all; `tcl::combine` is a bare `proc` in
///   `virtchannel_base/randseed.tcl`, whose package is `tcl::randomseed`.
///
/// Fixing these means re-filing the affected commands under their real
/// providers, which is per-command surface work rather than identity
/// work, so P5 records the census and leaves the rows. The exhaustiveness
/// test `the_identity_census_is_closed` (in `commands::tcllib`) fails the
/// build if a *new* unbacked name appears, so the list can only shrink.
pub const UNBACKED_PACKAGE_NAMES: &[(&str, &str)] = &[
    (
        "bench::out",
        "prefix of bench::out::csv / bench::out::text 0.1.3",
    ),
    (
        "fileutil::magic",
        "prefix of fileutil::magic::rt 3.1 and siblings",
    ),
    ("grammar::me", "prefix of grammar::me::tcl 0.2 and siblings"),
    (
        "page::util",
        "prefix of page::util::peg / ::quote / ::norm::* 0.2",
    ),
    (
        "page::util::norm",
        "prefix of page::util::norm::peg / ::lemon 0.2",
    ),
    ("pt", "prefix of pt::ast 1.2, pt::peg 1.0 and siblings"),
    (
        "pt_export_api",
        "a doctools manpage identity (pt/pt_to_api.man)",
    ),
    (
        "pt_import_api",
        "a doctools manpage identity (pt/pt_from_api.man)",
    ),
    ("tcl::chan", "prefix of tcl::chan::fifo 1.1 and siblings"),
    (
        "tcl::combine",
        "a proc in virtchannel_base/randseed.tcl; package is tcl::randomseed",
    ),
    (
        "tcl::transform",
        "prefix of tcl::transform::hex 1.1 and siblings",
    ),
];

/// Whether `package` is a recorded [`UNBACKED_PACKAGE_NAMES`] row.
#[must_use]
pub fn is_unbacked_package_name(package: &str) -> bool {
    UNBACKED_PACKAGE_NAMES
        .iter()
        .any(|&(name, _)| name == package)
}

/// The modelled tcllib module named `package`, if the table carries one.
#[must_use]
pub fn tcllib_module(package: &str) -> Option<&'static TcllibModule> {
    TCLLIB_MODULES
        .binary_search_by(|module| module.package.cmp(package))
        .ok()
        .map(|index| &TCLLIB_MODULES[index])
}

/// The applicability [`VersionSet`] of `package`'s declarations — the
/// union of its trains, on **its own** package axis.
///
/// `None` for a package the table does not carry (every non-tcllib
/// provider), which keeps [`crate::model::surface::declarations_for_spec`]
/// on the full-axis fallback there. The set is never empty and never a
/// single point: a train is a `package require` requirement, so even a
/// one-train module spans `[V, major+1)`.
#[must_use]
pub fn module_version_set(package: &str) -> Option<VersionSet> {
    let module = tcllib_module(package)?;
    VersionSet::from_requirements(VersionAxisId::package(package), module.trains).ok()
}

/// The Tcl ladder bits `package`'s commands must be gated **out** of,
/// because the module's own `package require Tcl` line excludes them.
///
/// This is the §5.4 "package interplay" rule applied at ladder
/// granularity: tcllib 2.0's `csv` cannot load on Tcl 8.4 at all, so
/// offering `csv::split` under the `tcl8.4` environment over-reports
/// availability — the D5 "oldest never over-reports" rule. `None` when
/// the module states no floor, or names one at or below the ladder's
/// oldest release.
///
/// Deliberately a **subtraction on the core axis**, not a comparison
/// between axes: the module's *own* version lives on its own axis (I2),
/// and what is compared here is the module's declared Tcl requirement
/// against the Tcl ladder.
#[must_use]
pub fn core_floor_surface(package: &str) -> &'static [SpecSurface] {
    match tcllib_module(package).and_then(|module| module.core_floor) {
        Some("8.5") => SpecSurface::TCL85_PLUS,
        Some("8.6") => SpecSurface::TCL86_PLUS,
        Some("9.0") => SpecSurface::TCL90_PLUS,
        Some("9.1") => SpecSurface::TCL91,
        // "8.4" and anything the table does not spell: the whole ladder.
        _ => SpecSurface::ALL_TCL,
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tcl_dialect::model::Version;

    fn v(text: &str) -> Version {
        Version::parse(text).expect("test version")
    }

    #[test]
    fn table_is_sorted_and_unique() {
        for pair in TCLLIB_MODULES.windows(2) {
            assert!(
                pair[0].package < pair[1].package,
                "`{}` must sort before `{}` (binary search)",
                pair[0].package,
                pair[1].package,
            );
        }
    }

    #[test]
    fn every_row_is_well_formed() {
        for module in TCLLIB_MODULES {
            assert!(!module.trains.is_empty(), "{}", module.package);
            assert!(!module.evidence.is_empty(), "{}", module.package);
            let set = module_version_set(module.package)
                .unwrap_or_else(|| panic!("{} has a version set", module.package));
            assert_eq!(&VersionAxisId::package(module.package), set.axis());
            assert!(!set.is_empty(), "{}", module.package);
            for train in module.trains {
                assert!(
                    set.contains(&v(train)),
                    "{} must admit its own train {train}",
                    module.package,
                );
            }
            if let Some(floor) = module.core_floor {
                assert!(
                    matches!(floor, "8.4" | "8.5" | "8.6" | "9.0" | "9.1"),
                    "{}: `{floor}` is not a ladder release",
                    module.package,
                );
            }
        }
    }

    /// A train is a *requirement*, so a module's applicability is a range
    /// — never the single point its `pkgIndex.tcl` happens to ship. This
    /// is the deliverable's "fix any pinned-to-one-version rows" rule,
    /// stated as an invariant over the whole table.
    #[test]
    fn a_single_train_is_still_a_range() {
        let csv = module_version_set("csv").expect("csv");
        assert!(csv.contains(&v("0.10")));
        assert!(csv.contains(&v("0.99")), "the whole 0.x line, not a point");
        assert!(!csv.contains(&v("1.0")), "the next major is a new train");
        assert!(!csv.contains(&v("0.9.9")), "below the shipped version");
    }

    /// The parallel-train modules: two disjoint ranges, not one span.
    #[test]
    fn parallel_trains_stay_disjoint_ranges() {
        for (package, old, new, between) in [
            ("md5", "1.4.6", "2.0.9", "1.9.0"),
            ("sha1", "1.1.2", "2.0.5", "1.5"),
            ("snit", "1.4.3", "2.3.4", "1.9"),
            ("struct::tree", "1.2.3", "2.1.3", "1.9"),
            ("struct::graph", "1.2.2", "2.4.4", "1.9"),
        ] {
            let set = module_version_set(package).expect(package);
            assert_eq!(set.ranges().len(), 2, "{package} has two trains");
            assert!(set.contains(&v(old)), "{package} {old}");
            assert!(set.contains(&v(new)), "{package} {new}");
            assert!(set.contains(&v(between)), "{package} {between} (1.x line)");
            assert!(!set.contains(&v("0.9")), "{package} below both trains");
        }
        // …and the gap between the 1.x line's top and the 2.x train's
        // start is genuinely outside the set.
        let tree = module_version_set("struct::tree").expect("struct::tree");
        assert!(!tree.contains(&v("2.0")), "2.0 predates the 2.1.3 train");
    }

    /// Package identity is the `package require` spelling, not the module
    /// directory and not the command namespace — the two rows that make
    /// the distinction load-bearing.
    #[test]
    fn identity_is_the_require_spelling_not_the_directory() {
        assert!(tcllib_module("oo::util").is_some());
        assert!(tcllib_module("ooutil").is_none(), "the directory name");
        assert!(tcllib_module("sha256").is_some());
        assert!(tcllib_module("sha2").is_none(), "the ::sha2 namespace");
        assert_eq!(
            tcllib_module("sha256").map(|m| m.evidence),
            Some("sha1/pkgIndex.tcl")
        );
    }

    /// Every module sits on **its own** axis, so no two modules' sets are
    /// comparable (invariant I2) and none of them is the core axis.
    #[test]
    fn each_module_owns_its_axis() {
        let tree = module_version_set("struct::tree").expect("struct::tree");
        let list = module_version_set("struct::list").expect("struct::list");
        assert!(
            tree.intersect(&list).is_err(),
            "different package axes must refuse comparison (I2)"
        );
        let core = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
        for module in TCLLIB_MODULES {
            assert_ne!(
                &core,
                module_version_set(module.package)
                    .expect(module.package)
                    .axis(),
                "{} must not sit on the core axis",
                module.package,
            );
        }
    }

    /// The placement half of the deliverable: a tcllib module is
    /// **hosted**, never ambient. No compiled environment ships one as
    /// part of its runtime, so none is closed-world and none is
    /// placement-gated — its commands stay leniently visible with W120
    /// owning the nag, and its floor comes from `package require`.
    #[test]
    fn hosted_modules_are_never_ambient() {
        for module in TCLLIB_MODULES {
            assert!(
                !crate::model::surface::is_placement_gated_package(module.package),
                "{} must not be placement-gated (it is hosted, not ambient)",
                module.package,
            );
            assert!(
                !crate::model::surface::is_closed_world_package(module.package),
                "{} must not be a closed-world vendor runtime",
                module.package,
            );
        }
    }
}
