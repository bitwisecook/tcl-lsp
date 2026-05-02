// Namespace tree — root + child links + per-ns sub-tables.
//
// Phase P1.2 of the runtime correctness rework
// (``docs/design/runtime/namespace-tree.md``).  This module owns
// the ``Namespace`` struct and the root-ns singleton.  Nothing in
// the runtime calls into it yet — P1.3 wires
// ``tcl_ns_set``/``tcl_ns_restore`` to a real ``*Namespace``, and
// P2 flips ``proc_lookup`` to walk the tree.
//
// Storage model: every sub-table (child / cmd / var) is a
// ``hash_table.Table(NS_BUCKET_SIZE)``.  Bucket payload is a single
// u32 — a child Namespace*, a Command*, or a Var* respectively.
// All addresses are absolute u32 offsets into the bump-allocated
// linear memory managed by ``tcl_obj.zig``.
//
// Why ``extern struct``: we hand bare ``u32`` addresses across the
// runtime (matching the rest of the WASM ABI) and need a
// guaranteed layout so a ``@ptrFromInt`` cast points at the right
// fields without padding surprises.

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("../valtypes/hash_table.zig");
const tcl_array = @import("../valtypes/tcl_array.zig");

/// Sub-table bucket size.  All three (child / cmd / var) tables use
/// the same 12-byte header + 4-byte u32 value layout, which keeps
/// ``Table(16)`` monomorphised once and shared between them.
pub const NS_BUCKET_SIZE: u32 = 16;

/// Initial sub-table capacity.  Most namespaces have only a handful
/// of children / commands; 16 buckets keeps the load factor low
/// without wasting much linear memory.  ``Table.grow`` doubles on
/// demand.
const NS_INITIAL_CAP: u32 = 16;

/// Offset into a sub-table bucket where the value (handle) lives.
/// Equal to ``ht.HEADER_SIZE`` by construction; named for clarity
/// at call sites that read / write the handle.
pub const OFF_HANDLE: u32 = ht.HEADER_SIZE;

const ChildTable = ht.Table(NS_BUCKET_SIZE);
const CmdTable = ht.Table(NS_BUCKET_SIZE);
const VarTable = ht.Table(NS_BUCKET_SIZE);

/// Mirror of C Tcl 9's ``Namespace`` (tclInt.h:271), trimmed to the
/// fields we actually consume.  See ``docs/design/runtime/namespace-tree.md``
/// §3 for the full mapping and §4 for the deferred fields.
pub const Namespace = extern struct {
    /// Simple (unqualified) name.  ``(name_ptr, name_len)`` points
    /// into a heap-copied byte buffer owned by this struct.  Both
    /// zero for the root ns (whose simple name is the empty string).
    name_ptr: u32,
    name_len: u32,

    /// ``::``-prefixed FQN, lazily materialised from the parent
    /// chain.  Both zero until first use.  P1.2 doesn't read this
    /// — populated later when we add ``[namespace current]``.
    full_name_ptr: u32,
    full_name_len: u32,

    /// Enclosing namespace, as an absolute address.  Zero only for
    /// the root.
    parent: u32,

    /// Sub-tables.  Lazily initialised on first insert; ``buf == 0``
    /// is the "empty" marker shared with ``hash_table.Table``.
    child_table: ChildTable,
    cmd_table: CmdTable,
    var_table: VarTable,

    /// ``namespace export`` patterns.  P4.1 fills in.
    export_patterns: u32,
    export_pattern_count: u32,

    /// ``namespace path`` ordered targets.  P5.1 fills in.
    path_array: u32,
    path_len: u32,

    /// Bumped on every command add / delete in this ns; cascaded
    /// through ``path_source_head`` to invalidate dependents.  P5.3.
    cmd_ref_epoch: u32,

    /// Doubly-linked list head of ``NamespacePathEntry`` nodes
    /// whose ``target_ns`` is this ns.  P5 wires it.
    path_source_head: u32,

    /// NS_DYING / NS_DEAD / NS_TEARDOWN bits.  Stays zero in our
    /// runtime — we have no namespace deletion path (see design
    /// doc §4 "Refcounting / cleanup").
    flags: u32,
};

/// Address of the current interpreter's root namespace.  The child-
/// interp wave moved the hidden-commands table (and other per-interp
/// state) into ``tcl_interp_registry.Interp``; the root namespace
/// address itself lives here so ``ns_root()`` can stay call-cycle-
/// free.  ``tcl_interp_registry`` swaps this global when entering a
/// child interp via ``interp eval``.
///
/// Zero before the first ``ns_root()`` call (lazy allocation).
pub var root_addr: u32 = 0;

/// Currently-active namespace handle.  Zero means "no explicit
/// context set" — readers should treat that as root.  Compiled
/// procs flip this via ``ns_set`` / ``ns_restore`` (in
/// ``tcl_interp.zig``) before dispatching into the interpreter so
/// dynamic ``proc $name body`` lands in the enclosing ns.
///
/// Public so that ``tcl_interp.zig`` can write through it without a
/// circular dependency loop (interp imports ns; if ns imported
/// interp we'd cycle).
pub var current_ns: u32 = 0;

/// The "current namespace" with the zero-means-root convention
/// applied — every internal caller wanting a non-zero handle should
/// go through this rather than reading ``current_ns`` directly.
pub fn ns_current() u32 {
    return if (current_ns != 0) current_ns else ns_root();
}

/// Allocate a fresh, empty root namespace (no parent, empty simple
/// name).  Used by ``tcl_interp_registry.child_create`` to give every
/// child interpreter its own namespace tree.  The root-interp case
/// goes through ``ns_root()`` below; this helper is the multi-root
/// analogue for child interps.
pub fn ns_alloc_root() u32 {
    return alloc_namespace();
}

/// Allocate and zero-initialise a new ``Namespace`` at a fresh
/// linear-memory address.  Sub-tables start with ``buf == 0`` (will
/// init lazily on first insert).
fn alloc_namespace() u32 {
    const size: u32 = @sizeOf(Namespace);
    const addr = alloc(size);
    // ``alloc`` doesn't zero, so wipe the whole struct.  This is
    // important for the sub-table ``buf`` fields — a non-zero ``buf``
    // would make ``Table.init`` skip allocation and the
    // ``find``/``insert`` paths would dereference garbage.
    const slice: [*]u8 = @ptrFromInt(addr);
    @memset(slice[0..size], 0);
    return addr;
}

/// Copy ``len`` bytes of ``name`` onto the heap and stash the
/// pointer + length on the namespace's ``name_*`` fields.  The
/// source bytes may live anywhere — typically a TclObj string slab
/// that could be reclaimed; we always own a copy.
fn install_name(ns_addr: u32, name_ptr: u32, name_len: u32) void {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (name_len == 0) {
        ns.name_ptr = 0;
        ns.name_len = 0;
        return;
    }
    const buf = alloc(name_len);
    memcpy(buf, name_ptr, name_len);
    ns.name_ptr = buf;
    ns.name_len = name_len;
}

/// Return the root (global) namespace address.  Idempotent — a
/// second call returns the same address.  All other module-private
/// helpers assume the root has been initialised before they're
/// reached, so callers that touch any ``Namespace`` should call
/// ``ns_root()`` once at startup (or rely on the pattern of
/// "every ns ultimately descends from root, so creating the first
/// child triggers root creation").
pub fn ns_root() u32 {
    if (root_addr != 0) return root_addr;
    root_addr = alloc_namespace();
    // Root has empty simple name and no parent.  Sub-tables stay
    // zero until first insert.
    return root_addr;
}

