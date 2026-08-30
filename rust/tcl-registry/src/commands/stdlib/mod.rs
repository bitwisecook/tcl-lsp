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

//! `stdlib` command specifications.

#![allow(non_snake_case)]

mod gettimes;
mod history;
mod http__cleanup;
mod http__code;
mod http__config;
mod http__cookiejar;
mod http__data;
mod http__error;
mod http__formatquery;
mod http__geturl;
mod http__meta;
mod http__ncode;
mod http__posterror;
mod http__quotestring;
mod http__reasonphrase;
mod http__register;
mod http__registererror;
mod http__requestheaders;
mod http__requestheadervalue;
mod http__requestline;
mod http__reset;
mod http__responsebody;
mod http__responsecode;
mod http__responseheaders;
mod http__responseheadervalue;
mod http__responseinfo;
mod http__responseline;
mod http__size;
mod http__status;
mod http__unregister;
mod http__wait;
mod lgen;
mod lstring;
mod msgcat__mc;
mod msgcat__mcexists;
mod msgcat__mcflmset;
mod msgcat__mcflset;
mod msgcat__mcforgetpackage;
mod msgcat__mcload;
mod msgcat__mcloadedlocales;
mod msgcat__mclocale;
mod msgcat__mcmax;
mod msgcat__mcmset;
mod msgcat__mcn;
mod msgcat__mcpackageconfig;
mod msgcat__mcpackagelocale;
mod msgcat__mcpackagenamespaceget;
mod msgcat__mcpreferences;
mod msgcat__mcset;
mod msgcat__mcunknown;
mod msgcat__mcutil;
mod noop;
mod pkg__create;
mod pkg_mkindex;
mod platform__generic;
mod platform__identify;
mod platform__patterns;
mod platform__shell__generic;
mod platform__shell__identify;
mod safe__interpaddtoaccesspath;
mod safe__interpconfigure;
mod safe__interpcreate;
mod safe__interpdelete;
mod safe__interpfindinaccesspath;
mod safe__interpinit;
mod safe__setlogcmd;
mod safe__setsyncmode;
mod tcl__idna__decode;
mod tcl__idna__encode;
mod tcl__optkeydelete;
mod tcl__optkeyerror;
mod tcl__optkeyparse;
mod tcl__optkeyregister;
mod tcl__optparse;
mod tcl__optproc;
mod tcl__optprocarggiven;
mod tcl__tm__path;
mod tcl__tm__roots;
mod tcl_endofword;
mod tcl_startofnextword;
mod tcl_startofpreviousword;
mod tcl_wordbreakafter;
mod tcl_wordbreakbefore;
mod tcltest__bytestring;
mod tcltest__cleanuptests;
mod tcltest__configure;
mod tcltest__custommatch;
mod tcltest__debug;
mod tcltest__errorchannel;
mod tcltest__errorfile;
mod tcltest__getmatchingfiles;
mod tcltest__interpreter;
mod tcltest__limitconstraints;
mod tcltest__loadfile;
mod tcltest__loadscript;
mod tcltest__loadtestedcommands;
mod tcltest__mainthread;
mod tcltest__makedirectory;
mod tcltest__makefile;
mod tcltest__match;
mod tcltest__matchdirectories;
mod tcltest__matchfiles;
mod tcltest__normalizemsg;
mod tcltest__normalizepath;
mod tcltest__outputchannel;
mod tcltest__outputfile;
mod tcltest__preservecore;
mod tcltest__removedirectory;
mod tcltest__removefile;
mod tcltest__restorestate;
mod tcltest__runalltests;
mod tcltest__savestate;
mod tcltest__singleprocess;
mod tcltest__skip;
mod tcltest__skipdirectories;
mod tcltest__skipfiles;
mod tcltest__temporarydirectory;
mod tcltest__test;
mod tcltest__testconstraint;
mod tcltest__testsdirectory;
mod tcltest__threadreap;
mod tcltest__verbose;
mod tcltest__viewfile;
mod tcltest__workingdirectory;
mod testaccessproc;
mod testalarm;
mod testapplylambda;
mod testappverifierpresent;
mod testasync;
mod testbigdata;
mod testbignumobj;
mod testbooleanobj;
mod testbumpinterpepoch;
mod testbytestring;
mod testchancreate;
mod testchannel;
mod testchannelevent;
mod testchmod;
mod testcmdinfo;
mod testcmdobj2;
mod testcmdtoken;
mod testcmdtrace;
mod testconcatobj;
mod testconvertobj;
mod testcpuid;
mod testcreatecommand;
mod testdcall;
mod testdel;
mod testdelassocdata;
mod testdoubledigits;
mod testdoubleobj;
mod testdstring;
mod testencoding;
mod testevalex;
mod testevalobjv;
mod testevent;
mod testexithandler;
mod testexitmainloop;
mod testexprdouble;
mod testexprdoubleobj;
mod testexprlong;
mod testexprlongobj;
mod testexprparser;
mod testexprstring;
mod testfevent;
mod testfile;
mod testfilehandler;
mod testfilelink;
mod testfilesystem;
mod testfilewait;
mod testfindexecutable;
mod testfindfirst;
mod testfindlast;
mod testfork;
mod testfstildeexpand;
mod testgetassocdata;
mod testgetdefenc;
mod testgetindexfromobjstruct;
mod testgetint;
mod testgetintforindex;
mod testgetopenfile;
mod testgetplatform;
mod testgetunichar;
mod testgetvarfullname;
mod testgotsig;
mod testhandlecount;
mod testhashsystemhash;
mod testindexobj;
mod testinterpdelete;
mod testinterpresolver;
mod testintobj;
mod testisempty;
mod testlink;
mod testlinkarray;
mod testlistapi;
mod testlistobj;
mod testlistrep;
mod testlocale;
mod testlongsize;
mod testlutil;
mod testmainthread;
mod testmsb;
mod testnrelevels;
mod testnreunwind;
mod testnumutfchars;
mod testobj;
mod testopenfilechannelproc;
mod testpanic;
mod testparseargs;
mod testparser;
mod testparsevar;
mod testparsevarname;
mod testpostinit;
mod testpreferstable;
mod testprint;
mod testpurebytesobj;
mod testregexp;
mod testreturn;
mod testsaveresult;
mod testservicemode;
mod testset2;
mod testsetassocdata;
mod testsetbytearraylength;
mod testsetdefenc;
mod testseterr;
mod testseterrorcode;
mod testsetmainloop;
mod testsetnoerr;
mod testsetobjerrorcode;
mod testsetplatform;
mod testsimplefilesystem;
mod testsize;
mod testsocket;
mod teststaticlibrary;
mod teststaticpkg;
mod teststatproc;
mod teststringbytes;
mod teststringobj;
mod testtranslatefilename;
mod testuniclass;
mod testupvar;
mod testutfnext;
mod testutfprev;
mod testutftonormalized;
mod testutftonormalizeddstring;
mod testwrongnumargs;

