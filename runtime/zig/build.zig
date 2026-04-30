const std = @import("std");

pub fn build(b: *std.Build) void {
    // Target wasm32-wasi for WASI I/O support (fd_write for puts).
    // Build as a WASI reactor (no _start, exports via rdynamic).
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
    });
    const optimize = b.standardOptimizeOption(.{});

    // -Dleak-check=true enables the per-type-tag alloc/free counter
    // in tcl_obj.zig.  Off by default — production builds skip the
    // bookkeeping entirely.  See ``tcl_test_alloc_count`` and
    // ``tcl_test_finalize`` exports for how the test harness reads
    // the result.  Plan: ``docs/design/compiler/wasm-aot-staircase-s0.md``
    // §S0.2.
    const leak_check = b.option(
        bool,
        "leak-check",
        "enable leak counters in tcl_obj.zig (S0.2 debug aid)",
    ) orelse false;
    const build_options = b.addOptions();
    build_options.addOption(bool, "leak_check", leak_check);

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

    // Zig 0.16 split module creation from Compile: the executable
    // wraps a ``root_module`` that carries source/target/link info.
    const root_module = b.createModule(.{
        .root_source_file = b.path("tcl_runtime.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    root_module.addOptions("build_options", build_options);

    // ``regex_include/`` comes first on the include path so
    // ``#include "regcustom.h"`` in ``regguts.h`` resolves to our
    // override rather than the upstream file.  It also provides
    // the minimal ``tclInt.h`` stub that ``regex.h`` pulls in for
    // the ``Tcl_UniChar`` typedef.
    root_module.addIncludePath(b.path("regex_include"));
    root_module.addIncludePath(b.path("vendor/tcl-regex"));

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
    root_module.addCSourceFiles(.{
        .root = b.path("vendor/tcl-regex"),
        .files = &.{
            "regcomp.c",
            "regexec.c",
            "regfree.c",
            "regerror.c",
        },
        .flags = c_flags,
    });
    root_module.addCSourceFile(.{
        .file = b.path("regex_include/tcl_reg_shim.c"),
        .flags = c_flags,
    });

    const exe = b.addExecutable(.{
        .name = "tcl_runtime",
        .root_module = root_module,
    });

    // Make the C compilation depend on the fetch step — the regex
    // sources must exist before the compiler reads them.  The paths
    // passed to ``addCSourceFiles`` are resolved lazily at the
    // build-graph execution phase, so the files only need to exist
    // by the time the C compile runs, not at ``build.zig`` parse
    // time.
    exe.step.dependOn(&fetch_regex.step);

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