/// Find a direct child of ``parent`` with simple name
/// ``(name_ptr, name_len)``.  Returns 0 if not present.
pub fn ns_lookup(parent: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(parent);
    if (ns.child_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.child_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

/// Find-or-create.  Returns the existing child if present, else
/// allocates a new ``Namespace``, links it under ``parent``, and
/// returns its address.  Sub-tables of the new child are empty.
pub fn ns_create(parent: u32, name_ptr: u32, name_len: u32) u32 {
    const existing = ns_lookup(parent, name_ptr, name_len);
    if (existing != 0) return existing;

    const ns: *Namespace = @ptrFromInt(parent);
    ns.child_table.init(NS_INITIAL_CAP);
    if (ns.child_table.needs_grow()) ns.child_table.grow();

    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = ns.child_table.insert_header(name_ptr, name_len, hash);

    const child_addr = alloc_namespace();
    install_name(child_addr, name_ptr, name_len);
    const child: *Namespace = @ptrFromInt(child_addr);
    child.parent = parent;

    write_i32(bucket + OFF_HANDLE, @bitCast(child_addr));
    return child_addr;
}

/// Walk a fully-qualified namespace name like ``::tcltest`` or
/// ``::a::b::c`` and find-or-create each component starting from
/// the root.  Returns the address of the deepest namespace.
///
/// This is a P1.3 precursor to the full
/// ``TclGetNamespaceForQualName``-style resolver landing in P1.4 —
/// it handles only fully-qualified inputs (the form compiled procs
/// emit when stamping their enclosing namespace).  Non-``::``-
/// prefixed input is treated as relative to root, matching the
/// "stamped FQN" use case rather than the more general C semantics.
///
/// An empty name (or ``::`` alone) returns ``ns_root()``.
pub fn ns_create_from_fqn(name_ptr: u32, name_len: u32) u32 {
    var ns = ns_root();
    if (name_len == 0) return ns;
    const src: [*]const u8 = @ptrFromInt(name_ptr);

    // Skip leading ``::`` (zero, one, or repeated colons all read
    // as "anchor at root", matching Tcl's behaviour where two or
    // more adjacent colons are treated as a single ns separator).
    var i: u32 = 0;
    while (i < name_len and src[i] == ':') : (i += 1) {}

    // Walk components separated by one-or-more ``:``s.
    while (i < name_len) {
        // Find the end of this component (start of next ``:`` run
        // or end of string).
        var j: u32 = i;
        while (j < name_len and src[j] != ':') : (j += 1) {}
        const comp_len: u32 = j - i;
        if (comp_len > 0) {
            ns = ns_create(ns, name_ptr + i, comp_len);
        }
        // Skip the ``:`` run.
        i = j;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }
    return ns;
}

/// Build a fresh ``::ns::simple`` byte buffer.  Used by command-
/// registration paths (``tcl_rename``, ``tcl_alias``) that need to
/// stamp the full qualified name onto a ``Command.name`` slot.
/// Root-ns children collapse ``::`` + ``name`` rather than producing
/// ``::::name``, matching C Tcl's
/// ``Tcl_GetCommandFullName`` formatting.
///
/// Returns ``(buf_ptr, buf_len)`` — both the address of the newly-
/// allocated byte buffer and its total length.  The caller owns
/// the result (the bump allocator never frees, so "owns" here is
/// the usual "may read forever" contract).
pub fn ns_build_fqn(target_ns: u32, simple_ptr: u32, simple_len: u32) struct { ptr: u32, len: u32 } {
    const parent_full = ns_full_name(target_ns);
    const parent_is_root = parent_full.len == 2;
    const total: u32 = if (parent_is_root) 2 + simple_len else parent_full.len + 2 + simple_len;
    const buf = alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    if (parent_is_root) {
        dst[0] = ':';
        dst[1] = ':';
        off = 2;
    } else {
        const ps: [*]const u8 = @ptrFromInt(parent_full.ptr);
        for (0..parent_full.len) |k| dst[k] = ps[k];
        dst[parent_full.len] = ':';
        dst[parent_full.len + 1] = ':';
        off = parent_full.len + 2;
    }
    if (simple_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(simple_ptr);
        for (0..simple_len) |k| dst[off + k] = sp[k];
    }
    return .{ .ptr = buf, .len = total };
}

/// Materialise the namespace's fully-qualified name (``::a::b``) on
/// demand and cache it in ``full_name_*``.  Root returns ``::``.
/// Subsequent calls return the cached pointer + length.
pub fn ns_full_name(ns_addr: u32) struct { ptr: u32, len: u32 } {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (ns.full_name_ptr != 0 or ns.full_name_len != 0) {
        return .{ .ptr = ns.full_name_ptr, .len = ns.full_name_len };
    }

    if (ns.parent == 0) {
        // Root namespace: full name is the literal ``::``.
        const buf = alloc(2);
        const dst: [*]u8 = @ptrFromInt(buf);
        dst[0] = ':';
        dst[1] = ':';
        ns.full_name_ptr = buf;
        ns.full_name_len = 2;
        return .{ .ptr = buf, .len = 2 };
    }

    // ``<parent_full>::<simple>``.  Materialise parent first; if
    // parent is root, its full name is ``::`` and we'd produce
    // ``::::name`` if we naively concatenated, so collapse the case.
    const parent_full = ns_full_name(ns.parent);
    const total: u32 = blk: {
        if (parent_full.len == 2) {
            // Parent is root → ``::name``.
            break :blk 2 + ns.name_len;
        }
        // Parent is anything else → ``<parent>::<name>``.
        break :blk parent_full.len + 2 + ns.name_len;
    };
    const buf = alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    if (parent_full.len == 2) {
        dst[0] = ':';
        dst[1] = ':';
        off = 2;
    } else {
        const ps: [*]const u8 = @ptrFromInt(parent_full.ptr);
        for (0..parent_full.len) |k| dst[k] = ps[k];
        dst[parent_full.len] = ':';
        dst[parent_full.len + 1] = ':';
        off = parent_full.len + 2;
    }
    if (ns.name_len > 0) {
        const np: [*]const u8 = @ptrFromInt(ns.name_ptr);
        for (0..ns.name_len) |k| dst[off + k] = np[k];
    }
    ns.full_name_ptr = buf;
    ns.full_name_len = total;
    return .{ .ptr = buf, .len = total };
}

/// Result of a qualified-name resolution.  Matches the four
/// out-arguments of C Tcl's ``TclGetNamespaceForQualName`` minus
/// ``actualCxtPtrPtr`` (always derivable as ``cxt`` after the
/// global-anchoring shortcut).
///
/// ``target_ns`` and ``alt_ns`` use 0 as "not present" rather than
/// option types so the struct is plain ``extern``-friendly.  Same
/// for ``simple_*`` — zero pointer with zero length means "qualified
/// name was ``::`` alone" (or trailing ``::``), the resolution
/// landed on a namespace itself.
pub const QualifiedResult = extern struct {
    target_ns: u32,
    simple_ptr: u32,
    simple_len: u32,
    alt_ns: u32,
};

/// Resolve a (possibly qualified) name within a context namespace
/// — the find-only variant.  Mirrors ``TclGetNamespaceForQualName``
/// (``tclNamesp.c:2272``) without the create-on-miss flag.
///
/// Inputs are byte ranges into linear memory; the returned
/// ``simple_ptr`` is a pointer **into the caller's input buffer**,
/// not a heap copy.  Callers must keep ``name_ptr`` alive while
/// they consume the result.
///
/// Behaviour summary (see ``docs/design/runtime/namespace-tree.md``
/// §5.1):
///
/// * ``::``-prefixed names anchor at root; non-prefixed names anchor
///   at ``cxt``.
/// * The "alt" search path runs in parallel from root whenever the
///   primary path doesn't already start there — same use as
///   ``Tcl_FindCommand`` looking up a partially-qualified name in
///   both the current ns and the global ns.
/// * Missing intermediates set the corresponding handle to 0; the
///   walk still continues so we always report the trailing simple
///   name in ``simple_*``.
/// * ``::`` alone (or empty input) returns root with empty
///   ``simple_*`` and ``alt_ns == 0``.
pub fn ns_resolve_qualified(cxt: u32, name_ptr: u32, name_len: u32) QualifiedResult {
    var result: QualifiedResult = .{
        .target_ns = 0,
        .simple_ptr = 0,
        .simple_len = 0,
        .alt_ns = 0,
    };

    const root = ns_root();

    // 1. Anchor.  ``::``-prefixed → root; else ``cxt`` (or root if
    //    no context was given).
    var ns: u32 = if (cxt != 0) cxt else root;
    var i: u32 = 0;
    if (name_len >= 2) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        if (src[0] == ':' and src[1] == ':') {
            ns = root;
            i = 2;
            // Skip any further ``:`` (Tcl treats two-or-more
            // adjacent colons as a single separator).
            while (i < name_len and src[i] == ':') : (i += 1) {}
        }
    }

    // 2. Set up the alternate "from root" path — only meaningful
    //    when the primary path is NOT already root.
    var alt: u32 = if (ns == root) 0 else root;

    // 3. Trivial case: no remaining content → resolution landed on
    //    ``ns`` itself with an empty simple name.
    if (i >= name_len) {
        result.target_ns = ns;
        // alt stays 0 — this matches C's "altNsPtrPtr = NULL" for
        // the ``::``-only special case (tclNamesp.c:2370-2375).
        return result;
    }

    // 4. Walk components.  Each iteration consumes one
    //    ``::``-delimited component; the last component (no
    //    trailing ``::``) becomes the simple name.
    const src: [*]const u8 = @ptrFromInt(name_ptr);
    while (i < name_len) {
        // Find component end: position of the next ``::`` or end of
        // input.  We look for a single ``:`` byte first, then
        // confirm a second follows; a lone ``:`` inside a name is
        // legal in Tcl (rare — usually appears in ensembled cmd
        // names) and should NOT be treated as a separator.
        var end: u32 = i;
        var has_sep = false;
        while (end < name_len) : (end += 1) {
            if (src[end] == ':' and end + 1 < name_len and src[end + 1] == ':') {
                has_sep = true;
                break;
            }
        }
        const comp_len: u32 = end - i;

        if (!has_sep) {
            // Last component — the simple name.
            result.target_ns = ns;
            result.simple_ptr = name_ptr + i;
            result.simple_len = comp_len;
            result.alt_ns = alt;
            return result;
        }

        // Intermediate — descend in primary and alt paths.  A 0
        // handle on either side means we're already off-tree; a
        // ``find`` miss flips the side to 0 too.
        if (comp_len > 0) {
            if (ns != 0) {
                ns = child_lookup(ns, name_ptr + i, comp_len);
            }
            if (alt != 0) {
                alt = child_lookup(alt, name_ptr + i, comp_len);
            }
        }

        // Advance past the ``::`` separator and any extra colons.
        i = end + 2;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }

    // 5. Trailing ``::`` — name was ``::a::`` or similar.  ``ns``
    //    is the namespace itself; simple name is empty.
    result.target_ns = ns;
    result.alt_ns = alt;
    return result;
}

/// Same shape as ``ns_resolve_qualified`` but find-or-creates each
/// intermediate namespace along the walk.  Used by registration
/// paths (``proc_register`` dual-write, ``namespace eval``, etc.)
/// where a missing intermediate should be materialised, not
/// reported as a miss.
///
/// ``alt_ns`` is always 0 in the result — when we're creating, the
/// "search-from-root alternate" doesn't apply (we already committed
/// to a single tree location).
pub fn ns_resolve_qualified_creating(cxt: u32, name_ptr: u32, name_len: u32) QualifiedResult {
    var result: QualifiedResult = .{
        .target_ns = 0,
        .simple_ptr = 0,
        .simple_len = 0,
        .alt_ns = 0,
    };

    const root = ns_root();
    var ns: u32 = if (cxt != 0) cxt else root;
    var i: u32 = 0;
    if (name_len >= 2) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        if (src[0] == ':' and src[1] == ':') {
            ns = root;
            i = 2;
            while (i < name_len and src[i] == ':') : (i += 1) {}
        }
    }

    if (i >= name_len) {
        result.target_ns = ns;
        return result;
    }

    const src: [*]const u8 = @ptrFromInt(name_ptr);
    while (i < name_len) {
        var end: u32 = i;
        var has_sep = false;
        while (end < name_len) : (end += 1) {
            if (src[end] == ':' and end + 1 < name_len and src[end + 1] == ':') {
                has_sep = true;
                break;
            }
        }
        const comp_len: u32 = end - i;

        if (!has_sep) {
            result.target_ns = ns;
            result.simple_ptr = name_ptr + i;
            result.simple_len = comp_len;
            return result;
        }

        if (comp_len > 0) {
            ns = ns_create(ns, name_ptr + i, comp_len);
        }

        i = end + 2;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }

    result.target_ns = ns;
    return result;
}

