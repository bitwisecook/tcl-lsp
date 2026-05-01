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
    // -Dasyncify=true post-processes the output with binaryen's
    // ``wasm-opt --asyncify`` so calls to ``asyncify_start_unwind`` /
    // ``_stop_unwind`` / ``_start_rewind`` / ``_stop_rewind`` /
    // ``_get_state`` (declared as extern imports in
    // ``sched/tcl_asyncify.zig``) become real stack-saving / -restoring
    // operations.  Stage 2 of the event-loop staircase: enables real
    // ``coroutine`` / ``yield`` semantics with yields from arbitrary
    // call depth.  Cost is ~50% binary size and ~2x runtime overhead,
    // so the flag is off by default; tests that need real coroutines
    // (and CI builds) opt in.
    const asyncify = b.option(
        bool,
        "asyncify",
        "post-process with wasm-opt --asyncify (Stage 2 coroutines)",
    ) orelse false;
    const build_options = b.addOptions();
    build_options.addOption(bool, "leak_check", leak_check);
    build_options.addOption(bool, "asyncify", asyncify);

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

    if (asyncify) {
        // Run wasm-opt --asyncify on the linker output and install
        // the result as ``zig-out/bin/tcl_runtime.wasm``.  DWARF
        // sections trip the asyncify pass on Zig 0.16 debug builds
        // (TODO: DW_LNE_define_file), so strip them first.
        // ``--enable-bulk-memory`` matches the linker's feature set
        // (Zig stdlib uses ``memory.fill``).
        const wasm_opt = b.findProgram(
            &.{"wasm-opt"},
            &.{ "/opt/binaryen/bin", "/usr/local/bin", "/usr/bin" },
        ) catch @panic("wasm-opt (binaryen) not found; install for -Dasyncify=true builds");
        const post = b.addSystemCommand(&.{
            wasm_opt,
            "--strip-dwarf",
            "--enable-bulk-memory",
            "--asyncify",
            // Exclude ``tcl_coro_drive`` from instrumentation so the
            // unwind triggered by ``yield`` stops at the coroutine
            // driver instead of propagating all the way to the host.
            // The driver inspects ``asyncify_get_state()`` after
            // ``eval_script`` returns and reports SUSPENDED to its
            // caller (``eval_proc_call_bucket``).  Resume is
            // symmetric: the driver calls ``asyncify_start_rewind``
            // then re-enters ``eval_script``, which fast-forwards
            // through the saved frames back to the yield site.
            // Internal Zig name (file.module.func mangled form) —
            // wasm-opt's removelist matches on the names section,
            // not on export names.
            "--pass-arg=asyncify-removelist@sched.tcl_coro.tcl_coro_drive",
        });
        post.addArtifactArg(exe);
        post.addArg("-o");
        const out = post.addOutputFileArg("tcl_runtime.wasm");
        b.getInstallStep().dependOn(&post.step);
        const install = b.addInstallFileWithDir(out, .bin, "tcl_runtime.wasm");
        b.getInstallStep().dependOn(&install.step);
    } else {
        b.installArtifact(exe);
    }

    // ---------------------------------------------------------------
    // ``zig build test`` — unit tests for the WASM runtime.
    //
    // The runtime modules speak the wasm32 ABI throughout (``u32``
    // pointer arguments cast via ``@ptrFromInt``), so the tests must
    // be compiled for the same target and run under wasmtime.  Setting
    // ``b.enable_wasmtime = true`` lets ``addRunArtifact`` invoke
    // ``wasmtime`` automatically for the produced ``.wasm`` binaries.
    // ``/usr/local/bin/wasmtime`` is provided by the SessionStart
    // hook in CI / Claude Code on the web.
    // ---------------------------------------------------------------
    b.enable_wasmtime = true;

    const test_target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
    });

    const test_step = b.step(
        "test",
        "Run all WASM runtime unit tests (test_*.zig under valtypes/, parse/, ...)",
    );

    // Auto-discover ``test_*.zig`` files under the build root — drop
    // a new test file under ``valtypes/`` / ``parse/`` / ``cmds/`` /
    // etc. and ``zig build test`` picks it up without a build.zig
    // edit.  ``vendor/`` and ``regex_include/`` are skipped: we don't
    // own those trees and they will never contain our tests.
    const test_files = collectTestFiles(b) catch |err| {
        std.debug.panic("failed to enumerate test_*.zig files: {s}", .{@errorName(err)});
    };

    for (test_files) |file| {
        const t_module = b.createModule(.{
            .root_source_file = b.path(file),
            .target = test_target,
            .optimize = optimize,
            // ``tcl_obj.alloc`` falls back to libc ``malloc`` past
            // the static heap, and the WASI test runner pulls in
            // ``__main_argc_argv`` / ``write`` from wasi-libc — both
            // resolve only when libc is linked.  Match the main
            // runtime's ``.link_libc = true`` so the test wasm
            // instantiates under wasmtime.
            .link_libc = true,
        });
        // ``tcl_obj.zig`` (and a handful of other modules) gates
        // leak-counter bookkeeping behind ``@import("build_options")
        // .leak_check``.  Tests need the same options module wired
        // in or the import resolves to "no module named
        // build_options" and compilation fails — leak_check stays
        // false in tests, matching the production default.
        t_module.addOptions("build_options", build_options);
        const t_exe = b.addTest(.{
            .name = testNameFromPath(b, file),
            .root_module = t_module,
        });
        const run = b.addRunArtifact(t_exe);
        // We're invoking via wasmtime intentionally — silence Zig's
        // foreign-binary safety net so a missing native runner
        // doesn't fail the step before wasmtime gets a chance.
        run.skip_foreign_checks = true;
        test_step.dependOn(&run.step);
    }
}

