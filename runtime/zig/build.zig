const std = @import("std");

pub fn build(b: *std.Build) void {
    // Target wasm32-wasi for WASI I/O support (fd_write for puts).
    // Build as a WASI reactor (no _start, exports via rdynamic).
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
    });
    const optimize = b.standardOptimizeOption(.{});

    // Fetch Tcl 9.0.3's regex engine sources on demand (idempotent
    // via a stamp file; see scripts/fetch_tcl_regex.sh).  Running
    // the script from the repo root keeps its REPO_ROOT resolution
    // correct regardless of the cwd ``zig build`` was invoked from.
    //
    // ``b.build_root.path`` is ``runtime/zig`` here; we climb one
    // level to reach the repo root where ``scripts/`` lives.
    const fetch_regex = b.addSystemCommand(&.{
        "bash",
        "../../scripts/fetch_tcl_regex.sh",
    });
    fetch_regex.setCwd(b.path("."));

    const exe = b.addExecutable(.{
        .name = "tcl_runtime",
        .root_source_file = b.path("tcl_runtime.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Make the C compilation depend on the fetch step — the regex
    // sources must exist before the compiler reads them.  The paths
    // passed to ``addCSourceFiles`` are resolved lazily at the
    // build-graph execution phase, so the files only need to exist
    // by the time the C compile runs, not at ``build.zig`` parse
    // time.
    exe.step.dependOn(&fetch_regex.step);

    // ``regex_include/`` comes first on the include path so
    // ``#include "regcustom.h"`` in ``regguts.h`` resolves to our
    // override rather than the upstream file.  It also provides
    // the minimal ``tclInt.h`` stub that ``regex.h`` pulls in for
    // the ``Tcl_UniChar`` typedef.
    exe.addIncludePath(b.path("regex_include"));
    exe.addIncludePath(b.path("vendor/tcl-regex"));

    // The Spencer engine amalgamates most of its ``.c`` files via
    // ``#include``s in ``regcomp.c`` (regc_*.c) and ``regexec.c``
    // (rege_*.c), so the unit list here is intentionally short.
    // ``regfronts.c`` is included for completeness — ``__REG_NOFRONT``
    // in our ``regcustom.h`` turns it into a no-op compilation
    // unit but keeps the build graph closed if we ever want the
    // char-based front-ends.
    const c_flags = &[_][]const u8{
        "-std=c99",
        "-DNDEBUG",
        // wasi-libc supplies ``stddef.h`` / ``stdint.h`` /
        // ``stdlib.h`` / ``assert.h`` / ``limits.h`` / ``string.h``;
        // all the engine needs beyond those is our shim.
        "-Wno-unused-parameter",
        "-Wno-sign-compare",
        "-Wno-unused-variable",
    };
    // ``regfronts.c`` defines the char-based ``regcomp`` / ``regexec``
    // frontends.  We set ``__REG_NOFRONT`` in regcustom.h to disable
    // them, so compiling this file would build unused (and
    // unresolved: ``re_comp`` / ``re_exec`` are the char variants
    // which are also off) code.  Skip it — upstream's Makefile does
    // the same under ``__REG_NOFRONT``.
    exe.addCSourceFiles(.{
        .root = b.path("vendor/tcl-regex"),
        .files = &.{
            "regcomp.c",
            "regexec.c",
            "regfree.c",
            "regerror.c",
        },
        .flags = c_flags,
    });
    exe.addCSourceFile(.{
        .file = b.path("regex_include/tcl_reg_shim.c"),
        .flags = c_flags,
    });

    // wasi-libc for malloc/free/memcpy/memset/qsort/strlen/str*
    // used by the regex engine, plus ``stdio.h`` / ``stdlib.h`` /
    // ``limits.h`` / ``assert.h`` headers.
    //
    // Linking libc also imports POSIX symbols (``close`` / ``open``
    // / ``read`` / ``gets`` / ``puts`` / ``seek`` / ``tell`` /
    // ``socket`` / ``exec`` / ``source`` / ``load`` / ``glob`` /
    // ``chan`` / ``error``) that share names with Tcl command
    // stubs we were previously exporting unprefixed from Zig.
    // Those stubs have been renamed to ``tcl_cmd_<name>`` — the
    // internal Zig symbol and the WASM export both carry the
    // prefix, and Python's ``_RUNTIME_IMPORTS`` imports them
    // under the same prefixed name.  The user-facing Tcl command
    // name (``close``, ``puts``, etc.) is unchanged — only the
    // internal WASM import wiring moved.
    exe.linkLibC();

    // Export all pub/export functions and mark as reactor.
    // ``wasi_exec_model = .reactor`` tells Zig/wasm-ld to wire
    // wasi-libc's ``crt1-reactor.o`` which exports
    // ``_initialize`` instead of ``_start``; the embedder calls
    // ``_initialize`` after instantiation to run ctors
    // (preopen-fd scan, global locks).  Without this the preopens
    // configured on ``WasiConfig`` are invisible to wasi-libc's
    // path-resolution machinery and every ``access``/``stat``
    // returns ``ENOTCAPABLE``.
    exe.rdynamic = true;
    exe.entry = .disabled;
    exe.wasi_exec_model = .reactor;

    b.installArtifact(exe);
}