/// Insert (or update) a command in ``ns.cmd_table`` keyed by simple
/// name.  Returns the bucket base so the caller can later read /
/// rewrite the value.  P2.x stores the corresponding flat
/// ``proc_table`` bucket address as the value; P3+/P4 may swap to
/// a real ``Command`` struct.
///
/// The key bytes are heap-copied by ``Table.insert_header`` so the
/// caller's input buffer can be released after the call.
pub fn ns_cmd_put(ns_addr: u32, name_ptr: u32, name_len: u32, value: u32) u32 {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    ns.cmd_table.init(NS_INITIAL_CAP);
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        write_i32(bucket + OFF_HANDLE, @bitCast(value));
        // Update-in-place still counts as a mutation — re-binding
        // the same name to a new Command shadows what dependents
        // may have cached.  Bump the epoch.
        bump_cmd_ref_epoch(ns_addr);
        return bucket;
    }
    if (ns.cmd_table.needs_grow()) ns.cmd_table.grow();
    const bucket = ns.cmd_table.insert_header(name_ptr, name_len, hash);
    write_i32(bucket + OFF_HANDLE, @bitCast(value));
    bump_cmd_ref_epoch(ns_addr);
    return bucket;
}

/// Find a command in ``ns.cmd_table`` by simple name.  Returns the
/// stored value (i.e. flat-bucket address while P2.x is in dual-
/// write mode) or 0 if absent.
pub fn ns_cmd_find(ns_addr: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

/// Recursive walker over every namespace reachable from ``ns`` via
/// the ``child_table`` links.  For each populated ``cmd_table``
/// bucket encountered, invokes ``visit(ctx, ns, name_ptr, name_len, cmd)``.
/// ``cmd == 0`` entries (tombstones from ``ns_cmd_clear`` / delete)
/// are skipped before the visitor sees them.
///
/// The visitor is a comptime function pointer so the generic
/// ``ctx: anytype`` stays monomorphised per call site — no vtable,
/// no dispatch overhead.  Callers use the visitor for both sizing
/// and filling passes (``interp aliases``, any future ns-tree
/// introspection walker) by toggling behaviour on a field inside
/// ``ctx``.
///
/// The ``interp hidden`` walker does NOT use this helper — hidden
/// commands live in the interpreter-wide flat table, not the ns
/// tree, so they have a different traversal shape.
pub fn walk_tree_cmd_tables(
    ns: u32,
    ctx: anytype,
    comptime visit: fn (@TypeOf(ctx), u32, u32, u32, u32) void,
) void {
    const n: *const Namespace = @ptrFromInt(ns);
    if (n.cmd_table.buf != 0) {
        const bucket_size: u32 = 16;
        var i: u32 = 0;
        while (i < n.cmd_table.cap) : (i += 1) {
            const bucket = n.cmd_table.buf + i * bucket_size;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const cmd: u32 = @bitCast(read_i32(bucket + OFF_HANDLE));
            if (cmd == 0) continue;
            const name_len: u32 = @bitCast(read_i32(bucket + 4));
            visit(ctx, ns, name_ptr, name_len, cmd);
        }
    }
    if (n.child_table.buf != 0) {
        const bucket_size: u32 = 16;
        var i: u32 = 0;
        while (i < n.child_table.cap) : (i += 1) {
            const bucket = n.child_table.buf + i * bucket_size;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const child: u32 = @bitCast(read_i32(bucket + OFF_HANDLE));
            if (child != 0) walk_tree_cmd_tables(child, ctx, visit);
        }
    }
}

/// Clear the value of a cmd_table bucket so future ``ns_cmd_find``
/// calls return 0 for this name.  The bucket's header (name / hash)
/// stays populated so probe chains aren't broken — the same
/// "tombstone via zero value" pattern ``ns_forget`` uses for dead
/// redirect commands.  Bumps ``cmd_ref_epoch`` + cascades through
/// ``path_source_head`` so dependents see the clearing.
///
/// Used by the rename wave to retire an old name after the Command
/// has been re-inserted under a new one, and by the delete-via-
/// rename-to-empty path.  Returns true if an entry was cleared
/// (i.e. the name was present), false if it wasn't there.
pub fn ns_cmd_clear(ns_addr: u32, name_ptr: u32, name_len: u32) bool {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return false;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        if (read_i32(bucket + OFF_HANDLE) == 0) return false;
        write_i32(bucket + OFF_HANDLE, 0);
        bump_cmd_ref_epoch(ns_addr);
        return true;
    }
    return false;
}