use crate::spec::CommandSpec;

/// Return all `stdlib` command specifications.
///
/// The flat list is assembled from per-namespace sub-builders (one per
/// logical package family) so no single function carries the whole table.
/// Concatenation order is preserved exactly — the registry-port tests
/// assert the resulting set is identical.
#[must_use]
pub fn stdlib_command_specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(http_specs());
    specs.extend(msgcat_specs());
    specs.extend(pkg_platform_safe_specs());
    specs.extend(tcl_namespace_specs());
    specs.extend(tcltest_specs());
    specs.extend(test_harness_specs());
    specs.extend(test_harness_specs_h_w());
    specs
}

/// `gettimes`/`history`, the `http::*` family, then `lgen`/`lstring`.
fn http_specs() -> Vec<CommandSpec> {
    vec![
        gettimes::spec(),
        history::spec(),
        http__cleanup::spec(),
        http__code::spec(),
        http__config::spec(),
        http__cookiejar::spec(),
        http__data::spec(),
        http__error::spec(),
        http__formatquery::spec(),
        http__geturl::spec(),
        http__meta::spec(),
        http__ncode::spec(),
        http__posterror::spec(),
        http__quotestring::spec(),
        http__reasonphrase::spec(),
        http__register::spec(),
        http__registererror::spec(),
        http__requestheaders::spec(),
        http__requestheadervalue::spec(),
        http__requestline::spec(),
        http__reset::spec(),
        http__responsebody::spec(),
        http__responsecode::spec(),
        http__responseheaders::spec(),
        http__responseheadervalue::spec(),
        http__responseinfo::spec(),
        http__responseline::spec(),
        http__size::spec(),
        http__status::spec(),
        http__unregister::spec(),
        http__wait::spec(),
        lgen::spec(),
        lstring::spec(),
    ]
}

