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

//! `tcllib` command specifications.

#![allow(non_snake_case)]

mod base64__decode;
mod base64__encode;
mod cmdline__getargv0;
mod cmdline__getfiles;
mod cmdline__getknownopt;
mod cmdline__getknownoptions;
mod cmdline__getopt;
mod cmdline__getoptions;
mod cmdline__typedgetopt;
mod cmdline__typedgetoptions;
mod cmdline__typedusage;
mod cmdline__usage;
mod control__assert;
mod control__control;
mod control__do;
mod control__no_op;
mod csv__iscomplete;
mod csv__join;
mod csv__joinlist;
mod csv__joinmatrix;
mod csv__read2matrix;
mod csv__read2queue;
mod csv__report;
mod csv__split;
mod csv__split2matrix;
mod csv__split2queue;
mod csv__writematrix;
mod csv__writequeue;
mod dns__address;
mod dns__cleanup;
mod dns__cname;
mod dns__configure;
mod dns__dump;
mod dns__error;
mod dns__errorcode;
mod dns__name;
mod dns__reset;
mod dns__resolve;
mod dns__result;
mod dns__status;
mod dns__wait;
mod fileutil__appendtofile;
mod fileutil__cat;
mod fileutil__filetype;
mod fileutil__find;
mod fileutil__findbypattern;
mod fileutil__foreachline;
mod fileutil__fullnormalize;
mod fileutil__grep;
mod fileutil__insertintofile;
mod fileutil__install;
mod fileutil__jail;
mod fileutil__lexnormalize;
mod fileutil__maketempdir;
mod fileutil__relative;
mod fileutil__relativeurl;
mod fileutil__removefromfile;
mod fileutil__replaceinfile;
mod fileutil__stripn;
mod fileutil__strippwd;
mod fileutil__tempdir;
mod fileutil__tempdirreset;
mod fileutil__tempfile;
mod fileutil__test;
mod fileutil__touch;
mod fileutil__traverse;
mod fileutil__updateinplace;
mod fileutil__writefile;
mod html__html_entities;
mod html__tagstrip;
mod ip__collapse;
mod ip__contract;
mod ip__equal;
mod ip__is;
mod ip__mask;
mod ip__normalize;
mod ip__prefix;
mod ip__subtract;
mod ip__type;
mod ip__version;
mod json__dict2json;
mod json__json2dict;
mod json__list2json;
mod json__many_json2dict;
mod json__string2json;
mod json__validate;
mod logger__disable;
mod logger__enable;
mod logger__import;
mod logger__init;
mod logger__initnamespace;
mod logger__levels;
mod logger__servicecmd;
mod logger__services;
mod logger__setlevel;
mod logger__walk;
mod math__statistics__analyse_kruskal_wallis;
mod math__statistics__autocorr;
mod math__statistics__basic_stats;
mod math__statistics__control_rchart;
mod math__statistics__control_xbar;
mod math__statistics__corr;
mod math__statistics__crosscorr;
mod math__statistics__filter;
mod math__statistics__group_rank;
mod math__statistics__histogram;
mod math__statistics__histogram_alt;
mod math__statistics__interval_mean_stdev;
mod math__statistics__lillieforsfit;
mod math__statistics__linear_model;
mod math__statistics__linear_residuals;
mod math__statistics__map;
mod math__statistics__max;
mod math__statistics__mean;
mod math__statistics__mean_histogram_limits;
mod math__statistics__median;
mod math__statistics__min;
mod math__statistics__minmax_histogram_limits;
mod math__statistics__number;
mod math__statistics__print_2x2;
mod math__statistics__pstdev;
mod math__statistics__pvar;
mod math__statistics__quantiles;
mod math__statistics__samplescount;
mod math__statistics__spearman_rank;
mod math__statistics__spearman_rank_extended;
mod math__statistics__stdev;
mod math__statistics__t_test_mean;
mod math__statistics__test_2x2;
mod math__statistics__test_anova_f;
mod math__statistics__test_duckworth;
mod math__statistics__test_dunnett;
mod math__statistics__test_kruskal_wallis;
mod math__statistics__test_normal;
mod math__statistics__test_rchart;
mod math__statistics__test_tukey_range;
mod math__statistics__test_wilcoxon;
mod math__statistics__test_xbar;
mod math__statistics__var;
mod md5__md5;
mod mime__buildmessage;
mod mime__copymessage;
mod mime__field_decode;
mod mime__finalize;
mod mime__getbody;
mod mime__getcontenttype;
mod mime__getheader;
mod mime__getproperty;
mod mime__getsize;
mod mime__gettransferencoding;
mod mime__initialize;
mod mime__mapencoding;
mod mime__parseaddress;
mod mime__parsedatetime;
mod mime__reversemapencoding;
mod mime__setheader;
mod mime__uniqueid;
mod mime__word_decode;
mod mime__word_encode;
mod report__defstyle;
mod report__report;
mod report__rmstyle;
mod report__stylearguments;
mod report__stylebody;
mod report__styles;
mod sha1__sha1;
mod sha2__sha256;
mod smtp__sendmessage;
mod snit__compile;
mod snit__macro;
mod snit__method;
mod snit__type;
mod snit__typemethod;
mod snit__widget;
mod snit__widgetadaptor;
mod stooop__class;
mod struct__list;
mod struct__queue;
mod struct__set;
mod struct__stack;
mod textutil__adjust;
mod textutil__blank;
mod textutil__cap;
mod textutil__capeachword;
mod textutil__chop;
mod textutil__indent;
mod textutil__longestcommonprefix;
mod textutil__longestcommonprefixlist;
mod textutil__splitn;
mod textutil__splitx;
mod textutil__strrepeat;
mod textutil__tabify;
mod textutil__tabify2;
mod textutil__tail;
mod textutil__trim;
mod textutil__trimemptyheading;
mod textutil__trimleft;
mod textutil__trimprefix;
mod textutil__trimright;
mod textutil__uncap;
mod textutil__undent;
mod textutil__untabify;
mod textutil__untabify2;
mod uri__canonicalize;
mod uri__geturl;
mod uri__isrelative;
mod uri__join;
mod uri__register;
mod uri__resolve;
mod uri__setquirkoption;
mod uri__split;
mod uuid__uuid;
mod yaml__dict2yaml;
mod yaml__huddle2yaml;
mod yaml__list2yaml;
mod yaml__setoptions;
mod yaml__yaml2dict;
mod yaml__yaml2huddle;