/// Full command resolution mirroring ``Tcl_FindCommand``
/// (``tclNamesp.c:2631``).  Returns the bucket value (flat
/// proc_table bucket address while P2.x is dual-write — eventually
/// a ``*Command`` once P2.4 retires the flat table) or 0 if not
/// found.  See ``docs/design/runtime/namespace-tree.md`` §5.2.
///
/// Resolution order:
///
/// * If the name contains ``::`` (qualified or fully-qualified):
///   walk ``ns_resolve_qualified`` and probe both the primary and
///   alt target's ``cmd_table`` for the simple trailing name.
/// * Otherwise (unqualified): probe ``cxt.cmd_table``, then fall
///   back to root's.  P5 inserts the ``commandPathArray`` walk
///   between these two steps.
///
/// Pure read — no mutation of the tree.  Safe to call from any
/// resolution hot path.
pub fn ns_find_command(cxt: u32, name_ptr: u32, name_len: u32) u32 {
    const root = ns_root();
    const start: u32 = if (cxt != 0) cxt else root;

    // Detect qualification: any ``::`` substring, including a
    // leading one.  Cheap linear scan — names are short.
    var has_colons = false;
    if (name_len >= 2) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        var i: u32 = 0;
        while (i + 1 < name_len) : (i += 1) {
            if (src[i] == ':' and src[i + 1] == ':') {
                has_colons = true;
                break;
            }
        }
    }

    if (has_colons) {
        const r = ns_resolve_qualified(start, name_ptr, name_len);
        if (r.simple_len == 0) return 0; // ``::``-only or trailing ``::``
        if (r.target_ns != 0) {
            const v = ns_cmd_find(r.target_ns, r.simple_ptr, r.simple_len);
            if (v != 0) return v;
        }
        if (r.alt_ns != 0) {
            const v = ns_cmd_find(r.alt_ns, r.simple_ptr, r.simple_len);
            if (v != 0) return v;
        }
        return 0;
    }

    // Unqualified resolution (mirrors ``Tcl_FindCommand``):
    //   1. Context ns ``cmd_table``.
    //   2. Each entry of the context's ``commandPathArray`` in
    //      declaration order (P5.2).
    //   3. Root ns ``cmd_table``.
    if (start != 0) {
        const v = ns_cmd_find(start, name_ptr, name_len);
        if (v != 0) return v;

        const plen = ns_path_len(start);
        var pi: u32 = 0;
        while (pi < plen) : (pi += 1) {
            const e = ns_path_entry(start, pi);
            if (e.target_ns == 0) continue;
            // Skip the path entry pointing back at the context
            // ns itself — already probed in step 1.
            if (e.target_ns == start) continue;
            const pv = ns_cmd_find(e.target_ns, name_ptr, name_len);
            if (pv != 0) return pv;
        }
    }
    if (start != root) {
        const v = ns_cmd_find(root, name_ptr, name_len);
        if (v != 0) return v;
    }
    return 0;
}

