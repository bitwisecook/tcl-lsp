// Unit tests for ``interp/tcl_ns.zig`` — the namespace tree
// (root, ``ns_create`` find-or-create, ``ns_full_name`` FQN
// materialisation, qualified-name resolution) and the export-pattern
// machinery used by ``namespace export`` / ``namespace import``.
//
// Wave 5 of the WASM-runtime test suite (#268).
//
// The namespace tree is module-global state: every test binary that
// touches ``ns_root`` shares the same root through the rest of the
// suite.  We design each test to be order-independent by using
// fresh, test-specific child names — the bump allocator never
// recycles, so a leaked child from one case is harmless to the
// next.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const ns = @import("interp/tcl_ns.zig");
const fixture = @import("runtime_test_fixture.zig");

// -- helpers --------------------------------------------------------

fn ptr_of(s: []const u8) u32 {
    return @intCast(@intFromPtr(s.ptr));
}

fn slice_at(p: u32, l: u32) []const u8 {
    if (l == 0) return &[_]u8{};
    const src: [*]const u8 = @ptrFromInt(p);
    return src[0..l];
}

// -- ns_root --------------------------------------------------------

test "ns_root is idempotent and returns a stable address" {
    const r1 = ns.ns_root();
    const r2 = ns.ns_root();
    try testing.expect(r1 != 0);
    try testing.expectEqual(r1, r2);
}

test "ns_root has empty simple name and no parent" {
    const root = ns.ns_root();
    const node: *ns.Namespace = @ptrFromInt(root);
    try testing.expectEqual(@as(u32, 0), node.name_len);
    try testing.expectEqual(@as(u32, 0), node.parent);
}

test "ns_full_name on root returns ::" {
    const root = ns.ns_root();
    const fq = ns.ns_full_name(root);
    try testing.expectEqualStrings("::", slice_at(fq.ptr, fq.len));
}

// -- ns_create / ns_lookup -----------------------------------------

test "ns_create on a fresh name allocates a child and links it" {
    const root = ns.ns_root();
    const name = "test_ns_create_fresh";
    const child = ns.ns_create(root, ptr_of(name), name.len);
    try testing.expect(child != 0);
    try testing.expect(child != root);

    const node: *ns.Namespace = @ptrFromInt(child);
    try testing.expectEqual(@as(u32, root), node.parent);
    try testing.expectEqualStrings(name, slice_at(node.name_ptr, node.name_len));
}

test "ns_create is find-or-create — same name returns the same child" {
    const root = ns.ns_root();
    const name = "test_ns_create_idempotent";
    const c1 = ns.ns_create(root, ptr_of(name), name.len);
    const c2 = ns.ns_create(root, ptr_of(name), name.len);
    try testing.expectEqual(c1, c2);
}

test "ns_lookup returns the registered child" {
    const root = ns.ns_root();
    const name = "test_ns_lookup_hit";
    const created = ns.ns_create(root, ptr_of(name), name.len);
    const found = ns.ns_lookup(root, ptr_of(name), name.len);
    try testing.expectEqual(created, found);
}

test "ns_lookup returns 0 on miss" {
    const root = ns.ns_root();
    const missing = "test_ns_lookup_no_such_child";
    try testing.expectEqual(@as(u32, 0), ns.ns_lookup(root, ptr_of(missing), missing.len));
}

// -- ns_full_name on a child ---------------------------------------

test "ns_full_name on a top-level child collapses ::name (not ::::name)" {
    const root = ns.ns_root();
    const name = "test_ns_full_name_top";
    const child = ns.ns_create(root, ptr_of(name), name.len);
    const fq = ns.ns_full_name(child);
    try testing.expectEqualStrings("::test_ns_full_name_top", slice_at(fq.ptr, fq.len));
}

