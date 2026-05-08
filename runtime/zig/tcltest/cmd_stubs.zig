// Stub registrations for upstream `tclTest.c` commands not yet
// graduated to real implementations elsewhere in
// ``runtime/zig/tcltest/``.  Each stub raises an explicit Tcl error
// when called, so test scripts hitting an unported command see a
// clear "not yet ported" / "not supported under WASM" message
// rather than ``invalid command name``.
//
// The migration path: as a real implementation lands in a sibling
// ``cmd_*.zig`` file, the matching :data:`registrations` row here is
// removed.  ``dispatch/tcl_cmd_table.zig`` concatenates every
// tcltest cmd file's ``registrations`` slice, so the static name
// table reflects whatever's currently shipped.
//
// See ``docs/design/compiler/wasm-extensions.md`` for the full
// extension contract and
// ``docs/design/compiler/wasm-extensions-tcltest.md`` for the
// per-command triage matrix.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn build_msg(buf: []const u8) i32 {
    const dst = obj.alloc(@intCast(buf.len));
    if (dst == 0) return 0;
    const dst_p: [*]u8 = @ptrFromInt(dst);
    for (buf, 0..) |c, i| dst_p[i] = c;
    return obj.obj_new_string_take(dst, @intCast(buf.len), @intCast(buf.len));
}

fn stub_named(words: []const i32, comptime name: []const u8, comptime reason: []const u8) result_mod.InterpResult {
    _ = words;
    var buf: [256]u8 = undefined;
    const slice = std.fmt.bufPrint(
        &buf,
        "{s}: {s}",
        .{ name, reason },
    ) catch name ++ ": stub";
    catch_mod.tcl_cmd_error(build_msg(slice));
    return result_mod.from_globals(0);
}

const NOT_SUPPORTED = "not supported under WASM (probes C-only state we don't replicate)";
const NOT_YET_PORTED = "not yet ported to the WASM runtime — track at docs/design/compiler/wasm-extensions-tcltest.md";

// -- NOT-PORTABLE stubs ----------------------------------------------------

