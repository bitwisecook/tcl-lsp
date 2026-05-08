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
// extension contract.  The complete triage matrix used to live at
// ``docs/design/compiler/wasm-extensions-tcltest.md`` — that doc is
// removed once every command leaves this file.

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
const NOT_YET_PORTED = "not yet ported to the WASM runtime";

// -- NOT-PORTABLE stubs ----------------------------------------------------

fn st_testsocket(w: []const i32) result_mod.InterpResult { return stub_named(w, "testsocket", NOT_SUPPORTED ++ " (BSD sockets unavailable)"); }
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
fn st_testasync(w: []const i32) result_mod.InterpResult { return stub_named(w, "testasync", NOT_SUPPORTED ++ " (Tcl_AsyncMark requires async signal delivery)"); }

// -- NOT-YET-PORTED stubs --------------------------------------------------

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
fn st_testparser(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparser", NOT_YET_PORTED); }
fn st_testparsevar(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparsevar", NOT_YET_PORTED); }
fn st_testparsevarname(w: []const i32) result_mod.InterpResult { return stub_named(w, "testparsevarname", NOT_YET_PORTED); }
fn st_testexprparser(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprparser", NOT_YET_PORTED); }
fn st_testexprlong(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprlong", NOT_YET_PORTED); }
fn st_testexprlongobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprlongobj", NOT_YET_PORTED); }
fn st_testexprdouble(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprdouble", NOT_YET_PORTED); }
fn st_testexprdoubleobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprdoubleobj", NOT_YET_PORTED); }
fn st_testexprstring(w: []const i32) result_mod.InterpResult { return stub_named(w, "testexprstring", NOT_YET_PORTED); }
fn st_testconcatobj(w: []const i32) result_mod.InterpResult { return stub_named(w, "testconcatobj", NOT_YET_PORTED); }
fn st_testgetvarfullname(w: []const i32) result_mod.InterpResult { return stub_named(w, "testgetvarfullname", NOT_YET_PORTED); }
fn st_testupvar(w: []const i32) result_mod.InterpResult { return stub_named(w, "testupvar", NOT_YET_PORTED); }
fn st_testregexp(w: []const i32) result_mod.InterpResult { return stub_named(w, "testregexp", NOT_YET_PORTED); }
fn st_testlistrep(w: []const i32) result_mod.InterpResult { return stub_named(w, "testlistrep", NOT_YET_PORTED); }
fn st_testinterpresolver(w: []const i32) result_mod.InterpResult { return stub_named(w, "testinterpresolver", NOT_YET_PORTED); }
fn st_test_build_info(w: []const i32) result_mod.InterpResult { return stub_named(w, "::tcl::test::build-info", NOT_YET_PORTED); }
fn st_test_ns_basic_createdcommand(w: []const i32) result_mod.InterpResult { return stub_named(w, "test_ns_basic::createdcommand", NOT_YET_PORTED); }
fn st_value_at(w: []const i32) result_mod.InterpResult { return stub_named(w, "value:at:", NOT_YET_PORTED); }
fn st_lstring(w: []const i32) result_mod.InterpResult { return stub_named(w, "lstring", NOT_YET_PORTED); }
fn st_lgen(w: []const i32) result_mod.InterpResult { return stub_named(w, "lgen", NOT_YET_PORTED); }
fn st_procbody_proc(w: []const i32) result_mod.InterpResult { return stub_named(w, "tcl::procbodytest::proc", NOT_YET_PORTED); }
fn st_procbody_check(w: []const i32) result_mod.InterpResult { return stub_named(w, "tcl::procbodytest::check", NOT_YET_PORTED); }

pub const registrations = [_]reg.CmdEntry{
    // NOT-PORTABLE
    .{ .name = "testsocket", .arity_min = 0, .arity_max = null, .handler = &st_testsocket },
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
    .{ .name = "testasync", .arity_min = 0, .arity_max = null, .handler = &st_testasync },

    // NOT-YET-PORTED — graduate one by one
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
    .{ .name = "testparser", .arity_min = 0, .arity_max = null, .handler = &st_testparser },
    .{ .name = "testparsevar", .arity_min = 0, .arity_max = null, .handler = &st_testparsevar },
    .{ .name = "testparsevarname", .arity_min = 0, .arity_max = null, .handler = &st_testparsevarname },
    .{ .name = "testexprparser", .arity_min = 0, .arity_max = null, .handler = &st_testexprparser },
    .{ .name = "testexprlong", .arity_min = 0, .arity_max = null, .handler = &st_testexprlong },
    .{ .name = "testexprlongobj", .arity_min = 0, .arity_max = null, .handler = &st_testexprlongobj },
    .{ .name = "testexprdouble", .arity_min = 0, .arity_max = null, .handler = &st_testexprdouble },
    .{ .name = "testexprdoubleobj", .arity_min = 0, .arity_max = null, .handler = &st_testexprdoubleobj },
    .{ .name = "testexprstring", .arity_min = 0, .arity_max = null, .handler = &st_testexprstring },
    .{ .name = "testconcatobj", .arity_min = 0, .arity_max = null, .handler = &st_testconcatobj },
    .{ .name = "testgetvarfullname", .arity_min = 0, .arity_max = null, .handler = &st_testgetvarfullname },
    .{ .name = "testupvar", .arity_min = 0, .arity_max = null, .handler = &st_testupvar },
    .{ .name = "testregexp", .arity_min = 0, .arity_max = null, .handler = &st_testregexp },
    .{ .name = "testlistrep", .arity_min = 0, .arity_max = null, .handler = &st_testlistrep },
    .{ .name = "testinterpresolver", .arity_min = 0, .arity_max = null, .handler = &st_testinterpresolver },

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

    // Trivial helpers (no-op) — also registered via cmd_misc, but
    // these reach BUILTINS via cmd_stubs to keep the trivial set
    // visible regardless of which cluster owns it next.
    .{ .name = "noop", .arity_min = 0, .arity_max = null, .handler = &st_noop },
};

fn st_noop(w: []const i32) result_mod.InterpResult { _ = w; return result_mod.ok(obj.obj_new_string(0, 0)); }
