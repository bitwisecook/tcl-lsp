#include "tcl_lsp/lsp/document_store.hpp"

#include <catch2/catch_test_macros.hpp>

using tcl_lsp::DocumentStore;

TEST_CASE("DocumentStore open and get", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "set x 1", 1);

    REQUIRE(store.is_open("file:///test.tcl"));
    REQUIRE(store.size() == 1);

    auto src = store.get_source("file:///test.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first == "set x 1");
    CHECK(src->second == 1);

    CHECK(store.get_language_id("file:///test.tcl") == "tcl");
}

TEST_CASE("DocumentStore change updates source and version", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "set x 1", 1);
    store.change("file:///test.tcl", "set x 2", 2);

    auto src = store.get_source("file:///test.tcl");
    REQUIRE(src.has_value());
    CHECK(src->first == "set x 2");
    CHECK(src->second == 2);
}

TEST_CASE("DocumentStore close removes document", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "set x 1", 1);
    store.close("file:///test.tcl");

    CHECK_FALSE(store.is_open("file:///test.tcl"));
    CHECK(store.size() == 0);
    CHECK_FALSE(store.get_source("file:///test.tcl").has_value());
}

TEST_CASE("DocumentStore get_source on missing URI returns nullopt", "[lsp][document_store]") {
    DocumentStore store;
    CHECK_FALSE(store.get_source("file:///missing.tcl").has_value());
}

TEST_CASE("DocumentStore get_language_id on missing URI returns empty", "[lsp][document_store]") {
    DocumentStore store;
    CHECK(store.get_language_id("file:///missing.tcl").empty());
}

TEST_CASE("DocumentStore multiple documents", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///a.tcl", "tcl", "proc a {} {}", 1);
    store.open("file:///b.tcl", "tcl-irule", "when HTTP_REQUEST {}", 1);

    CHECK(store.size() == 2);
    CHECK(store.get_language_id("file:///a.tcl") == "tcl");
    CHECK(store.get_language_id("file:///b.tcl") == "tcl-irule");

    store.close("file:///a.tcl");
    CHECK(store.size() == 1);
    CHECK(store.is_open("file:///b.tcl"));
}

TEST_CASE("DocumentStore change on missing URI is no-op", "[lsp][document_store]") {
    DocumentStore store;
    store.change("file:///missing.tcl", "new text", 5);
    CHECK(store.size() == 0);
}

// Snapshot tests

TEST_CASE("DocumentStore get_snapshot returns shared buffer", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "set x 1", 1);

    auto snap = store.get_snapshot("file:///test.tcl");
    REQUIRE(snap != nullptr);
    CHECK(snap->source() == "set x 1");
    CHECK(snap->version() == 1);
    CHECK(snap->epoch() > 0);
}

TEST_CASE("DocumentStore get_snapshot returns nullptr for missing", "[lsp][document_store]") {
    DocumentStore store;
    CHECK(store.get_snapshot("file:///missing.tcl") == nullptr);
}

TEST_CASE("DocumentStore snapshot is immutable after mutation", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "set x 1", 1);
    auto snap1 = store.get_snapshot("file:///test.tcl");

    store.change("file:///test.tcl", "set x 2", 2);
    auto snap2 = store.get_snapshot("file:///test.tcl");

    // snap1 still sees old content (immutable)
    CHECK(snap1->source() == "set x 1");
    CHECK(snap1->version() == 1);

    CHECK(snap2->source() == "set x 2");
    CHECK(snap2->version() == 2);

    // Epochs are distinct and monotonically increasing.
    CHECK(snap2->epoch() > snap1->epoch());
}

TEST_CASE("DocumentStore epoch increments on every mutation", "[lsp][document_store]") {
    DocumentStore store;
    auto e0 = store.epoch();

    store.open("file:///a.tcl", "tcl", "a", 1);
    auto e1 = store.epoch();
    CHECK(e1 > e0);

    store.change("file:///a.tcl", "b", 2);
    auto e2 = store.epoch();
    CHECK(e2 > e1);

    store.close("file:///a.tcl");
    auto e3 = store.epoch();
    CHECK(e3 > e2);
}

TEST_CASE("DocumentStore snapshot survives document close", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "hello", 1);
    auto snap = store.get_snapshot("file:///test.tcl");

    store.close("file:///test.tcl");

    // Snapshot is still valid even though document is closed.
    REQUIRE(snap != nullptr);
    CHECK(snap->source() == "hello");
    CHECK(snap->version() == 1);
}

TEST_CASE("DocumentStore snapshot position queries work", "[lsp][document_store]") {
    DocumentStore store;
    store.open("file:///test.tcl", "tcl", "set x 1\nset y 2", 1);
    auto snap = store.get_snapshot("file:///test.tcl");
    REQUIRE(snap != nullptr);

    // Line 1, character 4 ("y" in "set y 2")
    auto pos = snap->offset_to_position(12);
    CHECK(pos.line == 1);
    CHECK(pos.character == 4);

    auto offset = snap->position_to_offset(1, 4);
    CHECK(offset == 12);
}

TEST_CASE("DocumentStore change on missing URI does not bump epoch", "[lsp][document_store]") {
    DocumentStore store;
    auto e0 = store.epoch();
    store.change("file:///missing.tcl", "text", 1);
    CHECK(store.epoch() == e0);
}