// Grouped crypto / hash / checksum / encoding packages (one module per
// package family, each exposing a `specs()` builder).
mod base32;
mod channels_docs;
mod ciphers;
mod clients;
mod crc;
mod data_ext;
mod data_structures;
mod data_util;
mod encoding_pkgs;
mod ensembles;
mod functional;
mod image_pkgs;
mod math_core;
mod math_ext;
mod md4;
mod md5crypt;
mod misc_ext;
mod misc_pkgs;
mod namespacex;
mod otp;
mod rc4;
mod ripemd;
mod term_text;
mod text_misc;
mod web_asn;

use crate::model::tcllib::core_floor_surface;
#[cfg(test)]
use crate::model::tcllib::tcllib_module;
use crate::spec::CommandSpec;
use tcl_dialect::model::SpecSurface;

/// Return all `tcllib` command specifications.
///
/// The flat list is assembled from per-namespace sub-builders (grouped by
/// tcllib package family) so no single function carries the whole table,
/// then each spec's `required_package` is derived from its namespace.
/// Concatenation order is preserved exactly — the registry-port tests
/// assert the resulting set is identical.
#[must_use]
pub fn tcllib_command_specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(base64_cmdline_control_csv_dns_specs());
    specs.extend(fileutil_html_ip_json_specs());
    specs.extend(logger_math_specs());
    specs.extend(md5_mime_snit_struct_specs());
    specs.extend(textutil_uri_yaml_specs());
    specs.extend(crypto_encoding_specs());
    // Every tcllib command is gated on its providing package.
    // Derive `required_package` from the command's namespace so
    // W120 (missing-package-require) and package-gated
    // completion fire for tcllib commands used without the
    // corresponding `package require`.
    // A tcllib command is available where its package's `package require`
    // can be honoured, at the Tcl releases the package's own
    // `package require Tcl` floor admits. The restricted F5 embedded
    // dialects (iRules, iApps) run in TMM with a fixed command set and can
    // host no package at all, so they refuse these commands through the
    // `required_package` gate — the environment answers it, rather than
    // every spec carrying a subtraction (§3.3).
    for spec in &mut specs {
        if spec.required_package.is_none() {
            spec.required_package = tcllib_required_package(spec.name);
        }
        if spec.surface.is_none() {
            spec.surface = Some(
                spec.owning_package()
                    .map_or(SpecSurface::ALL_TCL, core_floor_surface),
            );
        }
    }
    specs
}

/// `base64::*`, `cmdline::*`, `control::*`, `csv::*`, and `dns::*`.
fn base64_cmdline_control_csv_dns_specs() -> Vec<CommandSpec> {
    vec![
        base64__decode::spec(),
        base64__encode::spec(),
        cmdline__getargv0::spec(),
        cmdline__getfiles::spec(),
        cmdline__getknownopt::spec(),
        cmdline__getknownoptions::spec(),
        cmdline__getopt::spec(),
        cmdline__getoptions::spec(),
        cmdline__typedgetopt::spec(),
        cmdline__typedgetoptions::spec(),
        cmdline__typedusage::spec(),
        cmdline__usage::spec(),
        control__assert::spec(),
        control__control::spec(),
        control__do::spec(),
        control__no_op::spec(),
        csv__iscomplete::spec(),
        csv__join::spec(),
        csv__joinlist::spec(),
        csv__joinmatrix::spec(),
        csv__read2matrix::spec(),
        csv__read2queue::spec(),
        csv__report::spec(),
        csv__split::spec(),
        csv__split2matrix::spec(),
        csv__split2queue::spec(),
        csv__writematrix::spec(),
        csv__writequeue::spec(),
        dns__address::spec(),
        dns__cleanup::spec(),
        dns__cname::spec(),
        dns__configure::spec(),
        dns__dump::spec(),
        dns__error::spec(),
        dns__errorcode::spec(),
        dns__name::spec(),
        dns__reset::spec(),
        dns__resolve::spec(),
        dns__result::spec(),
        dns__status::spec(),
        dns__wait::spec(),
    ]
}