fn st_testsocket(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsocket", NOT_SUPPORTED ++ " (BSD sockets unavailable)"); }
fn st_testmainthread(w: []const i32) result_mod.InterpResult { return stub_named(w, "testmainthread", NOT_SUPPORTED ++ " (no threads under WASM)"); }
fn st_testcpuid(w: []const i32) result_mod.InterpResult { return stub_named(w, "testcpuid", NOT_SUPPORTED ++ " (x86 cpuid unavailable)"); }
fn st_testfevent(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfevent", NOT_SUPPORTED ++ " (file events: no event loop)"); }
fn st_testevent(w: []const i32) result_mod.InterpResult { return stub_named(w, "testevent", NOT_SUPPORTED ++ " (no event loop)"); }
fn st_testsetmainloop(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsetmainloop", NOT_SUPPORTED ++ " (no main loop)"); }
fn st_testexitmainloop(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexitmainloop", NOT_SUPPORTED ++ " (no main loop)"); }
fn st_testexithandler(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexithandler", NOT_SUPPORTED ++ " (no atexit hooks)"); }
fn st_testservicemode(w: []const i32) result_mod.InterpResult { return stub_named(w, "testservicemode", NOT_SUPPORTED ++ " (no event service mode)"); }
fn st_teststaticlibrary(w: []const i32) result_mod.InterpResult { return stub_named(w, "teststaticlibrary", NOT_SUPPORTED ++ " (dynamic loading unavailable)"); }
fn st_testlink(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlink", NOT_SUPPORTED ++ " (Tcl_LinkVar requires C address probes)"); }
fn st_testlinkarray(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlinkarray", NOT_SUPPORTED ++ " (Tcl_LinkArray requires C address probes)"); }
fn st_testchannel(w: []const i32) result_mod.InterpResult { return stub_named(w, "testchannel", NOT_SUPPORTED ++ " (channel internals)"); }
fn st_testchannelevent(w: []const i32) result_mod.InterpResult { return stub_named(w, "testchannelevent", NOT_SUPPORTED ++ " (no event loop)"); }
fn st_testfilesystem(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfilesystem", NOT_SUPPORTED ++ " (Tcl_FSRegister C API)"); }
fn st_testsimplefilesystem(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsimplefilesystem", NOT_SUPPORTED ++ " (Tcl_FSRegister C API)"); }
fn st_testfile(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfile", NOT_SUPPORTED ++ " (native FS probes)"); }
fn st_testfilelink(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfilelink", NOT_SUPPORTED ++ " (symlink semantics)"); }
fn st_testfstildeexpand(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfstildeexpand", NOT_SUPPORTED ++ " (HOME/passwd lookup)"); }
fn st_testtranslatefilename(w: []const i32) result_mod.InterpResult { return stub_named(w, "testtranslatefilename", NOT_SUPPORTED ++ " (native FS probes)"); }
fn st_testlocale(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlocale", NOT_SUPPORTED ++ " (setlocale unavailable)"); }
fn st_testasync(w: []const i32) result_mod.InterpResult { return stub_named(w, "testasync", NOT_SUPPORTED ++ " (Tcl_AsyncMark requires async signal delivery)"); }
fn st_testpanic(w: []const i32) result_mod.InterpResult { return stub_named(w, "testpanic", NOT_SUPPORTED ++ " (Tcl_Panic aborts the process)"); }
fn st_testhashsystemhash(w: []const i32) result_mod.InterpResult { return stub_named(w, "testhashsystemhash", NOT_SUPPORTED ++ " (system hash table internals)"); }
fn st_testhandlecount(w: []const i32) result_mod.InterpResult { return stub_named(w, "testhandlecount", NOT_SUPPORTED ++ " (Tcl_GetObjType internals)"); }
fn st_testappverifierpresent(w: []const i32) result_mod.InterpResult { return stub_named(w, "testappverifierpresent", NOT_SUPPORTED ++ " (Win32-only API)"); }
fn st_teststaticlibrary_alias(w: []const i32) result_mod.InterpResult { _ = w; return result_mod.from_globals(0); }
fn st_testprint(w: []const i32) result_mod.InterpResult { return stub_named(w, "testprint", NOT_SUPPORTED ++ " (printf into the C runtime)"); }
fn st_testnreunwind(w: []const i32) result_mod.InterpResult { return stub_named(w, "testnreunwind", NOT_SUPPORTED ++ " (NRE C-stack unwinding)"); }
fn st_testnrelevels(w: []const i32) result_mod.InterpResult { return stub_named(w, "testnrelevels", NOT_SUPPORTED ++ " (NRE C-stack levels)"); }
fn st_testinterpresolver(w: []const i32) result_mod.InterpResult { return stub_named(w, "testinterpresolver", NOT_SUPPORTED ++ " (Tcl_SetInterpResolver C API)"); }
fn st_testapplylambda(w: []const i32) result_mod.InterpResult { return stub_named(w, "testapplylambda", NOT_SUPPORTED ++ " (probes proc-body cache C state)"); }
fn st_testpreferstable(w: []const i32) result_mod.InterpResult { return stub_named(w, "testpreferstable", NOT_SUPPORTED ++ " (Tcl_PkgPreferences C state)"); }
fn st_testbumpinterpepoch(w: []const i32) result_mod.InterpResult { return stub_named(w, "testbumpinterpepoch", NOT_SUPPORTED ++ " (interp epoch counter is C-only)"); }
fn st_testgetplatform(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetplatform", NOT_SUPPORTED ++ " (TclPlatform global)"); }
fn st_testsetplatform(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsetplatform", NOT_SUPPORTED ++ " (TclPlatform global)"); }

// -- Generic NOT-YET-PORTED stubs ------------------------------------------
// These are commands whose Tcl-level behaviour we *could* replicate
// with more porting work but haven't yet.  Stub-with-error keeps
// upstream test corpus runs noisy-but-clear about what's missing.

fn st_gettimes(w: []const i32) result_mod.InterpResult { return stub_named(w, "gettimes", NOT_YET_PORTED); }
fn st_noop(w: []const i32) result_mod.InterpResult { _ = w; return result_mod.ok(obj.obj_new_string(0, 0)); } // truly trivial
fn st_testdcall(w: []const i32) result_mod.InterpResult { return stub_named(w, "testdcall", NOT_YET_PORTED); }
fn st_testdel(w: []const i32) result_mod.InterpResult { return stub_named(w, "testdel", NOT_YET_PORTED); }
fn st_testdelassocdata(w: []const i32) result_mod.InterpResult { return stub_named(w, "testdelassocdata", NOT_YET_PORTED); }
fn st_testgetassocdata(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetassocdata", NOT_YET_PORTED); }
fn st_testsetassocdata(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsetassocdata", NOT_YET_PORTED); }
fn st_testcmdinfo(w: []const i32) result_mod.InterpResult { return stub_named(w, "testcmdinfo", NOT_YET_PORTED); }
fn st_testcmdtoken(w: []const i32) result_mod.InterpResult { return stub_named(w, "testcmdtoken", NOT_YET_PORTED); }
fn st_testcmdtrace(w: []const i32) result_mod.InterpResult { return stub_named(w, "testcmdtrace", NOT_YET_PORTED); }
fn st_testcmdobj2(w: []const i32) result_mod.InterpResult { return stub_named(w, "testcmdobj2", NOT_YET_PORTED); }
fn st_testcreatecommand(w: []const i32) result_mod.InterpResult { return stub_named(w, "testcreatecommand", NOT_YET_PORTED); }
fn st_testinterpdelete(w: []const i32) result_mod.InterpResult { return stub_named(w, "testinterpdelete", NOT_YET_PORTED); }
fn st_testdstring(w: []const i32) result_mod.InterpResult { return stub_named(w, "testdstring", NOT_YET_PORTED); }
fn st_testencoding(w: []const i32) result_mod.InterpResult { return stub_named(w, "testencoding", NOT_YET_PORTED); }
// testevalex/testevalobjv/testreturn/testset*/testwrongnumargs — graduated to cmd_eval.zig
fn st_testparser(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparser", NOT_YET_PORTED); }
fn st_testparsevar(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparsevar", NOT_YET_PORTED); }
fn st_testparsevarname(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparsevarname", NOT_YET_PORTED); }
fn st_testexprparser(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprparser", NOT_YET_PORTED); }
fn st_testparseargs(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparseargs", NOT_YET_PORTED); }
fn st_testexprlong(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprlong", NOT_YET_PORTED); }
fn st_testexprlongobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprlongobj", NOT_YET_PORTED); }
fn st_testexprdouble(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprdouble", NOT_YET_PORTED); }
fn st_testexprdoubleobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprdoubleobj", NOT_YET_PORTED); }
fn st_testexprstring(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprstring", NOT_YET_PORTED); }
fn st_testconcatobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testconcatobj", NOT_YET_PORTED); }
fn st_testpurebytesobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testpurebytesobj", NOT_YET_PORTED); }
fn st_teststringbytes(w: []const i32) result_mod.InterpResult { return stub_named(w, "teststringbytes", NOT_YET_PORTED); }
fn st_testbytestring(w: []const i32) result_mod.InterpResult { return stub_named(w, "testbytestring", NOT_YET_PORTED); }
fn st_testsetbytearraylength(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsetbytearraylength", NOT_YET_PORTED); }
fn st_testutfnext(w: []const i32) result_mod.InterpResult { return stub_named(w, "testutfnext", NOT_YET_PORTED); }
fn st_testutfprev(w: []const i32) result_mod.InterpResult { return stub_named(w, "testutfprev", NOT_YET_PORTED); }
fn st_testnumutfchars(w: []const i32) result_mod.InterpResult { return stub_named(w, "testnumutfchars", NOT_YET_PORTED); }
fn st_testgetunichar(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetunichar", NOT_YET_PORTED); }
fn st_testfindfirst(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfindfirst", NOT_YET_PORTED); }
fn st_testfindlast(w: []const i32) result_mod.InterpResult { return stub_named(w, "testfindlast", NOT_YET_PORTED); }
fn st_testuniclass(w: []const i32) result_mod.InterpResult { return stub_named(w, "testuniclass", NOT_YET_PORTED); }
fn st_testdoubledigits(w: []const i32) result_mod.InterpResult { return stub_named(w, "testdoubledigits", NOT_YET_PORTED); }
fn st_testgetint(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetint", NOT_YET_PORTED); }
fn st_testgetintforindex(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetintforindex", NOT_YET_PORTED); }
fn st_testgetindexfromobjstruct(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetindexfromobjstruct", NOT_YET_PORTED); }
fn st_testgetvarfullname(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetvarfullname", NOT_YET_PORTED); }
fn st_testupvar(w: []const i32) result_mod.InterpResult { return stub_named(w, "testupvar", NOT_YET_PORTED); }
fn st_testregexp(w: []const i32) result_mod.InterpResult { return stub_named(w, "testregexp", NOT_YET_PORTED); }
fn st_testlistrep(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlistrep", NOT_YET_PORTED); }
fn st_testlongsize(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlongsize", NOT_YET_PORTED); }
fn st_testsize(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsize", NOT_YET_PORTED); }
fn st_testlutil(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlutil", NOT_YET_PORTED); }
fn st_testmsb(w: []const i32) result_mod.InterpResult { return stub_named(w, "testmsb", NOT_YET_PORTED); }
fn st_test_build_info(w: []const i32) result_mod.InterpResult { return stub_named(w, "::tcl::test::build-info", NOT_YET_PORTED); }
fn st_test_ns_basic_createdcommand(w: []const i32) result_mod.InterpResult { return stub_named(w, "test_ns_basic::createdcommand", NOT_YET_PORTED); }
fn st_value_at(w: []const i32) result_mod.InterpResult { return stub_named(w, "value:at:", NOT_YET_PORTED); }
fn st_lstring(w: []const i32) result_mod.InterpResult { return stub_named(w, "lstring", NOT_YET_PORTED); }
fn st_lgen(w: []const i32) result_mod.InterpResult { return stub_named(w, "lgen", NOT_YET_PORTED); }
fn st_procbody_proc(w: []const i32) result_mod.InterpResult { return stub_named(w, "tcl::procbodytest::proc", NOT_YET_PORTED); }
fn st_procbody_check(w: []const i32) result_mod.InterpResult { return stub_named(w, "tcl::procbodytest::check", NOT_YET_PORTED); }

pub const registrations = [_]reg.CmdEntry{
    // Truly trivial
    .{ .name = "noop", .arity_min = 0, .arity_max = null, .handler = &st_noop },

    // NOT-PORTABLE — fundamentally unavailable under WASM
    .{ .name = "testsocket", .arity_min = 0, .arity_max = null, .handler = &st_testsocket },
    .{ .name = "testmainthread", .arity_min = 0, .arity_max = null, .handler = &st_testmainthread },
    .{ .name = "testcpuid", .arity_min = 0, .arity_max = null, .handler = &st_testcpuid },
    .{ .name = "testfevent", .arity_min = 0, .arity_max = null, .handler = &st_testfevent },
    .{ .name = "testevent", .arity_min = 0, .arity_max = null, .handler = &st_testevent },
    .{ .name = "testsetmainloop", .arity_min = 0, .arity_max = null, .handler = &st_testsetmainloop },
    .{ .name = "testexitmainloop", .arity_min = 0, .arity_max = null, .handler = &st_testexitmainloop },
    .{ .name = "testexithandler", .arity_min = 0, .arity_max = null, .handler = &st_testexithandler },
    .{ .name = "testservicemode", .arity_min = 0, .arity_max = null, .handler = &st_testservicemode },
    .{ .name = "teststaticlibrary", .arity_min = 0, .arity_max = null, .handler = &st_teststaticlibrary },
    .{ .name = "testlink", .arity_min = 0, .arity_max = null, .handler = &st_testlink },
    .{ .name = "testlinkarray", .arity_min = 0, .arity_max = null, .handler = &st_testlinkarray },
    .{ .name = "testchannel", .arity_min = 0, .arity_max = null, .handler = &st_testchannel },
    .{ .name = "testchannelevent", .arity_min = 0, .arity_max = null, .handler = &st_testchannelevent },
    .{ .name = "testfilesystem", .arity_min = 0, .arity_max = null, .handler = &st_testfilesystem },
    .{ .name = "testsimplefilesystem", .arity_min = 0, .arity_max = null, .handler = &st_testsimplefilesystem },
    .{ .name = "testfile", .arity_min = 0, .arity_max = null, .handler = &st_testfile },
    .{ .name = "testfilelink", .arity_min = 0, .arity_max = null, .handler = &st_testfilelink },
    .{ .name = "testfstildeexpand", .arity_min = 0, .arity_max = null, .handler = &st_testfstildeexpand },
    .{ .name = "testtranslatefilename", .arity_min = 0, .arity_max = null, .handler = &st_testtranslatefilename },
    .{ .name = "testlocale", .arity_min = 0, .arity_max = null, .handler = &st_testlocale },
    .{ .name = "testasync", .arity_min = 0, .arity_max = null, .handler = &st_testasync },
    .{ .name = "testpanic", .arity_min = 0, .arity_max = null, .handler = &st_testpanic },
    .{ .name = "testhashsystemhash", .arity_min = 0, .arity_max = null, .handler = &st_testhashsystemhash },
    .{ .name = "testhandlecount", .arity_min = 0, .arity_max = null, .handler = &st_testhandlecount },
    .{ .name = "testappverifierpresent", .arity_min = 0, .arity_max = null, .handler = &st_testappverifierpresent },
    .{ .name = "testprint", .arity_min = 0, .arity_max = null, .handler = &st_testprint },
    .{ .name = "testnreunwind", .arity_min = 0, .arity_max = null, .handler = &st_testnreunwind },
    .{ .name = "testnrelevels", .arity_min = 0, .arity_max = null, .handler = &st_testnrelevels },
    .{ .name = "testinterpresolver", .arity_min = 0, .arity_max = null, .handler = &st_testinterpresolver },
    .{ .name = "testapplylambda", .arity_min = 0, .arity_max = null, .handler = &st_testapplylambda },
    .{ .name = "testpreferstable", .arity_min = 0, .arity_max = null, .handler = &st_testpreferstable },
    .{ .name = "testbumpinterpepoch", .arity_min = 0, .arity_max = null, .handler = &st_testbumpinterpepoch },
    .{ .name = "testgetplatform", .arity_min = 0, .arity_max = null, .handler = &st_testgetplatform },
    .{ .name = "testsetplatform", .arity_min = 0, .arity_max = null, .handler = &st_testsetplatform },

    // NOT-YET-PORTED — graduate one by one
    .{ .name = "gettimes", .arity_min = 0, .arity_max = null, .handler = &st_gettimes },
    .{ .name = "testdcall", .arity_min = 0, .arity_max = null, .handler = &st_testdcall },
    .{ .name = "testdel", .arity_min = 0, .arity_max = null, .handler = &st_testdel },
    .{ .name = "testdelassocdata", .arity_min = 0, .arity_max = null, .handler = &st_testdelassocdata },
    .{ .name = "testgetassocdata", .arity_min = 0, .arity_max = null, .handler = &st_testgetassocdata },
    .{ .name = "testsetassocdata", .arity_min = 0, .arity_max = null, .handler = &st_testsetassocdata },
    .{ .name = "testcmdinfo", .arity_min = 0, .arity_max = null, .handler = &st_testcmdinfo },
    .{ .name = "testcmdtoken", .arity_min = 0, .arity_max = null, .handler = &st_testcmdtoken },
    .{ .name = "testcmdtrace", .arity_min = 0, .arity_max = null, .handler = &st_testcmdtrace },
    .{ .name = "testcmdobj2", .arity_min = 0, .arity_max = null, .handler = &st_testcmdobj2 },
    .{ .name = "testcreatecommand", .arity_min = 0, .arity_max = null, .handler = &st_testcreatecommand },
    .{ .name = "testinterpdelete", .arity_min = 0, .arity_max = null, .handler = &st_testinterpdelete },
    .{ .name = "testdstring", .arity_min = 0, .arity_max = null, .handler = &st_testdstring },
    .{ .name = "testencoding", .arity_min = 0, .arity_max = null, .handler = &st_testencoding },
    // testevalex / testevalobjv / testreturn / testseterr / testsetnoerr /
    // testset2 / testseterrorcode / testsetobjerrorcode / testwrongnumargs
    // graduated to cmd_eval.zig
    .{ .name = "testparser", .arity_min = 0, .arity_max = null, .handler = &st_testparser },
    .{ .name = "testparsevar", .arity_min = 0, .arity_max = null, .handler = &st_testparsevar },
    .{ .name = "testparsevarname", .arity_min = 0, .arity_max = null, .handler = &st_testparsevarname },
    .{ .name = "testexprparser", .arity_min = 0, .arity_max = null, .handler = &st_testexprparser },
    .{ .name = "testparseargs", .arity_min = 0, .arity_max = null, .handler = &st_testparseargs },
    .{ .name = "testexprlong", .arity_min = 0, .arity_max = null, .handler = &st_testexprlong },
    .{ .name = "testexprlongobj", .arity_min = 0, .arity_max = null, .handler = &st_testexprlongobj },
    .{ .name = "testexprdouble", .arity_min = 0, .arity_max = null, .handler = &st_testexprdouble },
    .{ .name = "testexprdoubleobj", .arity_min = 0, .arity_max = null, .handler = &st_testexprdoubleobj },
    .{ .name = "testexprstring", .arity_min = 0, .arity_max = null, .handler = &st_testexprstring },
    .{ .name = "testconcatobj", .arity_min = 0, .arity_max = null, .handler = &st_testconcatobj },
    .{ .name = "testpurebytesobj", .arity_min = 0, .arity_max = null, .handler = &st_testpurebytesobj },
    .{ .name = "teststringbytes", .arity_min = 0, .arity_max = null, .handler = &st_teststringbytes },
    .{ .name = "testbytestring", .arity_min = 0, .arity_max = null, .handler = &st_testbytestring },
    .{ .name = "testsetbytearraylength", .arity_min = 0, .arity_max = null, .handler = &st_testsetbytearraylength },
    .{ .name = "testutfnext", .arity_min = 0, .arity_max = null, .handler = &st_testutfnext },
    .{ .name = "testutfprev", .arity_min = 0, .arity_max = null, .handler = &st_testutfprev },
    .{ .name = "testnumutfchars", .arity_min = 0, .arity_max = null, .handler = &st_testnumutfchars },
    .{ .name = "testgetunichar", .arity_min = 0, .arity_max = null, .handler = &st_testgetunichar },
    .{ .name = "testfindfirst", .arity_min = 0, .arity_max = null, .handler = &st_testfindfirst },
    .{ .name = "testfindlast", .arity_min = 0, .arity_max = null, .handler = &st_testfindlast },
    .{ .name = "testuniclass", .arity_min = 0, .arity_max = null, .handler = &st_testuniclass },
    .{ .name = "testdoubledigits", .arity_min = 0, .arity_max = null, .handler = &st_testdoubledigits },
    .{ .name = "testgetint", .arity_min = 0, .arity_max = null, .handler = &st_testgetint },
    .{ .name = "testgetintforindex", .arity_min = 0, .arity_max = null, .handler = &st_testgetintforindex },
    .{ .name = "testgetindexfromobjstruct", .arity_min = 0, .arity_max = null, .handler = &st_testgetindexfromobjstruct },
    .{ .name = "testgetvarfullname", .arity_min = 0, .arity_max = null, .handler = &st_testgetvarfullname },
    .{ .name = "testupvar", .arity_min = 0, .arity_max = null, .handler = &st_testupvar },
    .{ .name = "testregexp", .arity_min = 0, .arity_max = null, .handler = &st_testregexp },
    .{ .name = "testlistrep", .arity_min = 0, .arity_max = null, .handler = &st_testlistrep },
    .{ .name = "testlongsize", .arity_min = 0, .arity_max = null, .handler = &st_testlongsize },
    .{ .name = "testsize", .arity_min = 0, .arity_max = null, .handler = &st_testsize },
    .{ .name = "testlutil", .arity_min = 0, .arity_max = null, .handler = &st_testlutil },
    .{ .name = "testmsb", .arity_min = 0, .arity_max = null, .handler = &st_testmsb },

    // Namespace-prefixed test commands
    .{ .name = "::tcl::test::build-info", .arity_min = 0, .arity_max = null, .handler = &st_test_build_info },
    .{ .name = "test_ns_basic::createdcommand", .arity_min = 0, .arity_max = null, .handler = &st_test_ns_basic_createdcommand },
    .{ .name = "value:at:", .arity_min = 0, .arity_max = null, .handler = &st_value_at },

    // Abstract list demo (tclTestABSList.c)
    .{ .name = "lstring", .arity_min = 0, .arity_max = null, .handler = &st_lstring },
    .{ .name = "lgen", .arity_min = 0, .arity_max = null, .handler = &st_lgen },

    // Procbody (tclTestProcBodyObj.c)
    .{ .name = "tcl::procbodytest::proc", .arity_min = 0, .arity_max = null, .handler = &st_procbody_proc },
    .{ .name = "tcl::procbodytest::check", .arity_min = 0, .arity_max = null, .handler = &st_procbody_check },
};