/// The `msgcat::*` localisation family, then `noop`.
fn msgcat_specs() -> Vec<CommandSpec> {
    vec![
        msgcat__mc::spec(),
        msgcat__mcexists::spec(),
        msgcat__mcflmset::spec(),
        msgcat__mcflset::spec(),
        msgcat__mcforgetpackage::spec(),
        msgcat__mcload::spec(),
        msgcat__mcloadedlocales::spec(),
        msgcat__mclocale::spec(),
        msgcat__mcmax::spec(),
        msgcat__mcmset::spec(),
        msgcat__mcn::spec(),
        msgcat__mcpackageconfig::spec(),
        msgcat__mcpackagelocale::spec(),
        msgcat__mcpackagenamespaceget::spec(),
        msgcat__mcpreferences::spec(),
        msgcat__mcset::spec(),
        msgcat__mcunknown::spec(),
        msgcat__mcutil::spec(),
        noop::spec(),
    ]
}

/// `pkg::*`, `platform::*`, and `safe::*` interpreter families.
fn pkg_platform_safe_specs() -> Vec<CommandSpec> {
    vec![
        pkg__create::spec(),
        pkg_mkindex::spec(),
        platform__generic::spec(),
        platform__identify::spec(),
        platform__patterns::spec(),
        platform__shell__generic::spec(),
        platform__shell__identify::spec(),
        safe__interpaddtoaccesspath::spec(),
        safe__interpconfigure::spec(),
        safe__interpcreate::spec(),
        safe__interpdelete::spec(),
        safe__interpfindinaccesspath::spec(),
        safe__interpinit::spec(),
        safe__setlogcmd::spec(),
        safe__setsyncmode::spec(),
    ]
}

/// The `tcl::*` / `tcl_*` core-namespace family.
fn tcl_namespace_specs() -> Vec<CommandSpec> {
    vec![
        tcl__idna__decode::spec(),
        tcl__idna__encode::spec(),
        tcl__optkeydelete::spec(),
        tcl__optkeyerror::spec(),
        tcl__optkeyparse::spec(),
        tcl__optkeyregister::spec(),
        tcl__optparse::spec(),
        tcl__optproc::spec(),
        tcl__optprocarggiven::spec(),
        tcl__tm__path::spec(),
        tcl__tm__roots::spec(),
        tcl_endofword::spec(),
        tcl_startofnextword::spec(),
        tcl_startofpreviousword::spec(),
        tcl_wordbreakafter::spec(),
        tcl_wordbreakbefore::spec(),
    ]
}

/// The `tcltest::*` test-framework family.
///
/// The tcltest package version bundled with each Tcl release:
///
/// | Tcl     | tcltest  |
/// |---------|----------|
/// | 8.4.20  | 2.2.11   |
/// | 8.5.19  | 2.3.8    |
/// | 8.6.16  | 2.5.9    |
/// | 9.0.3   | 2.5.10   |
///
/// Every command below is gated to [`SpecSurface::ALL_TCL`] (Tcl 8.4-9.1) so it
/// is never offered in the restricted F5 iRules / iApps dialects.  The one
/// version exception is `bytestring`, which is not defined under Tcl 9.0+ and
/// is gated to [`SpecSurface::TCL8X`].  Option-level version drift is expressed
/// with `OptionSpec::min_version` against the *tcltest* package version (e.g.
/// `test -errorCode` needs tcltest `2.5`).
fn tcltest_specs() -> Vec<CommandSpec> {
    vec![
        tcltest__bytestring::spec(),
        tcltest__cleanuptests::spec(),
        tcltest__configure::spec(),
        tcltest__custommatch::spec(),
        tcltest__debug::spec(),
        tcltest__errorchannel::spec(),
        tcltest__errorfile::spec(),
        tcltest__getmatchingfiles::spec(),
        tcltest__interpreter::spec(),
        tcltest__limitconstraints::spec(),
        tcltest__loadfile::spec(),
        tcltest__loadscript::spec(),
        tcltest__loadtestedcommands::spec(),
        tcltest__mainthread::spec(),
        tcltest__makedirectory::spec(),
        tcltest__makefile::spec(),
        tcltest__match::spec(),
        tcltest__matchdirectories::spec(),
        tcltest__matchfiles::spec(),
        tcltest__normalizemsg::spec(),
        tcltest__normalizepath::spec(),
        tcltest__outputchannel::spec(),
        tcltest__outputfile::spec(),
        tcltest__preservecore::spec(),
        tcltest__removedirectory::spec(),
        tcltest__removefile::spec(),
        tcltest__restorestate::spec(),
        tcltest__runalltests::spec(),
        tcltest__savestate::spec(),
        tcltest__singleprocess::spec(),
        tcltest__skip::spec(),
        tcltest__skipdirectories::spec(),
        tcltest__skipfiles::spec(),
        tcltest__temporarydirectory::spec(),
        tcltest__test::spec(),
        tcltest__testconstraint::spec(),
        tcltest__testsdirectory::spec(),
        tcltest__threadreap::spec(),
        tcltest__verbose::spec(),
        tcltest__viewfile::spec(),
        tcltest__workingdirectory::spec(),
    ]
}

