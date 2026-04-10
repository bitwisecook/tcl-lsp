const std = @import("std");

pub fn build(b: *std.Build) void {
    // Target wasm32-freestanding for embedding in compiled Tcl modules.
    // Switch to wasm32-wasi when WASI I/O support is added.
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
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