/// `fileutil::*`, `html::*`, `ip::*`, and `json::*`.
fn fileutil_html_ip_json_specs() -> Vec<CommandSpec> {
    vec![
        fileutil__appendtofile::spec(),
        fileutil__cat::spec(),
        fileutil__filetype::spec(),
        fileutil__find::spec(),
        fileutil__findbypattern::spec(),
        fileutil__foreachline::spec(),
        fileutil__fullnormalize::spec(),
        fileutil__grep::spec(),
        fileutil__insertintofile::spec(),
        fileutil__install::spec(),
        fileutil__jail::spec(),
        fileutil__lexnormalize::spec(),
        fileutil__maketempdir::spec(),
        fileutil__relative::spec(),
        fileutil__relativeurl::spec(),
        fileutil__removefromfile::spec(),
        fileutil__replaceinfile::spec(),
        fileutil__stripn::spec(),
        fileutil__strippwd::spec(),
        fileutil__tempdir::spec(),
        fileutil__tempdirreset::spec(),
        fileutil__tempfile::spec(),
        fileutil__test::spec(),
        fileutil__traverse::spec(),
        fileutil__touch::spec(),
        fileutil__updateinplace::spec(),
        fileutil__writefile::spec(),
        html__html_entities::spec(),
        html__tagstrip::spec(),
        ip__collapse::spec(),
        ip__contract::spec(),
        ip__equal::spec(),
        ip__is::spec(),
        ip__mask::spec(),
        ip__normalize::spec(),
        ip__prefix::spec(),
        ip__subtract::spec(),
        ip__type::spec(),
        ip__version::spec(),
        json__dict2json::spec(),
        json__json2dict::spec(),
        json__list2json::spec(),
        json__many_json2dict::spec(),
        json__string2json::spec(),
        json__validate::spec(),
    ]
}

/// `logger::*` and the large `math::*` family.
fn logger_math_specs() -> Vec<CommandSpec> {
    vec![
        logger__disable::spec(),
        logger__enable::spec(),
        logger__import::spec(),
        logger__init::spec(),
        logger__initnamespace::spec(),
        logger__levels::spec(),
        logger__servicecmd::spec(),
        logger__services::spec(),
        logger__setlevel::spec(),
        logger__walk::spec(),
        math__statistics__analyse_kruskal_wallis::spec(),
        math__statistics__autocorr::spec(),
        math__statistics__basic_stats::spec(),
        math__statistics__control_rchart::spec(),
        math__statistics__control_xbar::spec(),
        math__statistics__corr::spec(),
        math__statistics__crosscorr::spec(),
        math__statistics__filter::spec(),
        math__statistics__group_rank::spec(),
        math__statistics__histogram::spec(),
        math__statistics__histogram_alt::spec(),
        math__statistics__interval_mean_stdev::spec(),
        math__statistics__lillieforsfit::spec(),
        math__statistics__linear_model::spec(),
        math__statistics__linear_residuals::spec(),
        math__statistics__map::spec(),
        math__statistics__max::spec(),
        math__statistics__mean::spec(),
        math__statistics__mean_histogram_limits::spec(),
        math__statistics__median::spec(),
        math__statistics__min::spec(),
        math__statistics__minmax_histogram_limits::spec(),
        math__statistics__number::spec(),
        math__statistics__print_2x2::spec(),
        math__statistics__pstdev::spec(),
        math__statistics__pvar::spec(),
        math__statistics__quantiles::spec(),
        math__statistics__samplescount::spec(),
        math__statistics__spearman_rank::spec(),
        math__statistics__spearman_rank_extended::spec(),
        math__statistics__stdev::spec(),
        math__statistics__t_test_mean::spec(),
        math__statistics__test_2x2::spec(),
        math__statistics__test_anova_f::spec(),
        math__statistics__test_duckworth::spec(),
        math__statistics__test_dunnett::spec(),
        math__statistics__test_kruskal_wallis::spec(),
        math__statistics__test_normal::spec(),
        math__statistics__test_rchart::spec(),
        math__statistics__test_tukey_range::spec(),
        math__statistics__test_wilcoxon::spec(),
        math__statistics__test_xbar::spec(),
        math__statistics__var::spec(),
    ]
}

