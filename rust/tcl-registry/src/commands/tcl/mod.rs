//! Tcl core command specifications — one file per command.

mod after_;
mod append_;
mod apply;
mod array_;
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
mod flush_;
mod for_;
mod foreach_;
mod foreachline;
mod format_;
mod gets_;
mod glob_;
mod global_;
mod if_;
mod incr_;
mod info_;
mod interp;
mod join_;
mod lappend_;
mod lassign;
mod lindex;
mod linsert;
mod list_;
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
mod mathop;
mod namespace_;
mod oo_abstract;
mod oo_class;
mod oo_classvariable;
mod oo_configurable;
mod oo_copy;
mod oo_define;
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
mod proc_;
mod puts_;
mod re_quote;
mod read_;
mod readfile;
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
mod tcl_idna;
mod tcl_process;
mod tcl_unsupported_corotype;
mod tell_;
mod throw_;
mod time;
mod trace;
mod try_;
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
        mathop::spec(),
        namespace_::spec(),
        oo_abstract::spec(),
        oo_class::spec(),
        oo_classvariable::spec(),
        oo_configurable::spec(),
        oo_copy::spec(),
        oo_define::spec(),
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
        proc_::spec(),
        puts_::spec(),
        re_quote::spec(),
        read_::spec(),
        readfile::spec(),
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
        tell_::spec(),
        throw_::spec(),
        time::spec(),
        trace::spec(),
        try_::spec(),
        unknown::spec(),
        unload::spec(),
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
