const std = @import("std");

pub fn build(b: *std.Build) void {
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

    // Export all public functions
    lib.rdynamic = true;

    b.installArtifact(lib);
}
