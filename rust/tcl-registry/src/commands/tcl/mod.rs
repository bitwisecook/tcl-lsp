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

//! Tcl core command specifications — one file per command.

// A few Tcl commands use the `::` namespace separator → double
// underscores in module names (e.g. `regex__quote` for `regex::quote`).
// Suppress the snake_case warnings for these.
#![allow(non_snake_case)]

mod after_;
mod append_;
mod apply;
mod array_;
mod auto_execok;
mod auto_import;
mod auto_load;
mod auto_load_index;
mod auto_mkindex;
mod auto_mkindex_old;
mod auto_qualify;
mod auto_reset;
mod bgerror;
mod binary_;
mod break_;
mod catch_;
mod cd;
mod chan_;
mod clock_;
mod close_;
mod concat_;
mod const_;
mod continue_;
mod coroinject;
mod coroprobe;
mod coroutine;
mod dict;
mod disabled_in_irules;
mod encoding_;
mod eof_;
mod error_;
mod eval_;
mod exec_;
mod exit_;
mod expr_;
mod fblocked;
mod fconfigure_;
mod fcopy;
mod file_;
mod fileevent;
mod filename;
mod flush_;
mod for_;
mod foreach_;
mod foreachline;
mod format_;
mod gets_;
mod glob_;
mod global_;
mod http;
mod if_;
mod incr_;
mod info_;
mod interp;
mod join_;
mod lappend_;
mod lassign;
mod ledit;
mod lfilter;
mod lindex;
mod linsert;
mod list_;
mod list_math_91;
mod llength;
mod lmap_;
mod load;
mod lpop;
mod lrange;
mod lremove;
mod lrepeat;
mod lreplace;
mod lreverse;
mod lsearch_;
mod lseq;
mod lset;
mod lsort_;
mod mathfunc;
mod mathfunc_generated;
mod mathop;
mod mathop_generated;
mod memory;
mod namespace_;
mod nextto;
mod oo_abstract;
mod oo_callback;
mod oo_class;
mod oo_classvariable;
mod oo_configurable;
mod oo_copy;
mod oo_define;
mod oo_helpers;
mod oo_link;
mod oo_my;
mod oo_next;
mod oo_objdefine;
mod oo_object;
mod oo_self;
mod oo_singleton;
mod open_;
mod package_;
mod parray;
mod pid;
mod pkg__create;
mod pkg_mkindex;
mod prefix_;
mod proc_;
mod puts_;
mod pwd;
mod re_quote;
mod read_;
mod readfile;
mod regex__quote;
mod regex_quote;
mod regexp_;
mod regexp_quote;
mod registry_;
mod regsub_;
mod rename_;
mod return_;
mod scan_;
mod seek_;
mod set_;
mod socket_;
mod source_;
mod split_;
mod string_;
mod subst_;
mod switch_;
mod tailcall_;
mod tcl__build_info;
mod tcl_findlibrary;
mod tcl_idna;
mod tcl_process;
mod tcl_unsupported_corotype;
mod tcl_zipfs;
mod tcllog;
mod tclpkgsetup;
mod tclpkgunknown;
mod tell_;
mod throw_;
mod time;
mod timer;
mod timerate;
mod trace;
mod try_;
pub(crate) use try_::is_dash_fallthrough as try_body_is_fallthrough;
mod unicode_;
mod unknown;
mod unload;
mod unset_;
mod update;
mod uplevel_;
mod upvar_;
mod variable_;
mod vwait;
mod while_;
mod writefile;
mod yield_;
mod yieldto;
mod zlib;

use crate::spec::CommandSpec;

/// Return all Tcl core command specifications.
// Flat declarative `vec![spec(), ...]` — splitting hurts
// readability for a one-shot table.
#[must_use]
pub fn tcl_command_specs() -> Vec<CommandSpec> {
    let mut specs = tcl_specs_a_through_l();
    specs.extend(tcl_specs_m_through_z());
    // The `tcl::mathop` operator ensemble (every spelling).
    specs.extend(mathop_generated::specs());
    // The `tcl::mathfunc` math-function ensemble (both qualified spellings).
    specs.extend(mathfunc_generated::specs());
    // Standalone specs for `dict` subcommands that are also genuine,
    // separately-callable `::tcl::dict::<name>` commands (issue #923 idx 105).
    specs.extend(dict::qualified_specs());
    // Standalone specs for the `oo::Helpers` members under their qualified
    // spelling — real, separately-callable commands whose *bare* twins are
    // method-context-only (issue #1026).
    specs.extend(oo_helpers::qualified_specs());
    // Simple named commands not yet implemented. (`vec!` — the spec
    // table is past clippy's stack-array size threshold.)
    specs.extend(vec![
        auto_execok::spec(),
        auto_import::spec(),
        auto_load::spec(),
        auto_mkindex::spec(),
        auto_mkindex_old::spec(),
        auto_qualify::spec(),
        auto_reset::spec(),
        auto_load_index::spec(),
        tcllog::spec(),
        tclpkgsetup::spec(),
        tclpkgunknown::spec(),
        bgerror::spec(),
        filename::spec(),
        http::spec(),
        memory::spec(),
        nextto::spec(),
        pkg__create::spec(),
        pkg_mkindex::spec(),
        pwd::spec(),
        tcl__build_info::spec(),
        tcl_findlibrary::spec(),
    ]);
    specs
}

