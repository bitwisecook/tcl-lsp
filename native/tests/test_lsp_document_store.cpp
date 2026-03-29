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