test "ns_full_name on a deep child concatenates with ::" {
    const root = ns.ns_root();
    const a = "test_ns_full_name_deep_a";
    const b = "b";
    const ns_a = ns.ns_create(root, ptr_of(a), a.len);
    const ns_b = ns.ns_create(ns_a, ptr_of(b), b.len);
    const fq = ns.ns_full_name(ns_b);
    try testing.expectEqualStrings("::test_ns_full_name_deep_a::b", slice_at(fq.ptr, fq.len));
}

test "ns_full_name caches the result — repeat calls return the same buffer" {
    const root = ns.ns_root();
    const name = "test_ns_full_name_cache";
    const child = ns.ns_create(root, ptr_of(name), name.len);
    const fq1 = ns.ns_full_name(child);
    const fq2 = ns.ns_full_name(child);
    try testing.expectEqual(fq1.ptr, fq2.ptr);
    try testing.expectEqual(fq1.len, fq2.len);
}

// -- ns_create_from_fqn --------------------------------------------

test "ns_create_from_fqn walks ::-separated components" {
    const name = "::test_create_from_fqn::level1::level2";
    const target = ns.ns_create_from_fqn(ptr_of(name), name.len);
    try testing.expect(target != 0);
    const fq = ns.ns_full_name(target);
    try testing.expectEqualStrings(
        "::test_create_from_fqn::level1::level2",
        slice_at(fq.ptr, fq.len),
    );
}

test "ns_create_from_fqn on :: alone returns root" {
    const name = "::";
    const target = ns.ns_create_from_fqn(ptr_of(name), name.len);
    try testing.expectEqual(ns.ns_root(), target);
}

test "ns_create_from_fqn collapses repeated colons (Tcl semantics)" {
    // Three or more adjacent colons read as a single separator.
    const name = "::test_create_from_fqn_collapse:::leaf";
    const target = ns.ns_create_from_fqn(ptr_of(name), name.len);
    const fq = ns.ns_full_name(target);
    try testing.expectEqualStrings(
        "::test_create_from_fqn_collapse::leaf",
        slice_at(fq.ptr, fq.len),
    );
}

// -- ns_resolve_qualified -------------------------------------------

test "ns_resolve_qualified on a simple name anchors at the context" {
    const root = ns.ns_root();
    const cxt_name = "test_resolve_simple_cxt";
    const cxt = ns.ns_create(root, ptr_of(cxt_name), cxt_name.len);
    const target_name = "leaf";
    const r = ns.ns_resolve_qualified(cxt, ptr_of(target_name), target_name.len);
    // Simple name → ``target_ns`` is the cxt itself; ``simple_*``
    // points back into the input.
    try testing.expectEqual(cxt, r.target_ns);
    try testing.expectEqualStrings(target_name, slice_at(r.simple_ptr, r.simple_len));
}

test "ns_resolve_qualified on a ::-prefixed name anchors at root" {
    const cxt_name = "test_resolve_qualified_cxt";
    const cxt = ns.ns_create(ns.ns_root(), ptr_of(cxt_name), cxt_name.len);
    const name = "::cmd_at_root";
    const r = ns.ns_resolve_qualified(cxt, ptr_of(name), name.len);
    // ``::``-prefixed name resolves against root regardless of cxt.
    try testing.expectEqual(ns.ns_root(), r.target_ns);
    try testing.expectEqualStrings("cmd_at_root", slice_at(r.simple_ptr, r.simple_len));
}

test "ns_resolve_qualified on :: alone returns root + empty simple name" {
    const name = "::";
    const r = ns.ns_resolve_qualified(ns.ns_root(), ptr_of(name), name.len);
    try testing.expectEqual(ns.ns_root(), r.target_ns);
    try testing.expectEqual(@as(u32, 0), r.simple_len);
}