/// Walk the build root looking for ``test_*.zig`` files.  Returns
/// the matched paths as build-root-relative slices, sorted so the
/// build graph is deterministic across hosts (filesystem walk order
/// otherwise depends on inode ordering).
fn collectTestFiles(b: *std.Build) ![][]const u8 {
    const io = b.graph.io;
    const build_root = b.build_root.path orelse ".";
    var dir = try std.Io.Dir.openDirAbsolute(io, build_root, .{ .iterate = true });
    defer dir.close(io);

    var walker = try dir.walk(b.allocator);
    defer walker.deinit();

    var list: std.ArrayList([]const u8) = .empty;
    while (try walker.next(io)) |entry| {
        if (entry.kind != .file) continue;
        const base = std.fs.path.basename(entry.path);
        if (!std.mem.startsWith(u8, base, "test_")) continue;
        if (!std.mem.endsWith(u8, base, ".zig")) continue;

        // Normalise to forward slashes BEFORE the directory-prefix
        // skip checks so they work identically on Windows
        // (``walker`` returns native separators — ``\`` on Windows,
        // ``/`` on POSIX).  Doing this after the filter would leave
        // Windows hosts walking ``vendor\`` / ``zig-out\`` trees that
        // matched none of the ``/``-suffixed prefix patterns.
        //
        // ``b.allocator`` is the build's arena, so the dup on the
        // skip path is reclaimed wholesale at end-of-build.
        const norm = try b.allocator.dupe(u8, entry.path);
        std.mem.replaceScalar(u8, norm, std.fs.path.sep, '/');

        // Skip vendored / dependency / build-output trees — they
        // won't contain our tests and walking into them slows
        // discovery, especially after ``zig build`` has populated
        // ``zig-out/`` with installed artifacts.  Both top-level
        // (``zig-out/...``) and nested (``deps/foo/zig-out/...``)
        // forms are filtered.
        if (std.mem.startsWith(u8, norm, "vendor/") or
            std.mem.startsWith(u8, norm, "regex_include/") or
            std.mem.startsWith(u8, norm, ".zig-cache/") or
            std.mem.startsWith(u8, norm, "zig-out/") or
            std.mem.indexOf(u8, norm, "/.zig-cache/") != null or
            std.mem.indexOf(u8, norm, "/zig-out/") != null)
        {
            continue;
        }
        try list.append(b.allocator, norm);
    }

    const slice = try list.toOwnedSlice(b.allocator);
    std.mem.sort([]const u8, slice, {}, struct {
        fn lessThan(_: void, a: []const u8, c: []const u8) bool {
            return std.mem.lessThan(u8, a, c);
        }
    }.lessThan);
    return slice;
}

/// Derive a short, unique test-step name from a discovered file
/// path.  ``valtypes/test_tcl_bs.zig`` → ``valtypes-tcl_bs`` so the
/// build summary is readable and Zig 0.16's
/// "looks-like-a-file-path" name validator stays happy.
fn testNameFromPath(b: *std.Build, file: []const u8) []const u8 {
    const without_ext = if (std.mem.endsWith(u8, file, ".zig"))
        file[0 .. file.len - ".zig".len]
    else
        file;
    const buf = b.allocator.dupe(u8, without_ext) catch @panic("OOM");
    // ``test_`` prefix on the basename is implied by the discovery
    // filter, so strip it for brevity.  Convert directory separators
    // to dashes.
    std.mem.replaceScalar(u8, buf, '/', '-');
    const marker = "test_";
    if (std.mem.lastIndexOf(u8, buf, marker)) |idx| {
        const tail = buf[idx + marker.len ..];
        const dir_prefix = if (idx == 0) "" else buf[0 .. idx - 1]; // trim trailing '-'
        if (dir_prefix.len == 0) return tail;
        return std.fmt.allocPrint(b.allocator, "{s}-{s}", .{ dir_prefix, tail }) catch @panic("OOM");
    }
    return buf;
}
