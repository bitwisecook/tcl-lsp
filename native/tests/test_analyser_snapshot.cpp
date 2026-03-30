// Phase 4g: incremental analysis with snapshot/restore.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"

#include <catch2/catch_test_macros.hpp>

#include <string>
#include <vector>

using namespace tcl_lsp;

static auto make_registry() -> TestCommandRegistry {
    TestCommandRegistry reg;
    reg.add_command("set", CommandSig{Arity{1, 2}, {}});
    reg.add_command("puts", CommandSig{Arity{1, 2}, {}});
    reg.add_command("proc", CommandSig{Arity{3, 3}, {}});
    reg.add_command("if", CommandSig{Arity{2, 100}, {}});
    reg.add_command("expr", CommandSig{Arity{1, 100}, {{0, ArgRole::EXPR}}});
    reg.add_command("return", CommandSig{Arity{0, 100}, {}});
    return reg;
}

TEST_CASE("snapshot round-trip preserves procs", "[analyser][snapshot]") {
    auto reg = make_registry();
    Analyser analyser(&reg);

    auto chunk1 = segment_commands("proc foo {a b} { return $a }");
    auto chunk2 = segment_commands("proc bar {x} { return $x }");

    // Analyse chunk 1.
    std::vector<std::vector<SegmentedCommand>> chunks{chunk1, chunk2};
    auto [result, snapshots] = analyser.analyse_chunked("", chunks);

    REQUIRE(snapshots.size() == 2);
    // After chunk 1: foo should be defined.
    CHECK(snapshots[0].result.all_procs().size() == 1);
    // After chunk 2: both foo and bar should be defined.
    CHECK(snapshots[1].result.all_procs().size() == 2);
    // Final result has both procs.
    CHECK(result.all_procs().size() == 2);
}

TEST_CASE("restore then re-analyse later chunk", "[analyser][snapshot]") {
    auto reg = make_registry();
    Analyser analyser(&reg);

    auto chunk1 = segment_commands("proc foo {a} { return $a }");
    auto chunk2_v1 = segment_commands("set x 1");
    auto chunk2_v2 = segment_commands("proc baz {} { puts hello }");

    // First pass: analyse both chunks.
    std::vector<std::vector<SegmentedCommand>> chunks_v1{chunk1, chunk2_v1};
    auto [result1, snaps1] = analyser.analyse_chunked("", chunks_v1);
    CHECK(result1.all_procs().size() == 1); // only foo

    // Restore to after chunk 1, re-analyse with only the updated chunk 2.
    analyser.restore(std::move(snaps1[0]));
    std::vector<std::vector<SegmentedCommand>> chunks_v2{chunk2_v2};
    auto [result2, snaps2] = analyser.analyse_chunked("", chunks_v2);
    // After restore + new chunk 2: foo (from snapshot) + baz (new).
    CHECK(result2.all_procs().size() == 2);
}

TEST_CASE("snapshot preserves alias state", "[analyser][snapshot]") {
    auto reg = make_registry();
    reg.add_command("interp", CommandSig{Arity{1, 100}, {}});
    Analyser analyser(&reg);

    auto chunk1 = segment_commands("interp alias {} myput {} puts");
    auto chunk2 = segment_commands("myput hello");

    std::vector<std::vector<SegmentedCommand>> chunks{chunk1, chunk2};
    auto [result, snapshots] = analyser.analyse_chunked("", chunks);

    // The alias should survive the snapshot boundary — myput should
    // resolve to puts and not trigger W123 (if W123 is enabled).
    // Just verify the snapshot captured alias state.
    CHECK(!snapshots[0].alias_state.empty());
}

TEST_CASE("snapshot preserves conditional depth", "[analyser][snapshot]") {
    auto reg = make_registry();
    Analyser analyser(&reg);

    // Simulate conditional context.
    auto chunk1 = segment_commands("set x 1");
    std::vector<std::vector<SegmentedCommand>> chunks{chunk1};
    auto [result, snapshots] = analyser.analyse_chunked("", chunks);
    CHECK(snapshots[0].conditional_depth == 0);
}