test "ns_resolve_qualified surfaces the alt-from-root path for unqualified cxt-anchored names" {
    // Per ``tclNamesp.c:2370``-style behaviour, an unqualified name
    // resolved in a non-root cxt also reports an alt anchor at root
    // so callers can probe both spaces.
    const cxt_name = "test_resolve_alt_cxt";
    const cxt = ns.ns_create(ns.ns_root(), ptr_of(cxt_name), cxt_name.len);
    const name = "leaf";
    const r = ns.ns_resolve_qualified(cxt, ptr_of(name), name.len);
    try testing.expectEqual(cxt, r.target_ns);
    try testing.expectEqual(ns.ns_root(), r.alt_ns);
}

// -- ns_export / ns_export_matches ---------------------------------

test "ns_export_matches returns false when no patterns are registered" {
    const cxt_name = "test_export_empty";
    const cxt = ns.ns_create(ns.ns_root(), ptr_of(cxt_name), cxt_name.len);
    const probe = "anything";
    try testing.expect(!ns.ns_export_matches(cxt, ptr_of(probe), probe.len));
}

test "ns_export + ns_export_matches honour glob semantics" {
    const cxt_name = "test_export_glob";
    const cxt = ns.ns_create(ns.ns_root(), ptr_of(cxt_name), cxt_name.len);
    const pat = "foo*";
    ns.ns_export(cxt, ptr_of(pat), pat.len);

    const hit = "foobar";
    const miss = "barfoo";
    try testing.expect(ns.ns_export_matches(cxt, ptr_of(hit), hit.len));
    try testing.expect(!ns.ns_export_matches(cxt, ptr_of(miss), miss.len));
}

test "ns_export accumulates across calls" {
    const cxt_name = "test_export_multi";
    const cxt = ns.ns_create(ns.ns_root(), ptr_of(cxt_name), cxt_name.len);
    const pat_a = "a*";
    const pat_b = "b*";
    ns.ns_export(cxt, ptr_of(pat_a), pat_a.len);
    ns.ns_export(cxt, ptr_of(pat_b), pat_b.len);

    const hit_a = "alpha";
    const hit_b = "beta";
    const miss = "gamma";
    try testing.expect(ns.ns_export_matches(cxt, ptr_of(hit_a), hit_a.len));
    try testing.expect(ns.ns_export_matches(cxt, ptr_of(hit_b), hit_b.len));
    try testing.expect(!ns.ns_export_matches(cxt, ptr_of(miss), miss.len));
}

test "ns_export with empty pattern is a no-op" {
    const cxt_name = "test_export_empty_pattern";
    const cxt = ns.ns_create(ns.ns_root(), ptr_of(cxt_name), cxt_name.len);
    const empty = "";
    ns.ns_export(cxt, ptr_of(empty), 0);
    // No patterns were actually added, so any probe still misses.
    const probe = "xyz";
    try testing.expect(!ns.ns_export_matches(cxt, ptr_of(probe), probe.len));
}

// -- global_set / global_get / global_exists -----------------------

fn body_global_set_get_round_trip() void {
    const name = obj.obj_new_string_copy(@intCast(@intFromPtr("g_value".ptr)), 7);
    _ = ns.global_set(name, obj.obj_new_int(123));
    captured_int = obj.obj_get_int(ns.global_get(name));
    captured_exists = obj.obj_get_int(ns.global_exists(name));
}

var captured_int: i64 = 0;
var captured_exists: i64 = 0;

test "global_set / global_get / global_exists round-trip" {
    captured_int = 0;
    captured_exists = 0;
    fixture.with_interp(&body_global_set_get_round_trip);
    try testing.expectEqual(@as(i64, 123), captured_int);
    try testing.expectEqual(@as(i64, 1), captured_exists);
}

fn body_global_exists_misses() void {
    const name = obj.obj_new_string_copy(@intCast(@intFromPtr("never_set_global".ptr)), 16);
    captured_exists = obj.obj_get_int(ns.global_exists(name));
}

test "global_exists returns 0 on a missing variable" {
    captured_exists = -1;
    fixture.with_interp(&body_global_exists_misses);
    try testing.expectEqual(@as(i64, 0), captured_exists);
}