/// Lower-level child lookup: returns the child handle, or 0 if not
/// present.  Splits out so ``ns_resolve_qualified`` doesn't have to
/// recompute the hash for every step (which it doesn't anyway, but
/// the dedicated inline keeps the hot path tidy).
inline fn child_lookup(parent: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(parent);
    if (ns.child_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.child_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

// -- Variables (P3.x) -------------------------------------------------------

/// Mirror of C Tcl 9's ``Var`` (``tclInt.h:637``), trimmed to the
/// shapes our runtime needs.  See
/// ``docs/design/runtime/namespace-tree.md`` §3 for the full field
/// mapping and §4 for the deferred trace / hash-bookkeeping bits.
///
/// Tagged by ``flags``:
///
/// * default (no ``VAR_ARRAY`` / ``VAR_LINK``): scalar — ``value`` is
///   a TclObj handle.  Uninitialised vars start with ``value == 0``.
/// * ``VAR_ARRAY``: ``value`` is the address of an
///   ``ArrayVarTable`` (``hash_table.Table(16)`` keyed by element
///   name, value = ``*Var``).  P3+ wires this; not used in P3.1.
/// * ``VAR_LINK``: ``value`` is the absolute address of another
///   ``Var`` to redirect through.  Created by ``upvar`` / ``global``
///   / ``variable`` (P3.3).  Always followed via
///   ``var_resolve_link`` to reach the terminal storage.
pub const Var = extern struct {
    flags: u32,
    value: u32,
};

const VAR_SIZE: u32 = @sizeOf(Var);

pub const VAR_ARRAY: u32 = 0x1;
pub const VAR_LINK: u32 = 0x2;
pub const VAR_IN_HASHTABLE: u32 = 0x4;
pub const VAR_NAMESPACE_VAR: u32 = 0x80;
pub const VAR_ARRAY_ELEMENT: u32 = 0x1000;
pub const VAR_CONSTANT: u32 = 0x10000;

/// Find a variable in ``ns.var_table`` by simple name.  Returns the
/// stored ``*Var`` (as a u32 handle) or 0 if absent.  Does not
/// follow ``VAR_LINK`` — callers that want the terminal storage
/// chain through ``var_resolve_link``.
pub fn ns_var_find(ns_addr: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (ns.var_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.var_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

/// Find-or-create.  When the entry is missing, allocates a fresh
/// scalar ``Var`` (flags = ``VAR_IN_HASHTABLE | VAR_NAMESPACE_VAR``,
/// value = 0) and inserts it under ``name``.  Returns the ``*Var``
/// handle either way.  Bump-allocator backed; callers never free.
pub fn ns_var_create(ns_addr: u32, name_ptr: u32, name_len: u32) u32 {
    const existing = ns_var_find(ns_addr, name_ptr, name_len);
    if (existing != 0) return existing;
    const ns: *Namespace = @ptrFromInt(ns_addr);
    ns.var_table.init(NS_INITIAL_CAP);
    if (ns.var_table.needs_grow()) ns.var_table.grow();
    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = ns.var_table.insert_header(name_ptr, name_len, hash);

    const var_addr = alloc(VAR_SIZE);
    const v: *Var = @ptrFromInt(var_addr);
    v.flags = VAR_IN_HASHTABLE | VAR_NAMESPACE_VAR;
    v.value = 0;
    write_i32(bucket + OFF_HANDLE, @bitCast(var_addr));
    return var_addr;
}

/// Follow a chain of ``VAR_LINK`` redirects to reach the terminal
/// ``*Var`` whose ``value`` slot actually holds the storage.  Loops
/// are pathological (the only way to create one is through Tcl-level
/// ``upvar`` cycles, which Tcl's variable layer also doesn't really
/// guard against beyond depth-limit traps); we cap at 64 hops which
/// is well above any sane variable-aliasing depth.
pub fn var_resolve_link(v_addr: u32) u32 {
    var cur = v_addr;
    var hops: u32 = 0;
    while (hops < 64) : (hops += 1) {
        if (cur == 0) return 0;
        const v: *const Var = @ptrFromInt(cur);
        if ((v.flags & VAR_LINK) == 0) return cur;
        cur = v.value;
    }
    return cur;
}

/// Read the scalar value (TclObj handle) from a Var, following
/// ``VAR_LINK`` redirects.  Returns 0 for an uninitialised /
/// missing Var or one that's currently shaped as an array (P3+
/// callers handle arrays separately).
pub fn var_get_scalar(v_addr: u32) u32 {
    const t = var_resolve_link(v_addr);
    if (t == 0) return 0;
    const v: *const Var = @ptrFromInt(t);
    if ((v.flags & VAR_ARRAY) != 0) return 0;
    return v.value;
}

/// Write a scalar value (TclObj handle) into a Var, following
/// ``VAR_LINK`` redirects.  Caller must already own the TclObj
/// reference; we never retain.
pub fn var_set_scalar(v_addr: u32, obj_handle: u32) void {
    const t = var_resolve_link(v_addr);
    if (t == 0) return;
    const v: *Var = @ptrFromInt(t);
    // Clear the array bit if it was somehow set — this isn't a
    // C-level "type promote", it's "treat this as a scalar slot
    // now".  Real callers won't flip a real array into a scalar;
    // this is just for the simple-globals replacement in P3.2.
    v.flags &= ~VAR_ARRAY;
    // MM-B.2 refcount discipline: the var slot holds a reference to
    // the value, so retain the incoming obj and release whatever was
    // there before.  Without this the slot's hold is "free" (no
    // refcount bump) and the parser-side release at end-of-statement
    // (MM-B.4) frees the value out from under us.  With both in
    // place, every TclObj eventually drops to refcount 0 when no
    // var, frame, or list/dict element holds it any more.
    const old: u32 = @bitCast(v.value);
    v.value = obj_handle;
    if (obj_handle != 0) obj.tcl_obj_retain(@bitCast(obj_handle));
    if (old != 0 and old != obj_handle) obj.tcl_obj_release(@bitCast(old));
}

// -- Namespace export patterns (P4.1) --------------------------------------
//
// ``namespace export pat1 pat2 …`` records glob patterns on the
// containing namespace.  ``namespace import ::src::pat`` (P4.2)
// walks the source ns's ``cmd_table`` and matches each command's
// simple name against these patterns to decide which redirects to
// create in the importing ns.
//
// Storage: ``Namespace.export_patterns`` is the address of a u32
// array of ``(pattern_ptr, pattern_len)`` pairs (8 bytes per
// pattern).  We grow by re-allocating + copying — append-only with
// no free, the bump allocator leaks the old buffer but namespace
// export lists are tiny (~1-10 patterns) so the waste is bounded
// and the code stays trivial.

const tcl_string = @import("../valtypes/tcl_string.zig");

/// Append ``(pattern_ptr, pattern_len)`` to ``ns.export_patterns``.
/// Bytes are heap-copied so the source slab can be released.  Empty
/// patterns are skipped — they'd match every name and aren't
/// emitted by real Tcl programs.
pub fn ns_export(ns_addr: u32, pattern_ptr: u32, pattern_len: u32) void {
    if (pattern_len == 0) return;
    const ns: *Namespace = @ptrFromInt(ns_addr);
    const old_count = ns.export_pattern_count;
    const new_count = old_count + 1;

    // Allocate the new array (8 bytes per entry) plus a heap copy
    // of the pattern bytes.  We never reuse the old array — the
    // bump allocator can't free, but namespace export lists are
    // tiny so the leakage is bounded.
    const new_buf = alloc(new_count * 8);
    if (old_count > 0) {
        memcpy(new_buf, ns.export_patterns, old_count * 8);
    }
    const pat_copy = alloc(pattern_len);
    memcpy(pat_copy, pattern_ptr, pattern_len);
    write_i32(new_buf + old_count * 8, @bitCast(pat_copy));
    write_i32(new_buf + old_count * 8 + 4, @bitCast(pattern_len));

    ns.export_patterns = new_buf;
    ns.export_pattern_count = new_count;
}

/// Test whether ``name`` matches any of ``ns``'s registered export
/// patterns.  Uses ``string match`` (Tcl glob) semantics via
/// ``tcl_string.glob_match``.  Returns false when no patterns are
/// registered — matches Tcl's "nothing exported by default" rule.
pub fn ns_export_matches(ns_addr: u32, name_ptr: u32, name_len: u32) bool {
    const ns: *const Namespace = @ptrFromInt(ns_addr);
    if (ns.export_pattern_count == 0) return false;
    var i: u32 = 0;
    while (i < ns.export_pattern_count) : (i += 1) {
        const off = ns.export_patterns + i * 8;
        const pp: u32 = @bitCast(read_i32(off));
        const pl: u32 = @bitCast(read_i32(off + 4));
        if (tcl_string.glob_match(pp, pl, name_ptr, name_len)) return true;
    }
    return false;
}

// -- Namespace path (P5.1) --------------------------------------------------

/// One ordered entry on a namespace's command resolution path.
/// ``target_ns`` is the namespace to probe; ``creator_ns`` is the
/// namespace whose ``path_array[i]`` this entry occupies.  The
/// ``prev`` / ``next`` slots form a doubly-linked list that hangs
/// off the *target's* ``path_source_head`` (P5.3) so we can
/// invalidate dependents when the target's commands change.
///
/// 16 bytes; bump-allocated en bloc as a contiguous array on the
/// owning namespace.
pub const NamespacePathEntry = extern struct {
    target_ns: u32,
    creator_ns: u32,
    prev: u32,
    next: u32,
};

const PATH_ENTRY_SIZE: u32 = @sizeOf(NamespacePathEntry);

/// Replace ``ns``'s command resolution path with the given list of
/// target namespaces.  ``targets_buf`` points at a packed array of
/// ``targets_count`` u32 ``*Namespace`` handles in linear memory
/// (any zero entries are skipped — left over from a parse miss).
///
/// Taking the array as a raw u32 byte-address (rather than a typed
/// ``[*]const u32``) avoids the alignment cast Zig would otherwise
/// require — our bump allocator returns 4-byte aligned addresses
/// in practice but the type system doesn't know that.
///
/// We re-allocate the path array on every call rather than growing
/// — paths are typically set once at module load and rarely
/// touched, so the leakage is bounded.  P5.3 also splices each new
/// entry into the *target* ns's ``path_source_head`` doubly-linked
/// list so future cmd-table mutations can walk back to the
/// dependents and bump their ``cmd_ref_epoch``.
pub fn ns_set_path(ns_addr: u32, targets_buf: u32, targets_count: u32) void {
    const ns: *Namespace = @ptrFromInt(ns_addr);

    // Unhook any previously-linked path entries from their
    // targets' source lists.  Without this, a subsequent
    // ``namespace path``-triggered ``cmd_ref_epoch`` cascade
    // would still walk the dead entries and bump invalid
    // dependents.
    if (ns.path_array != 0 and ns.path_len > 0) {
        var k: u32 = 0;
        while (k < ns.path_len) : (k += 1) {
            const e: *NamespacePathEntry = @ptrFromInt(ns.path_array + k * PATH_ENTRY_SIZE);
            unlink_path_source(e);
        }
    }

    var nonzero_count: u32 = 0;
    var i: u32 = 0;
    while (i < targets_count) : (i += 1) {
        const t: u32 = @bitCast(read_i32(targets_buf + i * 4));
        if (t != 0) nonzero_count += 1;
    }
    if (nonzero_count == 0) {
        ns.path_array = 0;
        ns.path_len = 0;
        return;
    }
    const buf = alloc(nonzero_count * PATH_ENTRY_SIZE);
    var slot: u32 = 0;
    var j: u32 = 0;
    while (j < targets_count) : (j += 1) {
        const t: u32 = @bitCast(read_i32(targets_buf + j * 4));
        if (t == 0) continue;
        const entry_addr = buf + slot * PATH_ENTRY_SIZE;
        const e: *NamespacePathEntry = @ptrFromInt(entry_addr);
        e.target_ns = t;
        e.creator_ns = ns_addr;
        e.prev = 0;
        e.next = 0;
        // Splice into target.path_source_head (prepend at head;
        // doubly-linked so unlinking on path replace is O(1)).
        const target: *Namespace = @ptrFromInt(t);
        const old_head = target.path_source_head;
        e.next = old_head;
        if (old_head != 0) {
            const oh: *NamespacePathEntry = @ptrFromInt(old_head);
            oh.prev = entry_addr;
        }
        target.path_source_head = entry_addr;
        slot += 1;
    }
    ns.path_array = buf;
    ns.path_len = nonzero_count;
}

/// Splice a path entry out of its target's ``path_source_head``
/// doubly-linked list.  Idempotent — tolerates entries that were
/// never linked (e.g. zero target).
fn unlink_path_source(e: *NamespacePathEntry) void {
    if (e.target_ns == 0) return;
    const target: *Namespace = @ptrFromInt(e.target_ns);
    if (e.prev != 0) {
        const p: *NamespacePathEntry = @ptrFromInt(e.prev);
        p.next = e.next;
    } else {
        // Head of the list.
        target.path_source_head = e.next;
    }
    if (e.next != 0) {
        const n: *NamespacePathEntry = @ptrFromInt(e.next);
        n.prev = e.prev;
    }
    e.prev = 0;
    e.next = 0;
}

/// Bump ``cmd_ref_epoch`` on ``ns`` and cascade through its
/// ``path_source_head`` list, bumping each dependent ns's epoch
/// too.  Called from ``ns_cmd_put`` so any cmd_table mutation
/// invalidates downstream cache layers.  In our runtime the only
/// "cache" is the proc-lookup LRU (which gets blown away by the
/// caller of ns_cmd_put — proc_register, ns_import — already), so
/// the epoch field is currently a record-keeper for future cache
/// machinery rather than the invalidation trigger.
pub fn bump_cmd_ref_epoch(ns_addr: u32) void {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    ns.cmd_ref_epoch +%= 1;
    var cur = ns.path_source_head;
    while (cur != 0) {
        const e: *const NamespacePathEntry = @ptrFromInt(cur);
        if (e.creator_ns != 0 and e.creator_ns != ns_addr) {
            const dep: *Namespace = @ptrFromInt(e.creator_ns);
            dep.cmd_ref_epoch +%= 1;
        }
        cur = e.next;
    }
}

/// Read-only view of an ns's path entries.  Returns the count;
/// caller indexes into ``ns.path_array`` directly.  Used by P5.2's
/// ``ns_find_command`` extension.
pub fn ns_path_len(ns_addr: u32) u32 {
    const ns: *const Namespace = @ptrFromInt(ns_addr);
    return ns.path_len;
}

pub fn ns_path_entry(ns_addr: u32, idx: u32) *const NamespacePathEntry {
    const ns: *const Namespace = @ptrFromInt(ns_addr);
    const buf = ns.path_array + idx * PATH_ENTRY_SIZE;
    return @ptrFromInt(buf);
}

// -- Namespace import (P4.2) ------------------------------------------------

/// ``ImportedCmdData`` — the ``client_data`` payload C Tcl hangs
/// off the ``params_obj`` slot of a ``CMD_IMPORTED`` Command.
/// ``real_cmd`` is the source ``*Command`` (lookup follows it),
/// ``self_cmd`` points back at this redirect Command itself so
/// P4.3's import-back-pointer list can splice it on source delete.
///
/// 8 bytes; bump-allocated per import.
pub const ImportedCmdData = extern struct {
    real_cmd: u32,
    self_cmd: u32,
};

/// Mirror of the ``Command`` layout constants in ``tcl_procs.zig``.
/// Duplicated here because ``tcl_procs.zig`` imports this module, so
/// importing it back would be circular.  ``tcl_procs.zig`` has a
/// ``comptime`` block that asserts these values stay in sync with its
/// own canonical ``pub const``s, so any drift becomes a compile error.
pub const tcl_procs_constants = struct {
    pub const COMMAND_SIZE: u32 = 44;
    pub const OFF_FLAGS: u32 = 8;
    pub const OFF_PARAMS_OBJ: u32 = 12;
    pub const OFF_IMPORT_REF_HEAD: u32 = 32;
    pub const CMD_IMPORTED: u32 = 0x80;
};

/// Singly-linked list node used to track every redirect Command
/// that imports a given source Command.  ``namespace forget``
/// (P4.4) walks this list to splice out the redirects.  Allocated
/// on the bump allocator per import — never freed individually.
pub const ImportRef = extern struct {
    imported_cmd: u32,
    next: u32,
};

/// Allocate a redirect ``Command`` shaped like a regular Command
/// but with ``CMD_IMPORTED`` set and ``params_obj`` pointing at a
/// freshly-allocated ``ImportedCmdData`` whose ``real_cmd`` field
/// is the source.  ``name_*`` is the simple name the redirect
/// will be inserted under in the importing ns's ``cmd_table``.
fn alloc_import_redirect(name_ptr: u32, name_len: u32, source_cmd: u32) u32 {
    const c = tcl_procs_constants;
    const cmd = alloc(c.COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(cmd);
    @memset(slice[0..c.COMMAND_SIZE], 0);
    const nbuf = alloc(name_len);
    if (name_len > 0) memcpy(nbuf, name_ptr, name_len);
    write_i32(cmd, @bitCast(nbuf));
    write_i32(cmd + 4, @bitCast(name_len));
    write_i32(cmd + c.OFF_FLAGS, @bitCast(c.CMD_IMPORTED));

    const desc = alloc(@sizeOf(ImportedCmdData));
    const d: *ImportedCmdData = @ptrFromInt(desc);
    d.real_cmd = source_cmd;
    d.self_cmd = cmd;
    write_i32(cmd + c.OFF_PARAMS_OBJ, @bitCast(desc));
    return cmd;
}

/// ``namespace import ::src::pat`` semantics.  Walks the source
/// namespace's ``cmd_table``, matches each command's simple name
/// against (a) the trailing pattern from the import spec and
/// (b) the source ns's registered exports (P4.1), and inserts a
/// redirect Command into ``dest_ns.cmd_table`` for every match.
///
/// ``import_spec`` is the user-facing argument: anything from
/// ``foo`` to ``::tcltest::*``.  We split off the trailing
/// component as the simple match pattern; everything before is the
/// source namespace.
///
/// Returns the number of redirects created (0 if the source ns
/// doesn't exist or no exports matched).
pub fn ns_import(dest_ns: u32, import_spec_ptr: u32, import_spec_len: u32) u32 {
    if (import_spec_len == 0) return 0;

    // Resolve the spec to (source_ns, simple_pattern).  Reuse the
    // qualified-name walker — it splits off the trailing simple
    // component for free.
    const r = ns_resolve_qualified(ns_current(), import_spec_ptr, import_spec_len);
    const src_ns = r.target_ns;
    if (src_ns == 0 or r.simple_len == 0) return 0;

    var imported: u32 = 0;

    const ns: *const Namespace = @ptrFromInt(src_ns);
    if (ns.cmd_table.buf == 0) return 0;

    // Iterate the source ns's cmd_table buckets.  Each populated
    // bucket has a non-zero name_ptr in its header; the value
    // (at OFF_HANDLE) is the source ``*Command``.
    const bucket_size: u32 = 16; // matches NS_BUCKET_SIZE
    var i: u32 = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * bucket_size;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        // Filter 1: must match the import-spec trailing pattern.
        if (!tcl_string.glob_match(r.simple_ptr, r.simple_len, name_ptr, name_len)) continue;
        // Filter 2: must match the source ns's export list (the
        // "you can't import what wasn't exported" rule).
        if (!ns_export_matches(src_ns, name_ptr, name_len)) continue;

        const source_cmd: u32 = @bitCast(read_i32(bucket + OFF_HANDLE));
        if (source_cmd == 0) continue;
        // P4.3: chained imports point at the *terminal* source so
        // the back-pointer list always lives on the real command.
        // Mirrors C Tcl's ``DoImport`` walking
        // ``ImportedCmdData.realCmdPtr`` to its root.
        const real_source = unwrap_imports_chain(source_cmd);
        const redirect = alloc_import_redirect(name_ptr, name_len, real_source);
        _ = ns_cmd_put(dest_ns, name_ptr, name_len, redirect);
        link_import_ref(real_source, redirect);
        imported += 1;
    }

    return imported;
}

/// Follow ``CMD_IMPORTED`` links to the terminal Command — local
/// shadow of ``tcl_procs.unwrap_imports`` to avoid the circular
/// dependency.  Capped the same way (64 hops).
fn unwrap_imports_chain(cmd_in: u32) u32 {
    const c = tcl_procs_constants;
    var cur: u32 = cmd_in;
    var hops: u32 = 0;
    while (hops < 64) : (hops += 1) {
        if (cur == 0) return 0;
        const flags: u32 = @bitCast(read_i32(cur + c.OFF_FLAGS));
        if ((flags & c.CMD_IMPORTED) == 0) return cur;
        const desc: u32 = @bitCast(read_i32(cur + c.OFF_PARAMS_OBJ));
        if (desc == 0) return cur;
        const real: u32 = @bitCast(read_i32(desc));
        if (real == 0) return cur;
        cur = real;
    }
    return cur;
}

/// Prepend an ``ImportRef { imported_cmd: redirect, next: prev_head }``
/// onto ``source_cmd.import_ref_head``.  Singly-linked, no
/// duplicate detection (the same dest ns can't import the same
/// source command twice without first ``namespace forget``-ting
/// it, so duplicates would only arise from program bugs).
pub fn link_import_ref(source_cmd: u32, redirect: u32) void {
    const c = tcl_procs_constants;
    const prev_head: u32 = @bitCast(read_i32(source_cmd + c.OFF_IMPORT_REF_HEAD));
    const node = alloc(@sizeOf(ImportRef));
    const r: *ImportRef = @ptrFromInt(node);
    r.imported_cmd = redirect;
    r.next = prev_head;
    write_i32(source_cmd + c.OFF_IMPORT_REF_HEAD, @bitCast(node));
}

/// Splice the ``ImportRef`` for *redirect* out of ``source_cmd``'s
/// back-list.  Walks the singly-linked list once; quietly succeeds
/// if no matching node is present (defensive against forget-twice).
pub fn unlink_import_ref(source_cmd: u32, redirect: u32) void {
    const c = tcl_procs_constants;
    var prev_link_addr: u32 = source_cmd + c.OFF_IMPORT_REF_HEAD;
    var cur: u32 = @bitCast(read_i32(prev_link_addr));
    while (cur != 0) {
        const node: *ImportRef = @ptrFromInt(cur);
        if (node.imported_cmd == redirect) {
            // Patch ``prev.next`` (or the head, depending on
            // where we are) to skip this node.
            write_i32(prev_link_addr, @bitCast(node.next));
            return;
        }
        // Advance: ``prev_link_addr`` becomes the address of the
        // current node's ``next`` field, which is offset 4.
        prev_link_addr = cur + 4;
        cur = node.next;
    }
}

/// ``namespace forget pat1 pat2 …`` semantics.  For each redirect
/// in ``ns.cmd_table`` whose simple name matches one of the
/// patterns, deactivate it: clear the ``ImportedCmdData.real_cmd``
/// pointer (so ``unwrap_imports`` returns 0 → "not found") and
/// splice the redirect's ``ImportRef`` out of the source's
/// back-list.
///
/// Returns the count of redirects forgotten.  The cmd_table
/// buckets stay populated — our open-addressed table doesn't
/// support tombstones — but the now-dead redirects no longer
/// resolve to anything callable and are invisible to subsequent
/// imports / forgets that might re-overwrite them.
///
/// **Caveat for future cmd_table iterators** (e.g. ``info
/// commands`` when it's wired to the ns tree): a forgotten
/// redirect leaves a bucket whose ``OFF_HANDLE`` still points at
/// the redirect ``Command``, whose ``OFF_FLAGS`` still has
/// ``CMD_IMPORTED`` set, but whose ``ImportedCmdData.real_cmd``
/// is ``0``.  Iterators must skip entries where
/// ``(flags & CMD_IMPORTED) != 0`` and
/// ``ImportedCmdData.real_cmd == 0`` to avoid returning ghost
/// names.  The ``unwrap_imports`` helper already does this and
/// returns ``0`` for such entries.
pub fn ns_forget(ns_addr: u32, pattern_ptr: u32, pattern_len: u32) u32 {
    if (pattern_len == 0) return 0;
    const ns: *const Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return 0;

    const c = tcl_procs_constants;
    const bucket_size: u32 = 16;
    var forgotten: u32 = 0;
    var i: u32 = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * bucket_size;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        if (!tcl_string.glob_match(pattern_ptr, pattern_len, name_ptr, name_len)) continue;

        const redirect: u32 = @bitCast(read_i32(bucket + OFF_HANDLE));
        if (redirect == 0) continue;
        const flags: u32 = @bitCast(read_i32(redirect + c.OFF_FLAGS));
        if ((flags & c.CMD_IMPORTED) == 0) continue; // not an import; leave alone

        const desc: u32 = @bitCast(read_i32(redirect + c.OFF_PARAMS_OBJ));
        if (desc == 0) continue;
        const d: *ImportedCmdData = @ptrFromInt(desc);
        const source_cmd = d.real_cmd;
        // Unlink the back-pointer first so future imports of the
        // same source under a different name don't see a stale
        // entry pointing at the dead redirect.
        if (source_cmd != 0) unlink_import_ref(source_cmd, redirect);
        // Mark the redirect dead — ``unwrap_imports`` returns 0
        // for ``real_cmd == 0``, so ``proc_lookup`` will start
        // returning 0 on this name.
        d.real_cmd = 0;
        forgotten += 1;
    }
    return forgotten;
}

// -- Public globals ABI (moved from the retired tcl_globals.zig in P3.4)
//
// These four exports are the long-standing names compiled WASM
// modules import for global-variable access.  They live here now
// because the storage they front IS the root namespace's
// ``var_table`` — there's no separate flat table left to host them.
//
// Keeping the existing export names means the compiler's import
// table (``codegen/wasm/_imports.py``) doesn't change; only the
// implementation home moves.

const obj_new_int_pub = obj.obj_new_int;
const obj_get_int_pub = obj.obj_get_int;

/// Set a global variable.  Lazy-creates the ``*Var`` if missing.
/// Strip a single leading ``::`` from a name span so the root
/// namespace's ``var_table`` is keyed on the simple name.  Tcl's
/// global-namespace ``::x`` is the variable ``x`` *in* root, not a
/// distinct entry whose key happens to include the prefix; without
/// this the same physical variable would be stored under ``x`` (by
/// the codegen's ``set x …`` lowering) and looked up under ``::x``
/// (by ``$::x`` / ``info exists ::x``), missing every time.
fn strip_global_prefix(ptr: u32, len: u32) struct { ptr: u32, len: u32 } {
    if (len >= 2) {
        const p: [*]const u8 = @ptrFromInt(ptr);
        if (p[0] == ':' and p[1] == ':') {
            return .{ .ptr = ptr + 2, .len = len - 2 };
        }
    }
    return .{ .ptr = ptr, .len = len };
}

/// Re-entry guard for the scalar/array conflict check below.  Set
/// while ``global_set`` is calling ``stubs.raise`` so the error-
/// stamping path's recursive ``global_set("::errorInfo", …)`` calls
/// don't re-trigger the conflict probe (and stack-overflow if the
/// probe itself happens to find a match in the array directory).
var conflict_check_active: bool = false;

pub export fn global_set(name: i32, value: i32) i32 {
    const sn = obj.obj_ensure_string(name);
    const k = strip_global_prefix(sn.ptr, sn.len);
    // Scalar/array name-conflict detection.  Real Tcl raises
    // ``can't set "<name>": variable is array`` if the user tries to
    // store a scalar under a name that's currently shaped as an
    // array.  Use ``array_exists_raw`` (no TclObj allocation) keyed
    // by the *stripped* name so ``set ::a 1`` and ``set a 1`` both
    // see an existing array ``a`` — the array directory keys by the
    // post-normalisation form, which equals the var subsystem's
    // stripped form for top-level / global writes.  The previous
    // ``array_exists(name)`` allocated a TclObj on every set and
    // missed the conflict on ``set ::a`` vs an existing ``a(...)``
    // (Copilot review, PR #237).  ``conflict_check_active``
    // suppresses the probe inside the ``stubs.raise → tcl_cmd_error
    // → stamp_error_globals`` chain, which recursively writes
    // ``::errorInfo`` / ``::errorCode`` and would otherwise infinite-
    // loop here.
    if (!conflict_check_active and tcl_array.array_exists_raw(k.ptr, k.len)) {
        conflict_check_active = true;
        defer conflict_check_active = false;
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("can't set: variable is array");
        return 0;
    }
    const v = ns_var_create(ns_root(), k.ptr, k.len);
    var_set_scalar(v, @bitCast(value));
    // Notify the scheduler so an active ``vwait`` waiting on this
    // variable wakes up.  Cheap fast-path inside the hook when no
    // vwait is active.
    const sched = @import("../sched/tcl_sched.zig");
    sched.note_var_write(k.ptr, k.len);
    return value;
}

/// Probe used by ``tcl_array.find_or_create`` to detect a
/// scalar-shaped variable already living under the requested array
/// name.  Returns 1 iff a scalar exists with a non-null value, 0
/// otherwise.  Strips the leading ``::`` if present (the array side
/// passes the *normalised* name; the var side does not strip on
/// lookup, so we mirror what ``global_get`` does).
/// Raw accessor used by ``tcl_array.find_table`` to retry an
/// unqualified array lookup as ``<current_ns_full>::<name>``.
/// Returns the bump-allocator pointer of the cached name; the
/// matching length comes from :func:`current_ns_full_len`.
/// Returns 0/0 when the current namespace is root or unset.
pub export fn current_ns_full_ptr() u32 {
    if (current_ns == 0) return 0;
    return ns_full_name(current_ns).ptr;
}

pub export fn current_ns_full_len() u32 {
    if (current_ns == 0) return 0;
    return ns_full_name(current_ns).len;
}

pub export fn ns_scalar_exists(name_ptr: u32, name_len: u32) i32 {
    const k = strip_global_prefix(name_ptr, name_len);
    const v = ns_var_find(ns_root(), k.ptr, k.len);
    if (v == 0) return 0;
    if ((@as(*const Var, @ptrFromInt(v)).flags & VAR_ARRAY) != 0) return 0;
    const val = var_get_scalar(v);
    return if (val != 0) 1 else 0;
}

/// Get a global variable.  Returns 0 (a NULL TclObj handle) if the
/// var has never been set.
pub export fn global_get(name: i32) i32 {
    const sn = obj.obj_ensure_string(name);
    const k = strip_global_prefix(sn.ptr, sn.len);
    const v = ns_var_find(ns_root(), k.ptr, k.len);
    if (v == 0) return 0;
    return @bitCast(var_get_scalar(v));
}

/// Strict variant of :func:`global_get` for codegen-emitted reads of
/// ``::``-qualified or namespace-eval-qualified globals.  Raises
/// ``can't read "<name>": no such variable`` through
/// :func:`tcl_catch.var_unset_error` when the variable has never been
/// set, matching the Python VM and reference Tcl.  The lenient
/// :func:`global_get` is still used for paths that legitimately want
/// the missing-variable-is-fine behaviour (``info exists``, ``unset
/// -nocomplain``, frame readback after eval-fallback, the ``global``
/// command's pre-load of a possibly-uninitialised slot).
pub export fn global_get_or_error(name: i32) i32 {
    const v = global_get(name);
    if (v == 0) {
        const tcl_catch = @import("tcl_catch.zig");
        tcl_catch.var_unset_error(name);
    }
    return v;
}

/// ``info exists ::name`` — returns a TclObj 1 if the entry has
/// been written to root's var_table with a non-null value, OR if
/// an array with this name exists in the array directory.  A scalar
/// value of 0 (null TclObj) means the variable has been unset.
/// Arrays don't have scalar var entries, so the array directory must
/// be checked separately to support ``info exists arrName``.
pub export fn global_exists(name: i32) i32 {
    const sn = obj.obj_ensure_string(name);
    const k = strip_global_prefix(sn.ptr, sn.len);
    const v = ns_var_find(ns_root(), k.ptr, k.len);
    if (v != 0) {
        const val = var_get_scalar(v);
        if (val != 0) return obj_new_int_pub(1);
    }
    // Also check the array directory — arrays are stored there and
    // have no scalar entry in the var_table.
    return tcl_array.array_exists(name);
}

/// Numeric ``incr`` helper — historically lived in tcl_globals.zig
/// alongside the var ABI but doesn't actually touch globals
/// storage.  Kept here for ABI continuity (the compiler still
/// imports it under ``tcl_incr``); not a great architectural fit
/// long-term but moving it is out of scope for the namespace-tree
/// rework.
///
/// ``incr`` is a strict-integer command — Tcl rejects any value
/// whose string form spells a float (decimal point or exponent) or
/// is a boolean keyword with ``expected integer but got "X"``.  See
/// issue #262 (and the regression battery in
/// ``fuzzing/tests/test_fuzz_findings.py::TestIncrStrictParsing``).
/// The previous implementation called ``obj_get_int`` directly,
/// which silently truncates ``"52.60"`` to ``52`` and lets the
/// counter advance — surfacing as either a wasm/vm divergence (the
/// fuzzer's ``wasm-accepts-float-as-integer`` category) or a
/// runaway loop when the float is the loop counter.
pub export fn tcl_incr(o: i32, amount: i32) i32 {
    // Bail early if a prior helper in the same statement (e.g. a
    // missing-variable read on the increment expression) already
    // errored — don't clobber that diagnostic with an
    // ``expected integer`` follow-on.  Match reference Tcl's
    // "first error wins" semantics for a single command.
    if (@import("tcl_catch.zig").error_flag != 0) return obj_new_int_pub(0);
    if (!incr_is_strict_int(o)) {
        raise_expected_integer(o);
        return obj_new_int_pub(0);
    }
    if (!incr_is_strict_int(amount)) {
        raise_expected_integer(amount);
        return obj_new_int_pub(0);
    }
    // Bignum-aware addition: promote to ``*BigInt`` whenever either
    // operand is bignum-shaped so a wide variable counter (or a wide
    // increment delta — used by ``incr x [expr {1<<63}]``) keeps
    // full precision.  The pre-bignum path silently truncated to i64.
    const bignum = @import("../valtypes/tcl_bignum.zig");
    if (obj.obj_type(o) == obj.TYPE_BIGNUM or obj.obj_type(amount) == obj.TYPE_BIGNUM) {
        const ap = obj.obj_promote_to_bignum(o);
        defer if (ap.owned) bignum.destroy(ap.m);
        const bp = obj.obj_promote_to_bignum(amount);
        defer if (bp.owned) bignum.destroy(bp.m);
        if (ap.m == null or bp.m == null) return obj_new_int_pub(0);
        const r = bignum.alloc_add(ap.m.?, bp.m.?) orelse return obj_new_int_pub(0);
        return obj.obj_new_bignum_take(r);
    }
    const val = obj_get_int_pub(o);
    const amt = obj_get_int_pub(amount);
    const r = @addWithOverflow(val, amt);
    if (r[1] == 0) return obj_new_int_pub(r[0]);
    // i64 overflow → promote to bignum.
    return obj.obj_new_bignum(@as(i128, val) + @as(i128, amt));
}

fn incr_is_strict_int(o: i32) bool {
    if (o == 0) return true;
    const tag = obj.obj_type(o);
    if (tag == obj.TYPE_INT) return true;
    if (tag == obj.TYPE_BIGNUM) return true;
    if (tag == obj.TYPE_FLOAT) return false;
    // String / inline string — delegate to the canonical strict-decimal
    // parser used by ``obj_get_int``.  Keeping the validation and the
    // read in lock-step means anything we accept here will round-trip
    // cleanly to an int when ``tcl_incr`` calls ``obj_get_int`` next.
    //
    //   * empty / whitespace-only   → null → reject
    //   * floats (``"52.60"``, ``"1e5"``)        → null → reject (issue #262)
    //   * boolean keywords (``"yes"``, ``"true"``) → null → reject
    //   * alpha-only (``"abc"``, ``"deadbeef"``)   → null → reject
    //
    // The earlier hand-rolled char-class whitelist accepted any string
    // containing only ``[0-9a-fA-FxXoOb B]`` (so ``"abc"`` and
    // ``"deadbeef"`` slipped through and ``obj_get_int`` silently
    // returned 0 — Copilot review on PR #287).  Empty strings also
    // returned ``true`` and let ``incr ""`` succeed instead of raising
    // ``expected integer but got ""``.
    //
    // Note: ``try_parse_int`` is currently decimal-only.  Hex / octal /
    // binary integer literals (``"0xff"``, ``"0o17"``, ``"0b1010"``)
    // therefore round-trip through this validator as ``expected
    // integer …``.  That's tighter than reference Tcl, but
    // ``obj_get_int`` can't extract those bases either, so accepting
    // them here would silently miscompute (``incr i`` with
    // ``i = "0xFF"`` would yield ``1``, not ``256``).  Extending
    // ``try_parse_int`` to cover non-decimal bases is tracked
    // separately and out of scope for the bitwise/shift/incr domain
    // fixes covered by issues #260–#262.
    const s = obj.obj_ensure_string(o);
    if (obj.try_parse_int(s.ptr, s.len) != null) return true;
    // Accept bignum-shaped string literals so ``incr x 9223372036854775808``
    // doesn't reject the wide delta as ``expected integer``.  Match the
    // i128 / Managed parse discipline the arithmetic helpers use.
    const bignum = @import("../valtypes/tcl_bignum.zig");
    if (bignum.parse_i128(s.ptr, s.len) != null) return true;
    const m = bignum.alloc_from_string(s.ptr, s.len) orelse return false;
    bignum.destroy(m);
    return true;
}

fn raise_expected_integer(o: i32) void {
    // Preserve the first error in a chain — see ``tcl_incr`` for
    // the rationale.  Without this, a missing-variable read on the
    // increment expression that already set ``error_flag`` would be
    // overwritten by the follow-on ``expected integer`` diagnostic.
    if (@import("tcl_catch.zig").error_flag != 0) return;
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "expected integer but got \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            buf[off] = sp[i];
            off += 1;
        }
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    const msg = obj.obj_new_string(@bitCast(buf_addr), @bitCast(total));
    @import("tcl_catch.zig").tcl_cmd_error(msg);
}

// -- Test scaffolding -------------------------------------------------------
//
// The QualifiedResult struct is awkward to surface across a WASM
// FFI (multi-value returns aren't universally supported by hosts),
// so the test path stashes the result into module-level globals
// and exposes per-field accessors.  Production callers use the
// Zig-native ``ns_resolve_qualified`` directly.

var last_target_ns: u32 = 0;
var last_simple_ptr: u32 = 0;
var last_simple_len: u32 = 0;
var last_alt_ns: u32 = 0;

pub export fn tcl_ns_resolve_qualified(cxt: i32, name_ptr: i32, name_len: i32) i32 {
    const r = ns_resolve_qualified(@bitCast(cxt), @bitCast(name_ptr), @bitCast(name_len));
    last_target_ns = r.target_ns;
    last_simple_ptr = r.simple_ptr;
    last_simple_len = r.simple_len;
    last_alt_ns = r.alt_ns;
    return @bitCast(r.target_ns);
}

pub export fn tcl_ns_last_simple_ptr() i32 {
    return @bitCast(last_simple_ptr);
}

pub export fn tcl_ns_last_simple_len() i32 {
    return @bitCast(last_simple_len);
}

pub export fn tcl_ns_last_alt() i32 {
    return @bitCast(last_alt_ns);
}

/// Test-only: allocate ``len`` bytes via the bump allocator and
/// return the address.  Python tests use this to stage name bytes
/// into linear memory before calling resolution helpers.  Not
/// emitted by the compiler — would otherwise be a subtle DOS
/// vector if user code could call it directly.
pub export fn tcl_test_alloc(len: i32) i32 {
    return @bitCast(alloc(@bitCast(len)));
}

// -- WASM-exported wrappers -------------------------------------------------
//
// These give the Python-side runtime a stable ABI (i32 in / i32 out)
// that doesn't depend on Zig's slice calling convention.  All names
// are byte ranges into linear memory — same convention as
// ``tcl_globals.global_set`` etc.

pub export fn tcl_ns_root() i32 {
    return @bitCast(ns_root());
}

pub export fn tcl_ns_lookup(parent: i32, name_ptr: i32, name_len: i32) i32 {
    return @bitCast(ns_lookup(@bitCast(parent), @bitCast(name_ptr), @bitCast(name_len)));
}

pub export fn tcl_ns_create(parent: i32, name_ptr: i32, name_len: i32) i32 {
    return @bitCast(ns_create(@bitCast(parent), @bitCast(name_ptr), @bitCast(name_len)));
}
