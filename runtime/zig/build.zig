const std = @import("std");

pub fn build(b: *std.Build) void {
    // Target wasm32-wasi for WASI I/O support (fd_write for puts).
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
    });
    const optimize = b.standardOptimizeOption(.{});

    const lib = b.addSharedLibrary(.{
        .name = "tcl_runtime",
        .root_source_file = b.path("tcl_runtime.zig"),
        .target = target,
        .optimize = optimize,
    });
    lib.rdynamic = true;

    b.installArtifact(lib);
}
