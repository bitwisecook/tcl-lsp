#include "tcl_lsp/lsp/document_store.hpp"

#include <catch2/catch_test_macros.hpp>

// Integration tests for the TclLspServer.
// These test the server skeleton components that don't require
// the Python bridge (document store, capabilities, lifecycle).
// Full end-to-end testing with the Python bridge is done via
// the stdio-based integration test script.

using tcl_lsp::DocumentStore;

TEST_CASE("DocumentStore concurrent access pattern", "[lsp][integration]") {
    DocumentStore store;

    // Simulate rapid open → change → change → close cycle.
    store.open("file:///rapid.tcl", "tcl", "set x 1", 1);
    REQUIRE(store.is_open("file:///rapid.tcl"));

    store.change("file:///rapid.tcl", "set x 2\nset y 3", 2);
    auto src = store.get_source("file:///rapid.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first == "set x 2\nset y 3");
    CHECK(src->second == 2);

    store.change("file:///rapid.tcl", "proc foo {} { return 1 }", 3);
    src = store.get_source("file:///rapid.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first == "proc foo {} { return 1 }");
    CHECK(src->second == 3);

    store.close("file:///rapid.tcl");
    CHECK_FALSE(store.is_open("file:///rapid.tcl"));
}

TEST_CASE("DocumentStore handles many documents", "[lsp][integration]") {
    DocumentStore store;

    // Open 100 documents.
    for (int i = 0; i < 100; ++i) {
        auto uri = "file:///doc_" + std::to_string(i) + ".tcl";
        auto content = "set x " + std::to_string(i);
        store.open(uri, "tcl", content, 1);
    }

    CHECK(store.size() == 100);

    // Verify random access.
    auto src = store.get_source("file:///doc_42.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first == "set x 42");

    // Close all.
    for (int i = 0; i < 100; ++i) {
        store.close("file:///doc_" + std::to_string(i) + ".tcl");
    }
    CHECK(store.size() == 0);
}

TEST_CASE("DocumentStore version monotonicity", "[lsp][integration]") {
    DocumentStore store;
    store.open("file:///ver.tcl", "tcl", "v1", 1);

    // Versions don't have to be monotonic in LSP, but we track them.
    store.change("file:///ver.tcl", "v5", 5);
    auto src = store.get_source("file:///ver.tcl");
    REQUIRE(src.has_value());
    CHECK(src->second == 5);

    store.change("file:///ver.tcl", "v3", 3);
    src = store.get_source("file:///ver.tcl");
    REQUIRE(src.has_value());
    CHECK(src->second == 3);
}

TEST_CASE("DocumentStore re-open after close", "[lsp][integration]") {
    DocumentStore store;
    store.open("file:///reopen.tcl", "tcl", "first", 1);
    store.close("file:///reopen.tcl");
    store.open("file:///reopen.tcl", "tcl-irule", "second", 2);

    auto src = store.get_source("file:///reopen.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first == "second");
    CHECK(store.get_language_id("file:///reopen.tcl") == "tcl-irule");
}

TEST_CASE("DocumentStore empty source", "[lsp][integration]") {
    DocumentStore store;
    store.open("file:///empty.tcl", "tcl", "", 0);

    auto src = store.get_source("file:///empty.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first.empty());
}