/// `md5`, `mime::*`, `sha1`/`sha2`, `smtp`, `snit::*`, and `struct::*`.
fn md5_mime_snit_struct_specs() -> Vec<CommandSpec> {
    vec![
        md5__md5::spec(),
        mime__buildmessage::spec(),
        mime__copymessage::spec(),
        mime__field_decode::spec(),
        mime__finalize::spec(),
        mime__getbody::spec(),
        mime__getcontenttype::spec(),
        mime__getheader::spec(),
        mime__getproperty::spec(),
        mime__getsize::spec(),
        mime__gettransferencoding::spec(),
        mime__initialize::spec(),
        mime__mapencoding::spec(),
        mime__parseaddress::spec(),
        mime__parsedatetime::spec(),
        mime__reversemapencoding::spec(),
        mime__setheader::spec(),
        mime__uniqueid::spec(),
        mime__word_decode::spec(),
        mime__word_encode::spec(),
        report__defstyle::spec(),
        report__report::spec(),
        report__rmstyle::spec(),
        report__stylearguments::spec(),
        report__stylebody::spec(),
        report__styles::spec(),
        sha1__sha1::spec(),
        sha2__sha256::spec(),
        smtp__sendmessage::spec(),
        snit__compile::spec(),
        snit__macro::spec(),
        snit__method::spec(),
        snit__type::spec(),
        snit__typemethod::spec(),
        snit__widget::spec(),
        snit__widgetadaptor::spec(),
        stooop__class::spec(),
        struct__list::spec(),
        struct__queue::spec(),
        struct__set::spec(),
        struct__stack::spec(),
    ]
}

/// `textutil::*`, `uri::*`, `uuid`, and `yaml::*`.
fn textutil_uri_yaml_specs() -> Vec<CommandSpec> {
    vec![
        textutil__adjust::spec(),
        textutil__blank::spec(),
        textutil__cap::spec(),
        textutil__capeachword::spec(),
        textutil__chop::spec(),
        textutil__indent::spec(),
        textutil__longestcommonprefix::spec(),
        textutil__longestcommonprefixlist::spec(),
        textutil__splitn::spec(),
        textutil__splitx::spec(),
        textutil__strrepeat::spec(),
        textutil__tabify::spec(),
        textutil__tabify2::spec(),
        textutil__tail::spec(),
        textutil__trim::spec(),
        textutil__trimemptyheading::spec(),
        textutil__trimleft::spec(),
        textutil__trimprefix::spec(),
        textutil__trimright::spec(),
        textutil__uncap::spec(),
        textutil__undent::spec(),
        textutil__untabify::spec(),
        textutil__untabify2::spec(),
        uri__canonicalize::spec(),
        uri__geturl::spec(),
        uri__isrelative::spec(),
        uri__join::spec(),
        uri__register::spec(),
        uri__resolve::spec(),
        uri__setquirkoption::spec(),
        uri__split::spec(),
        uuid__uuid::spec(),
        yaml__dict2yaml::spec(),
        yaml__huddle2yaml::spec(),
        yaml__list2yaml::spec(),
        yaml__setoptions::spec(),
        yaml__yaml2dict::spec(),
        yaml__yaml2huddle::spec(),
    ]
}

/// The tcllib crypto / hash / checksum / encoding packages: `md4`,
/// `md5crypt`, `ripemd128`/`ripemd160`, the CRC family
/// (`crc16`/`crc32`/`cksum`/`sum`), the block ciphers (`aes`/`blowfish`/
/// `des`), `rc4`, `otp`, and `base32`/`base32::hex`.  Each command carries its
/// own `required_package` (the package name often differs from the leading
/// namespace segment — e.g. `crc::crc16` is provided by `crc16`, `DES::des`
/// by `des`), so these are exempt from the namespace-derivation fallback.
fn crypto_encoding_specs() -> Vec<CommandSpec> {
    let mut specs = vec![md4::md4(), md4::hmac()];
    specs.extend(md4::incremental());
    specs.extend(md5crypt::specs());
    specs.extend(ripemd::specs());
    specs.extend(crc::specs());
    specs.extend(ciphers::specs());
    specs.extend(rc4::specs());
    specs.extend(otp::specs());
    specs.extend(base32::specs());
    specs.extend(encoding_pkgs::specs());
    specs.extend(text_misc::specs());
    specs.extend(data_util::specs());
    specs.extend(math_core::specs());
    specs.extend(functional::specs());
    specs.extend(web_asn::specs());
    specs.extend(clients::specs());
    specs.extend(misc_pkgs::specs());
    specs.extend(image_pkgs::specs());
    specs.extend(ensembles::specs());
    specs.extend(term_text::specs());
    specs.extend(data_structures::specs());
    specs.extend(data_ext::specs());
    specs.extend(channels_docs::specs());
    specs.push(namespacex::spec());
    specs.extend(math_ext::specs());
    specs.extend(misc_ext::specs());
    specs
}