/// The internal `test*` C-API harness commands (`testa*`–`testg*`).
fn test_harness_specs() -> Vec<CommandSpec> {
    vec![
        testaccessproc::spec(),
        testalarm::spec(),
        testapplylambda::spec(),
        testappverifierpresent::spec(),
        testasync::spec(),
        testbigdata::spec(),
        testbignumobj::spec(),
        testbooleanobj::spec(),
        testbumpinterpepoch::spec(),
        testbytestring::spec(),
        testchancreate::spec(),
        testchannel::spec(),
        testchannelevent::spec(),
        testchmod::spec(),
        testcmdinfo::spec(),
        testcmdobj2::spec(),
        testcmdtoken::spec(),
        testcmdtrace::spec(),
        testconcatobj::spec(),
        testconvertobj::spec(),
        testcpuid::spec(),
        testcreatecommand::spec(),
        testdcall::spec(),
        testdel::spec(),
        testdelassocdata::spec(),
        testdoubledigits::spec(),
        testdoubleobj::spec(),
        testdstring::spec(),
        testencoding::spec(),
        testevalex::spec(),
        testevalobjv::spec(),
        testevent::spec(),
        testexithandler::spec(),
        testexitmainloop::spec(),
        testexprdouble::spec(),
        testexprdoubleobj::spec(),
        testexprlong::spec(),
        testexprlongobj::spec(),
        testexprparser::spec(),
        testexprstring::spec(),
        testfevent::spec(),
        testfile::spec(),
        testfilehandler::spec(),
        testfilelink::spec(),
        testfilesystem::spec(),
        testfilewait::spec(),
        testfindexecutable::spec(),
        testfindfirst::spec(),
        testfindlast::spec(),
        testfork::spec(),
        testfstildeexpand::spec(),
        testgetassocdata::spec(),
        testgetdefenc::spec(),
        testgetindexfromobjstruct::spec(),
        testgetint::spec(),
        testgetintforindex::spec(),
        testgetopenfile::spec(),
        testgetplatform::spec(),
        testgetunichar::spec(),
        testgetvarfullname::spec(),
        testgotsig::spec(),
    ]
}

/// The internal `test*` C-API harness commands (`testh*`–`testw*`).
fn test_harness_specs_h_w() -> Vec<CommandSpec> {
    vec![
        testhandlecount::spec(),
        testhashsystemhash::spec(),
        testindexobj::spec(),
        testinterpdelete::spec(),
        testinterpresolver::spec(),
        testintobj::spec(),
        testisempty::spec(),
        testlink::spec(),
        testlinkarray::spec(),
        testlistapi::spec(),
        testlistobj::spec(),
        testlistrep::spec(),
        testlocale::spec(),
        testlongsize::spec(),
        testlutil::spec(),
        testmainthread::spec(),
        testmsb::spec(),
        testnrelevels::spec(),
        testnreunwind::spec(),
        testnumutfchars::spec(),
        testobj::spec(),
        testopenfilechannelproc::spec(),
        testpanic::spec(),
        testparseargs::spec(),
        testparser::spec(),
        testparsevar::spec(),
        testparsevarname::spec(),
        testpostinit::spec(),
        testpreferstable::spec(),
        testprint::spec(),
        testpurebytesobj::spec(),
        testregexp::spec(),
        testreturn::spec(),
        testsaveresult::spec(),
        testservicemode::spec(),
        testset2::spec(),
        testsetassocdata::spec(),
        testsetbytearraylength::spec(),
        testsetdefenc::spec(),
        testseterr::spec(),
        testseterrorcode::spec(),
        testsetmainloop::spec(),
        testsetnoerr::spec(),
        testsetobjerrorcode::spec(),
        testsetplatform::spec(),
        testsimplefilesystem::spec(),
        testsize::spec(),
        testsocket::spec(),
        teststaticlibrary::spec(),
        teststaticpkg::spec(),
        teststatproc::spec(),
        teststringbytes::spec(),
        teststringobj::spec(),
        testtranslatefilename::spec(),
        testuniclass::spec(),
        testupvar::spec(),
        testutfnext::spec(),
        testutfprev::spec(),
        testutftonormalized::spec(),
        testutftonormalizeddstring::spec(),
        testwrongnumargs::spec(),
    ]
}
