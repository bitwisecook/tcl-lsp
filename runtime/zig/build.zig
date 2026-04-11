const std = @import("std");

pub fn build(b: *std.Build) void {
    // Target wasm32-wasi for WASI I/O support (fd_write for puts).
    // Build as a WASI reactor (no _start, exports via rdynamic).
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
    });
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "tcl_runtime",
        .root_source_file = b.path("tcl_runtime.zig"),
        .target = target,
        .optimize = optimize,
    });
    // Export all pub/export functions and mark as reactor (no _start).
    exe.rdynamic = true;
    exe.entry = .disabled;

    b.installArtifact(exe);
}
