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
mod testapplylambda;
mod testappverifierpresent;
mod testasync;
mod testbigdata;
mod testbignumobj;
mod testbooleanobj;
mod testbumpinterpepoch;
mod testbytestring;
mod testchannel;
mod testchannelevent;
mod testcmdinfo;
mod testcmdtoken;
mod testcmdtrace;
mod testconcatobj;
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
mod testfilelink;
mod testfilesystem;
mod testfindfirst;
mod testfindlast;
mod testfstildeexpand;
mod testgetassocdata;
mod testgetindexfromobjstruct;
mod testgetint;
mod testgetintforindex;
mod testgetplatform;
mod testgetunichar;
mod testgetvarfullname;
mod testhandlecount;
mod testhashsystemhash;
mod testindexobj;
mod testinterpdelete;
mod testinterpresolver;
mod testintobj;
mod testlink;
mod testlinkarray;
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
mod testpanic;
mod testparseargs;
mod testparser;
mod testparsevar;
mod testparsevarname;
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
mod teststringbytes;
mod teststringobj;
mod testtranslatefilename;
mod testuniclass;
mod testupvar;
mod testutfnext;
mod testutfprev;
mod testwrongnumargs;

use crate::spec::CommandSpec;

/// Return all `stdlib` command specifications.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn stdlib_command_specs() -> Vec<CommandSpec> {
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
        testapplylambda::spec(),
        testappverifierpresent::spec(),
        testasync::spec(),
        testbigdata::spec(),
        testbignumobj::spec(),
        testbooleanobj::spec(),
        testbumpinterpepoch::spec(),
        testbytestring::spec(),
        testchannel::spec(),
        testchannelevent::spec(),
        testcmdinfo::spec(),
        testcmdtoken::spec(),
        testcmdtrace::spec(),
        testconcatobj::spec(),
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
        testfilelink::spec(),
        testfilesystem::spec(),
        testfindfirst::spec(),
        testfindlast::spec(),
        testfstildeexpand::spec(),
        testgetassocdata::spec(),
        testgetindexfromobjstruct::spec(),
        testgetint::spec(),
        testgetintforindex::spec(),
        testgetplatform::spec(),
        testgetunichar::spec(),
        testgetvarfullname::spec(),
        testhandlecount::spec(),
        testhashsystemhash::spec(),
        testindexobj::spec(),
        testinterpdelete::spec(),
        testinterpresolver::spec(),
        testintobj::spec(),
        testlink::spec(),
        testlinkarray::spec(),
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
        testpanic::spec(),
        testparseargs::spec(),
        testparser::spec(),
        testparsevar::spec(),
        testparsevarname::spec(),
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
        teststringbytes::spec(),
        teststringobj::spec(),
        testtranslatefilename::spec(),
        testuniclass::spec(),
        testupvar::spec(),
        testutfnext::spec(),
        testutfprev::spec(),
        testwrongnumargs::spec(),
    ]
}