/// Map a tcllib command name to the package that provides it.
///
/// Each module declares the package its commands belong to.
/// Most packages own a
/// flat namespace (`csv::split` → `csv`); `math::statistics`
/// is a two-level namespace; the `struct::*` ensembles are
/// each their own package (the command name *is* the package).
fn tcllib_required_package(name: &str) -> Option<&'static str> {
    // `struct::*` ensembles: the command name is the package.
    match name {
        "struct::list" => return Some("struct::list"),
        "struct::queue" => return Some("struct::queue"),
        "struct::set" => return Some("struct::set"),
        "struct::stack" => return Some("struct::stack"),
        _ => {}
    }
    // `math::statistics::*` → two-level package namespace.
    if name.starts_with("math::statistics::") {
        return Some("math::statistics");
    }
    // Otherwise the package is the leading namespace segment.
    match name.split("::").next()? {
        "base64" => Some("base64"),
        "cmdline" => Some("cmdline"),
        "control" => Some("control"),
        "csv" => Some("csv"),
        "dns" => Some("dns"),
        "fileutil" => Some("fileutil"),
        "html" => Some("html"),
        "ip" => Some("ip"),
        "json" => Some("json"),
        "logger" => Some("logger"),
        "md5" => Some("md5"),
        "mime" => Some("mime"),
        "sha1" => Some("sha1"),
        // The commands live in `::sha2` but the module the user writes
        // is `package require sha256` — `sha1/sha256.tcl` ends with
        // `package provide sha256 1.0.6`, and no `sha2` package exists
        // anywhere in tcllib 2.0. Namespace ≠ package identity (P5).
        "sha2" => Some("sha256"),
        "smtp" => Some("smtp"),
        "snit" => Some("snit"),
        "textutil" => Some("textutil"),
        "uri" => Some("uri"),
        "uuid" => Some("uuid"),
        "yaml" => Some("yaml"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, SpecSurface, SurfaceQuery, surface_admits};

    #[test]
    fn every_tcllib_spec_has_required_package() {
        // Invariant: every tcllib command is
        // package-gated.  A `None` here means a namespace the
        // `tcllib_required_package` table doesn't cover.
        for spec in tcllib_command_specs() {
            assert!(
                spec.required_package.is_some(),
                "tcllib command `{}` has no required_package",
                spec.name,
            );
        }
    }

    #[test]
    fn required_package_matches_namespace() {
        let specs = tcllib_command_specs();
        let pkg = |name: &str| {
            specs
                .iter()
                .find(|s| s.name == name)
                .and_then(|s| s.required_package)
        };
        // Flat namespace → leading segment.
        assert_eq!(pkg("csv::split"), Some("csv"));
        assert_eq!(pkg("base64::decode"), Some("base64"));
        assert_eq!(pkg("uuid::uuid"), Some("uuid"));
        // Two-level namespace.
        assert_eq!(pkg("math::statistics::mean"), Some("math::statistics"));
        // `struct::*` ensembles own their package name.
        assert_eq!(pkg("struct::list"), Some("struct::list"));
        assert_eq!(pkg("struct::queue"), Some("struct::queue"));
        // Versioned sha packages. `::sha2` is the *namespace*; the
        // package `tmp/tcllib-2.0/modules/sha1/sha256.tcl` provides is
        // `sha256`, and that is what a user writes (P5).
        assert_eq!(pkg("sha1::sha1"), Some("sha1"));
        assert_eq!(pkg("sha2::sha256"), Some("sha256"));
    }

    /// **P5's identity census.** Every package name the catalogue files a
    /// tcllib command under is either a module the tcllib 2.0 sources
    /// really provide ([`TCLLIB_MODULES`](crate::model::tcllib::TCLLIB_MODULES))
    /// or a *recorded* gap
    /// ([`UNBACKED_PACKAGE_NAMES`](crate::model::tcllib::UNBACKED_PACKAGE_NAMES)).
    /// A new name backed by neither fails
    /// the build, so the census can only shrink.
    #[test]
    fn the_identity_census_is_closed() {
        use crate::model::tcllib::{is_unbacked_package_name, tcllib_module};
        let mut unknown: Vec<&str> = tcllib_command_specs()
            .iter()
            .filter_map(CommandSpec::owning_package)
            .filter(|package| {
                tcllib_module(package).is_none() && !is_unbacked_package_name(package)
            })
            .collect();
        unknown.sort_unstable();
        unknown.dedup();
        assert!(
            unknown.is_empty(),
            "tcllib package names backed by neither the module table nor the \
             recorded-gap list: {unknown:?}",
        );
    }

    /// Every row of the module table is a name the catalogue actually
    /// uses — the census closes in both directions, so a module drifts
    /// out of the table when its last command leaves the catalogue.
    ///
    /// `oo::util` is the one row provided from outside this module: its
    /// three commands (`link`, `mymethod`, `classvariable`) are `TclOO`
    /// helpers filed under `commands::tcl`, because the same names have a
    /// core 9.0 route beside the tcllib one.
    #[test]
    fn the_module_table_carries_no_unused_rows() {
        let mut used: std::collections::HashSet<&str> = tcllib_command_specs()
            .iter()
            .filter_map(CommandSpec::owning_package)
            .collect();
        used.insert("oo::util");
        let unused: Vec<&str> = crate::model::tcllib::TCLLIB_MODULES
            .iter()
            .map(|module| module.package)
            .filter(|package| !used.contains(package))
            .collect();
        assert!(unused.is_empty(), "unused module rows: {unused:?}");
    }

    /// The Tcl-core floor is now read per module from the sources, so it
    /// covers the whole distribution rather than the two names the old
    /// `match` happened to list (P5).
    #[test]
    fn tcl85_plus_packages_are_gated_out_of_tcl84() {
        let specs = tcllib_command_specs();
        for spec in &specs {
            let Some(package) = spec.owning_package() else {
                continue;
            };
            let rows = spec.surface.expect("tcllib command carries a surface");
            let Some(floor) = tcllib_module(package).and_then(|module| module.core_floor) else {
                continue;
            };
            assert!(
                !surface_admits(rows, Some(&SurfaceQuery::core(Family::Tcl, "8.4")))
                    || floor == "8.4",
                "`{}` (package {package}) must not be offered below its declared Tcl floor",
                spec.name,
            );
            assert!(
                surface_admits(rows, Some(&SurfaceQuery::core(Family::Tcl, "9.0"))),
                "`{}` should remain available under tcl9.0",
                spec.name,
            );
        }
        // The two names the old hand-written table carried still hold …
        let gated: Vec<_> = specs
            .iter()
            .filter(|s| s.name.starts_with("report::") || s.name.starts_with("stooop::"))
            .collect();
        assert!(!gated.is_empty(), "expected report::/stooop:: commands");
        for spec in gated {
            let rows = spec.surface.expect("a tcllib command carries a surface");
            let at =
                |release| surface_admits(rows, Some(&SurfaceQuery::core(Family::Tcl, release)));
            assert!(!at("8.4"), "{}", spec.name);
            assert!(at("8.6"), "{}", spec.name);
        }
        // … and so do the ones it silently missed: `csv` carries the same
        // `8.5 9` guard, and the 8.6-floor modules lose 8.5 as well.
        let admits = |rows: &[SpecSurface], release: &str| {
            surface_admits(rows, Some(&SurfaceQuery::core(Family::Tcl, release)))
        };
        let floor_of = |name: &str| {
            specs
                .iter()
                .find(|s| s.name == name)
                .and_then(|s| s.surface)
                .expect("spec")
        };
        assert!(!admits(floor_of("csv::split"), "8.4"));
        assert!(admits(floor_of("csv::split"), "8.5"));
        assert!(!admits(floor_of("defer::defer"), "8.5"));
        assert!(admits(floor_of("defer::defer"), "8.6"));
    }

    #[test]
    fn default_registry_carries_tcllib_required_package() {
        // The package survives into the built registry so the
        // analyser's W120 emitter can read it.
        let reg = crate::registry::CommandRegistry::build_default();
        assert_eq!(
            reg.get("csv::split").and_then(|s| s.required_package),
            Some("csv"),
        );
    }

    #[test]
    fn crypto_encoding_packages_are_registered_with_their_package() {
        // The crypto / hash / checksum / encoding cluster carries the *provider*
        // package, which frequently differs from the leading namespace segment.
        let specs = tcllib_command_specs();
        let pkg = |name: &str| {
            specs
                .iter()
                .find(|s| s.name == name)
                .and_then(|s| s.required_package)
        };
        // Namespace segment == package.
        assert_eq!(pkg("md4::md4"), Some("md4"));
        assert_eq!(pkg("md4::hmac"), Some("md4"));
        assert_eq!(pkg("md5crypt::md5crypt"), Some("md5crypt"));
        assert_eq!(pkg("aes::aes"), Some("aes"));
        assert_eq!(pkg("blowfish::blowfish"), Some("blowfish"));
        assert_eq!(pkg("rc4::rc4"), Some("rc4"));
        assert_eq!(pkg("otp::otp-md5"), Some("otp"));
        // Namespace segment != package.
        assert_eq!(pkg("ripemd::ripemd128"), Some("ripemd128"));
        assert_eq!(pkg("ripemd::ripemd160"), Some("ripemd160"));
        assert_eq!(pkg("crc::crc16"), Some("crc16"));
        assert_eq!(pkg("crc::crc32"), Some("crc32"));
        assert_eq!(pkg("crc::cksum"), Some("cksum"));
        assert_eq!(pkg("crc::sum"), Some("sum"));
        assert_eq!(pkg("DES::des"), Some("des"));
        assert_eq!(pkg("base32::encode"), Some("base32"));
        assert_eq!(pkg("base32::hex::encode"), Some("base32::hex"));
        // The 18-variant crc16 family is fully present.
        assert_eq!(pkg("crc::crc-ccitt"), Some("crc16"));
        assert_eq!(pkg("crc::xmodem"), Some("crc16"));
        assert_eq!(pkg("crc::modbus"), Some("crc16"));
        // Binary-encoding and text packages.
        assert_eq!(pkg("ascii85::encode"), Some("ascii85"));
        assert_eq!(pkg("uuencode::uuencode"), Some("uuencode"));
        assert_eq!(pkg("yencode::ydecode"), Some("yencode"));
        assert_eq!(pkg("soundex::knuth"), Some("soundex"));
        assert_eq!(pkg("stringprep::compare"), Some("stringprep"));
        assert_eq!(pkg("unicode::normalize"), Some("unicode"));
        // Data / utility packages (namespace != package for inifile).
        assert_eq!(pkg("ini::open"), Some("inifile"));
        assert_eq!(pkg("ini::commentchar"), Some("inifile"));
        assert_eq!(pkg("units::convert"), Some("units"));
        assert_eq!(pkg("counter::start"), Some("counter"));
        // math core packages (each math sub-namespace is its own package).
        assert_eq!(pkg("math::mean"), Some("math"));
        assert_eq!(pkg("math::random"), Some("math"));
        assert_eq!(pkg("math::fuzzy::teq"), Some("math::fuzzy"));
        assert_eq!(pkg("math::roman::toroman"), Some("math::roman"));
        assert_eq!(pkg("math::constants::constants"), Some("math::constants"));
        // Functional / scoping packages.
        assert_eq!(pkg("lambda"), Some("lambda"));
        assert_eq!(pkg("defer::defer"), Some("defer"));
        assert_eq!(pkg("tie::tie"), Some("tie"));
        assert_eq!(pkg("base32::core::define"), Some("base32::core"));
        // Web / protocol packages.
        assert_eq!(pkg("asn::asnInteger"), Some("asn"));
        assert_eq!(pkg("asn::asnGetSequence"), Some("asn"));
        assert_eq!(pkg("ncgi::parse"), Some("ncgi"));
        assert_eq!(pkg("htmlparse::parse"), Some("htmlparse"));
        // Ensemble + flat packages from the later batches.
        assert_eq!(pkg("generator"), Some("generator"));
        assert_eq!(pkg("debug"), Some("debug"));
        assert_eq!(pkg("hook"), Some("hook"));
        assert_eq!(pkg("websocket::open"), Some("websocket"));
    }

    #[test]
    fn generator_ensemble_has_subcommands() {
        // `generator` dispatches on a sub-command word (define/yield/foreach/…).
        let specs = tcllib_command_specs();
        let gen_cmd = specs
            .iter()
            .find(|s| s.name == "generator")
            .expect("generator registered");
        let names: Vec<&str> = gen_cmd.subcommands.iter().map(|s| s.name).collect();
        assert!(
            names.contains(&"define"),
            "generator has a define subcommand"
        );
        assert!(names.contains(&"yield"), "generator has a yield subcommand");
        assert!(
            gen_cmd.subcommands.len() > 20,
            "generator has many subcommands"
        );
    }

    #[test]
    fn lambda_body_and_param_roles_recurse() {
        // `lambda arguments body` — the argument list is a param list and the
        // body is a Tcl script, so both drive semantic-token recursion like a
        // bare `proc` / `apply`.
        use crate::ArgRole;
        let reg = crate::registry::CommandRegistry::build_default();
        let args = ["{x y}", "{return [expr {$x + $y}]}"];
        let bodies = reg.arg_indices_for_role("lambda", &args, ArgRole::Body);
        let params = reg.arg_indices_for_role("lambda", &args, ArgRole::ParamList);
        assert_eq!(bodies, vec![1], "lambda body index");
        assert_eq!(params, vec![0], "lambda param-list index");
    }

    #[test]
    fn unicode_normalize_form_is_a_closed_enum() {
        // `unicode::normalize form uclist` — the form word is a closed set
        // (D / C / KD / KC) so an out-of-set literal is flagged (W127).
        let reg = crate::registry::CommandRegistry::build_default();
        let spec = reg.get("unicode::normalize").expect("registered");
        let forms: Vec<&str> = spec
            .arg_values
            .iter()
            .find(|(i, _)| *i == 0)
            .map(|(_, vs)| vs.iter().map(|v| v.value).collect())
            .unwrap_or_default();
        assert_eq!(forms, ["D", "C", "KD", "KC"]);
        assert!(spec.closed_value_args.contains(&0), "form arg is closed");
    }

    #[test]
    fn cipher_mode_and_dir_options_are_enumerated() {
        // The block-cipher primary commands expose closed enum option values
        // (`-mode`, `-dir`) so completion / validation can drive them.
        let reg = crate::registry::CommandRegistry::build_default();
        let des = reg.get("DES::des").expect("DES::des registered");
        let mode = des
            .options
            .iter()
            .find(|o| o.name == "-mode")
            .expect("DES::des has -mode");
        let modes: Vec<&str> = mode.value_values().iter().map(|v| v.value).collect();
        assert!(mode.value_is_closed(), "-mode set is closed");
        assert_eq!(modes, ["ecb", "cbc", "cfb", "ofb"]);
    }

    #[test]
    fn control_package_maps_to_control() {
        // The `control` module's control-flow commands are gated on the
        // `control` package (issue #760).
        let specs = tcllib_command_specs();
        let names = [
            "control::do",
            "control::assert",
            "control::control",
            "control::no-op",
        ];
        for name in names {
            let spec = specs.iter().find(|s| s.name == name);
            assert_eq!(
                spec.and_then(|s| s.required_package),
                Some("control"),
                "missing/incorrect required_package for `{name}`",
            );
        }
    }

    #[test]
    fn control_do_recurses_body_expr_and_keyword() {
        // `control::do body ?option test?` — the body is a script, the
        // option word (`while`/`until`) is a keyword, and the test is an
        // expression.  These roles drive semantic-token recursion (#760).
        use crate::ArgRole;
        let reg = crate::registry::CommandRegistry::build_default();
        let args = ["{body}", "while", "{$x < 10}"];
        let bodies = reg.arg_indices_for_role("control::do", &args, ArgRole::Body);
        let keywords = reg.arg_indices_for_role("control::do", &args, ArgRole::Keyword);
        let exprs = reg.arg_indices_for_role("control::do", &args, ArgRole::Expr);
        assert_eq!(bodies, vec![0], "body role");
        assert_eq!(keywords, vec![1], "keyword role");
        assert_eq!(exprs, vec![2], "expr role");
    }

    #[test]
    fn control_do_option_roles_only_apply_to_full_form() {
        // `?option test?` is only meaningful as a complete pair — a lone
        // `option` (the malformed two-word form) must NOT be highlighted as
        // a keyword just because it sits at that position (PR #763 review).
        use crate::ArgRole;
        let reg = crate::registry::CommandRegistry::build_default();
        // body-only form: just the body recurses, no keyword/expr.
        let one = ["{body}"];
        assert_eq!(
            reg.arg_indices_for_role("control::do", &one, ArgRole::Body),
            vec![0],
        );
        assert!(
            reg.arg_indices_for_role("control::do", &one, ArgRole::Keyword)
                .is_empty(),
        );
        // malformed two-word form (`body` + lone option): no keyword role.
        let two = ["{body}", "while"];
        assert!(
            reg.arg_indices_for_role("control::do", &two, ArgRole::Keyword)
                .is_empty(),
            "lone option word must not be a keyword",
        );
        assert!(
            reg.arg_indices_for_role("control::do", &two, ArgRole::Expr)
                .is_empty(),
        );
    }

    #[test]
    fn control_assert_recurses_expr() {
        use crate::ArgRole;
        let reg = crate::registry::CommandRegistry::build_default();
        let args = ["{$x == 10}", "msg"];
        let exprs = reg.arg_indices_for_role("control::assert", &args, ArgRole::Expr);
        assert_eq!(exprs, vec![0], "control::assert expr role");
    }

    #[test]
    fn struct_list_foreachperm_recurses_body() {
        // `struct::list foreachperm var sequence body` — subcommand-level
        // body/var roles (indices are +1 for the subcommand word).
        use crate::ArgRole;
        let reg = crate::registry::CommandRegistry::build_default();
        let args = ["foreachperm", "p", "{a b c}", "{body}"];
        let bodies = reg.arg_indices_for_role("struct::list", &args, ArgRole::Body);
        let writes = reg.arg_indices_for_role("struct::list", &args, ArgRole::VarWrite);
        assert_eq!(bodies, vec![3], "foreachperm body index");
        assert_eq!(writes, vec![1], "foreachperm var index");
    }

    #[test]
    fn struct_list_mapfor_is_body_filterfor_is_expr() {
        // `mapfor`'s third argument is a Tcl script (body); `filterfor`'s
        // is an expression — they must not be conflated (PR #763 review).
        use crate::ArgRole;
        let reg = crate::registry::CommandRegistry::build_default();
        let mapfor = ["mapfor", "x", "{1 2 3}", "{body}"];
        let filterfor = ["filterfor", "x", "{1 2 3}", "{$x > 1}"];
        assert_eq!(
            reg.arg_indices_for_role("struct::list", &mapfor, ArgRole::Body),
            vec![3],
            "mapfor script is a body",
        );
        assert!(
            reg.arg_indices_for_role("struct::list", &mapfor, ArgRole::Expr)
                .is_empty(),
            "mapfor script is not an expr",
        );
        assert_eq!(
            reg.arg_indices_for_role("struct::list", &filterfor, ArgRole::Expr),
            vec![3],
            "filterfor cond is an expr",
        );
    }
}