/// `tcl` dialect specs for commands `a` through `l` (~half of the
/// table).  Split out from [`tcl_command_specs`] so neither half
/// trips clippy's `too_many_lines`.
fn tcl_specs_a_through_l() -> Vec<CommandSpec> {
    vec![
        after_::spec(),
        append_::spec(),
        apply::spec(),
        array_::spec(),
        binary_::spec(),
        break_::spec(),
        oo_callback::spec(),
        catch_::spec(),
        cd::spec(),
        chan_::spec(),
        clock_::spec(),
        close_::spec(),
        concat_::spec(),
        const_::spec(),
        continue_::spec(),
        coroinject::spec(),
        coroprobe::spec(),
        coroutine::spec(),
        dict::spec(),
        disabled_in_irules::spec(),
        encoding_::spec(),
        eof_::spec(),
        error_::spec(),
        eval_::spec(),
        exec_::spec(),
        exit_::spec(),
        expr_::spec(),
        fblocked::spec(),
        fconfigure_::spec(),
        fcopy::spec(),
        file_::spec(),
        fileevent::spec(),
        flush_::spec(),
        for_::spec(),
        foreach_::spec(),
        foreachline::spec(),
        format_::spec(),
        gets_::spec(),
        glob_::spec(),
        global_::spec(),
        if_::spec(),
        incr_::spec(),
        info_::spec(),
        interp::spec(),
        join_::spec(),
        lappend_::spec(),
        lassign::spec(),
        ledit::spec(),
        lfilter::spec(),
        list_math_91::divmod(),
        list_math_91::frexp(),
        list_math_91::modf(),
        list_math_91::remquo(),
        lindex::spec(),
        linsert::spec(),
        list_::spec(),
        llength::spec(),
        lmap_::spec(),
        load::spec(),
        lpop::spec(),
        lrange::spec(),
        lremove::spec(),
        lrepeat::spec(),
        lreplace::spec(),
        lreverse::spec(),
        lsearch_::spec(),
        lseq::spec(),
        lset::spec(),
        lsort_::spec(),
    ]
}

/// `tcl` dialect specs for commands `m` through `z`.  Sister of
/// [`tcl_specs_a_through_l`].
fn tcl_specs_m_through_z() -> Vec<CommandSpec> {
    vec![
        mathfunc::spec(),
        mathop::spec(),
        namespace_::spec(),
        oo_abstract::spec(),
        oo_class::spec(),
        oo_classvariable::spec(),
        oo_classvariable::spec_ooutil_86(),
        oo_configurable::spec(),
        oo_copy::spec(),
        oo_define::spec(),
        oo_link::spec(),
        oo_link::spec_ooutil_86(),
        oo_callback::mymethod_spec(),
        oo_callback::mymethod_spec_ooutil_86(),
        oo_my::spec(),
        oo_next::spec(),
        oo_objdefine::spec(),
        oo_object::spec(),
        oo_self::spec(),
        oo_singleton::spec(),
        open_::spec(),
        package_::spec(),
        parray::spec(),
        pid::spec(),
        prefix_::spec(),
        proc_::spec(),
        puts_::spec(),
        re_quote::spec(),
        read_::spec(),
        readfile::spec(),
        regex__quote::spec(),
        regex_quote::spec(),
        regexp_::spec(),
        regexp_quote::spec(),
        registry_::spec(),
        regsub_::spec(),
        rename_::spec(),
        return_::spec(),
        scan_::spec(),
        seek_::spec(),
        set_::spec(),
        socket_::spec(),
        source_::spec(),
        split_::spec(),
        string_::spec(),
        subst_::spec(),
        switch_::spec(),
        tailcall_::spec(),
        tcl_idna::spec(),
        tcl_idna::spec_qualified(),
        tcl_process::spec(),
        tcl_process::spec_qualified(),
        tcl_unsupported_corotype::spec(),
        tcl_unsupported_corotype::spec_qualified(),
        tcl_zipfs::spec(),
        tcl_zipfs::spec_qualified(),
        tell_::spec(),
        throw_::spec(),
        time::spec(),
        timer::spec(),
        timerate::spec(),
        trace::spec(),
        try_::spec(),
        unknown::spec(),
        unload::spec(),
        unicode_::spec(),
        unset_::spec(),
        update::spec(),
        uplevel_::spec(),
        upvar_::spec(),
        variable_::spec(),
        vwait::spec(),
        while_::spec(),
        writefile::spec(),
        yield_::spec(),
        yieldto::spec(),
        zlib::spec(),
    ]
}
