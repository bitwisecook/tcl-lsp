// Wrapper that pulls the inline ``test`` blocks of
// ``valtypes/tcl_bignum.zig`` into the Zig test runner.  The build
// graph collects ``test_*.zig`` files automatically (see
// ``build.zig::collectTestFiles``); inline tests inside source files
// only run when an importer reaches them, so this thin shim lets
// ``zig build test`` execute the bignum unit tests without forcing
// every other importer to host them transitively.

comptime {
    _ = @import("tcl_bignum.zig");
}
