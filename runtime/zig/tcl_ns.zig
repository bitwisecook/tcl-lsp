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

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("hash_table.zig");

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

/// Address of the root namespace, allocated lazily on first call to
/// ``ns_root``.  Zero before then.
var root_addr: u32 = 0;

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
        return bucket;
    }
    if (ns.cmd_table.needs_grow()) ns.cmd_table.grow();
    const bucket = ns.cmd_table.insert_header(name_ptr, name_len, hash);
    write_i32(bucket + OFF_HANDLE, @bitCast(value));
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

    // Unqualified: context first, then root.  P5.2 will splice
    // ``commandPathArray`` between these two.
    if (start != 0) {
        const v = ns_cmd_find(start, name_ptr, name_len);
        if (v != 0) return v;
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
    v.value = obj_handle;
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

const tcl_string = @import("tcl_string.zig");

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

const tcl_procs_constants = struct {
    // Mirror of the layout constants in ``tcl_procs.zig``.  We
    // can't ``@import`` that module here without a circular
    // dependency (procs imports ns), so we duplicate the offsets.
    // The values are pinned by the design doc + a static_assert in
    // tcl_procs.zig (a comptime ``if`` would catch a mismatch on
    // build).  Update both together.
    const COMMAND_SIZE: u32 = 32;
    const OFF_FLAGS: u32 = 8;
    const OFF_PARAMS_OBJ: u32 = 12;
    const CMD_IMPORTED: u32 = 0x80;
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
        const redirect = alloc_import_redirect(name_ptr, name_len, source_cmd);
        _ = ns_cmd_put(dest_ns, name_ptr, name_len, redirect);
        imported += 1;
    }

    return imported;
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
pub export fn global_set(name: i32, value: i32) i32 {
    const sn = obj.obj_ensure_string(name);
    const v = ns_var_create(ns_root(), sn.ptr, sn.len);
    var_set_scalar(v, @bitCast(value));
    return value;
}

/// Get a global variable.  Returns 0 (a NULL TclObj handle) if the
/// var has never been set.
pub export fn global_get(name: i32) i32 {
    const sn = obj.obj_ensure_string(name);
    const v = ns_var_find(ns_root(), sn.ptr, sn.len);
    if (v == 0) return 0;
    return @bitCast(var_get_scalar(v));
}

/// ``info exists ::name`` — returns a TclObj 1 if the entry has
/// ever been written to root's var_table, else 0.  Match the prior
/// "hash entry exists" behaviour rather than checking value != 0.
pub export fn global_exists(name: i32) i32 {
    const sn = obj.obj_ensure_string(name);
    const v = ns_var_find(ns_root(), sn.ptr, sn.len);
    if (v == 0) return obj_new_int_pub(0);
    return obj_new_int_pub(1);
}

/// Numeric ``incr`` helper — historically lived in tcl_globals.zig
/// alongside the var ABI but doesn't actually touch globals
/// storage.  Kept here for ABI continuity (the compiler still
/// imports it under ``tcl_incr``); not a great architectural fit
/// long-term but moving it is out of scope for the namespace-tree
/// rework.
pub export fn tcl_incr(o: i32, amount: i32) i32 {
    const val = obj_get_int_pub(o);
    const amt = obj_get_int_pub(amount);
    return obj_new_int_pub(val + amt);
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
