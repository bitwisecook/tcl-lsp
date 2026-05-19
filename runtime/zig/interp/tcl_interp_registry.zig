// Interpreter registry — one ``Interp`` per interpreter, keyed by
// simple name inside its parent.
//
// Ships the minimum viable primitives for child interpreters
// (``interp create`` / ``eval`` / ``delete`` / ``exists`` /
// ``slaves``), plus the per-interp ``hidden_cmd_table`` slot that
// used to live as a module-global in ``tcl_ns.zig``.
//
// Reference C Tcl sources:
//
// * ``tmp/tcl9.0.3/generic/tclBasic.c`` — ``Tcl_CreateInterp`` /
//   ``DeleteInterp`` (per-interp global-ns, hidden-table, call-frame
//   stack).
// * ``tmp/tcl9.0.3/generic/tclInterp.c`` — ``ChildCreate`` /
//   ``ChildEval`` / ``InterpObjCmd`` dispatch.
//
// Storage model (mirrors ``tmp/tcl9.0.3/generic/tclInterp.c``
// ``Parent.childTable`` + ``Child`` struct, trimmed to what the
// WASM runtime consumes):
//
//   Interp (per interp, bump-allocated):
//     root_ns          — this interp's global (::) Namespace*
//     hidden_cmd_table — this interp's hidden commands
//     parent           — parent Interp* (0 for the root interp)
//     name_*           — simple name in parent's children table
//     children         — child-interp registry (name -> Interp*)
//     flags            — INTERP_SAFE bit, etc.
//
// Swap semantics:
//
// ``interp eval child script`` saves ``tcl_ns.root_addr``,
// ``tcl_ns.current_ns``, and ``current_interp``, swaps them to the
// child's slots, dispatches the script, and restores on the way
// out.  The shared ``tcl_ns.root_addr`` avoids a circular import
// cycle (ns ↔ registry); swapping it keeps every ``tcl_ns.ns_root()``
// caller in the rest of the runtime seeing the right root without
// per-call lookups.

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("../valtypes/hash_table.zig");
const tcl_ns = @import("tcl_ns.zig");

/// ``-safe`` marker.  Matches C Tcl's ``INTERP_SAFE`` flag semantics
/// loosely — we record the bit but don't gate file/exec/package/env
/// access because the WASM runtime doesn't expose those anyway.  See
/// ``docs/design/runtime/child-interp.md`` §2 for the deferred-gating
/// discussion.
pub const INTERP_SAFE: u32 = 0x1;

/// Default per-interp recursion limit.  Matches Tcl 9's
/// ``Tcl_CreateInterp`` initial value
/// (``tclBasic.c`` ``Tcl_CreateInterp`` sets ``interp->maxNestingDepth = 1000``).
pub const DEFAULT_RECURSION_LIMIT: u32 = 1000;

/// Predicate: is the interp at ``addr`` flagged ``INTERP_SAFE``?
/// Empty paths and root-interp queries route through here.
pub fn interp_is_safe(addr: u32) bool {
    if (addr == 0) return false;
    const i: *Interp = @ptrFromInt(addr);
    return (i.flags & INTERP_SAFE) != 0;
}

/// Read the per-interp recursion limit, seeding the default on
/// first read so ``interp recursionlimit child`` always returns a
/// non-zero answer without forcing every ``child_create`` site to
/// remember to set it.
pub fn interp_recursion_limit(addr: u32) u32 {
    if (addr == 0) return DEFAULT_RECURSION_LIMIT;
    const i: *Interp = @ptrFromInt(addr);
    if (i.recursion_limit == 0) i.recursion_limit = DEFAULT_RECURSION_LIMIT;
    return i.recursion_limit;
}

/// Update the per-interp recursion limit.  Values < 1 are rejected
/// by the caller (matches Tcl 9, which bounces non-positive limits).
pub fn interp_set_recursion_limit(addr: u32, new_limit: u32) void {
    if (addr == 0) return;
    const i: *Interp = @ptrFromInt(addr);
    i.recursion_limit = new_limit;
}

/// Set / clear the ``INTERP_SAFE`` flag on ``addr``.  Used by
/// ``interp marktrusted`` (clear) — there is no Tcl-level command
/// to set the flag after creation; ``interp create -safe`` is the
/// only path for that direction.
pub fn interp_set_safe(addr: u32, safe: bool) void {
    if (addr == 0) return;
    const i: *Interp = @ptrFromInt(addr);
    if (safe) {
        i.flags |= INTERP_SAFE;
    } else {
        i.flags &= ~INTERP_SAFE;
    }
}

/// Per-interp alias chain selector.  ``.child`` walks
/// :attr:`Interp.aliases_head` (the source-side list of aliases
/// registered in this interp); ``.target`` walks
/// :attr:`Interp.alias_targets_head` (the destination-side list
/// of aliases pointing at commands in this interp).
pub const AliasChain = enum { child, target };

/// Prepend an :type:`AliasRec` onto one of the interp's alias
/// chains.  Mirrors ``Tcl_CreateAlias``'s wiring into
/// ``Interp.child.aliasTable`` (for the source side) and
/// ``Interp.parent.targetsPtr`` (for the target side).
pub fn alias_chain_push(addr: u32, rec_addr: u32, chain: AliasChain) void {
    if (addr == 0 or rec_addr == 0) return;
    const i: *Interp = @ptrFromInt(addr);
    const next_off: u32 = chain_next_offset(chain);
    const head_addr: u32 = addr + chain_head_offset(chain);
    // ``rec_addr`` points at an :type:`AliasRec`; ``next_off`` is
    // the byte offset of the matching ``next_*`` field within that
    // struct.  Mirrors C Tcl's ``aliasPtr->aliasEntryPtr = ...``
    // (the child-side link) and ``targetPtr->nextPtr = ...`` (the
    // parent-side link).
    obj.write_i32(rec_addr + next_off, @bitCast(obj.read_i32(head_addr)));
    obj.write_i32(head_addr, @bitCast(rec_addr));
    _ = i;
}

/// Unlink an :type:`AliasRec` from one of the interp's alias
/// chains.  Idempotent; missing entries are silently ignored
/// (matches C Tcl's ``Tcl_DeleteHashEntry`` semantics — the
/// caller may have already removed the record).
pub fn alias_chain_remove(addr: u32, rec_addr: u32, chain: AliasChain) void {
    if (addr == 0 or rec_addr == 0) return;
    const next_off: u32 = chain_next_offset(chain);
    var cursor: u32 = addr + chain_head_offset(chain);
    while (true) {
        const cur: u32 = @bitCast(obj.read_i32(cursor));
        if (cur == 0) return;
        if (cur == rec_addr) {
            obj.write_i32(cursor, obj.read_i32(cur + next_off));
            obj.write_i32(cur + next_off, 0);
            return;
        }
        cursor = cur + next_off;
    }
}

/// Walk one of the interp's alias chains.  Returns the first
/// :type:`AliasRec` whose ``token`` (original simple name)
/// matches ``name``.  Used by ``interp alias`` query/delete to
/// resolve the alias by its original spelling even after a
/// ``rename`` has moved the source Command around.
pub fn alias_chain_find_token(
    addr: u32,
    chain: AliasChain,
    name_ptr: u32,
    name_len: u32,
) u32 {
    if (addr == 0) return 0;
    const next_off: u32 = chain_next_offset(chain);
    var cur: u32 = @bitCast(obj.read_i32(addr + chain_head_offset(chain)));
    while (cur != 0) {
        const tok_ptr: u32 = @bitCast(obj.read_i32(cur + ALIAS_REC_OFF_TOKEN_PTR));
        const tok_len: u32 = @bitCast(obj.read_i32(cur + ALIAS_REC_OFF_TOKEN_LEN));
        if (tok_len == name_len and bytes_eq(tok_ptr, name_ptr, name_len)) return cur;
        cur = @bitCast(obj.read_i32(cur + next_off));
    }
    return 0;
}

fn chain_head_offset(chain: AliasChain) u32 {
    return switch (chain) {
        .child => @offsetOf(Interp, "aliases_head"),
        .target => @offsetOf(Interp, "alias_targets_head"),
    };
}

fn chain_next_offset(chain: AliasChain) u32 {
    return switch (chain) {
        .child => ALIAS_REC_OFF_NEXT_IN_CHILD,
        .target => ALIAS_REC_OFF_NEXT_TARGET_IN_PARENT,
    };
}

fn bytes_eq(a: u32, b: u32, len: u32) bool {
    if (len == 0) return true;
    const ap: [*]const u8 = @ptrFromInt(a);
    const bp: [*]const u8 = @ptrFromInt(b);
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        if (ap[i] != bp[i]) return false;
    }
    return true;
}

// AliasRec field offsets that this module reaches into via direct
// memory loads.  Kept in sync with ``cmds/tcl_alias.zig``'s
// ``AliasRec`` ``extern struct`` layout via the comptime assert
// below.
const ALIAS_REC_OFF_TARGET_NAME_PTR: u32 = 0;
const ALIAS_REC_OFF_TARGET_NAME_LEN: u32 = 4;
const ALIAS_REC_OFF_N_PREFIX: u32 = 8;
const ALIAS_REC_OFF_PREFIX_ARGS_ADDR: u32 = 12;
const ALIAS_REC_OFF_PARENT_INTERP: u32 = 16;
const ALIAS_REC_OFF_CHILD_INTERP: u32 = 20;
const ALIAS_REC_OFF_TOKEN_PTR: u32 = 24;
const ALIAS_REC_OFF_TOKEN_LEN: u32 = 28;
const ALIAS_REC_OFF_NEXT_IN_CHILD: u32 = 32;
const ALIAS_REC_OFF_NEXT_TARGET_IN_PARENT: u32 = 36;
const ALIAS_REC_SIZE: u32 = 40;

/// Set on an ``Interp`` once ``interp delete path`` has torn it down.
/// Dispatch paths that might still hold the stale ``Interp*`` (cross-
/// interp alias redirect Commands whose ``OFF_IMPORT_REF_HEAD`` slot
/// points here, or pending ``enter`` frames unwinding) check this bit
/// and surface a clean "unknown command" diagnostic instead of
/// walking a zeroed ``root_ns``.  Set by :func:`child_delete_recursive`
/// as part of the cascade.
pub const INTERP_DELETED: u32 = 0x2;

/// Bucket size shared across ``children`` and ``hidden_cmd_table``.
/// Keeps ``Table(16)`` monomorphised once; matches the
/// ``NS_BUCKET_SIZE`` used throughout the namespace tree so the
/// ``OFF_HANDLE`` convention (value at offset 12) stays uniform.
pub const BUCKET_SIZE: u32 = 16;

/// Initial sub-table capacity.  Most interps have zero to a handful
/// of children and a small number of hidden commands; 16 buckets
/// keeps the load factor low with minimal waste.
pub const INITIAL_CAP: u32 = 16;

const ChildTable = ht.Table(BUCKET_SIZE);
const HiddenTable = ht.Table(BUCKET_SIZE);

/// Per-interp state.  ``extern struct`` layout for the same reason
/// ``tcl_ns.Namespace`` is extern: we hand bare ``u32`` addresses
/// across the WASM ABI and need guaranteed field offsets.
pub const Interp = extern struct {
    /// This interp's root (global) ``Namespace`` address.  For the
    /// root interp this equals ``tcl_ns.root_addr`` (set when
    /// ``interp_root()`` adopts the lazily-allocated root ns); for
    /// child interps, a fresh ``Namespace`` allocated via
    /// ``tcl_ns.ns_alloc_root``.
    root_ns: u32,

    /// Interpreter-wide hidden-commands table.  C Tcl keeps one
    /// ``hiddenCmdTable`` per interp (``Interp.hiddenCmdTablePtr``)
    /// and cross-interp ``hide`` / ``expose`` primitives route
    /// through the resolved target interp's slot.  Pre-child-interp
    /// this lived as a module-global in ``tcl_ns.zig``.
    hidden_cmd_table: HiddenTable,

    /// Parent interp.  Zero only for the root interp.  Child creation
    /// walks the chain in reverse to build the full path for
    /// introspection helpers (not shipped this wave).
    parent: u32,

    /// Simple name in the parent's ``children`` table.  Zero-len for
    /// the root interp.
    name_ptr: u32,
    name_len: u32,

    /// Children keyed by simple name -> ``Interp*``.  Lazily
    /// initialised: ``buf == 0`` means "no children yet".
    children: ChildTable,

    /// ``INTERP_SAFE | ...``.  Safe interps currently behave
    /// identically to unsafe ones — see the top-of-file comment.
    flags: u32,

    /// Monotonic id issuer for ``interp create`` without an explicit
    /// path — matches C Tcl's per-parent ``idIssuer`` on the
    /// ``Parent`` struct (``tmp/tcl9.0.3/generic/tclInterp.c``).
    /// The anonymous name ``interp<N>`` advances this counter; it
    /// stays per-parent so siblings in different parents don't
    /// collide, and so a deleted-then-recreated anonymous interp
    /// under one parent doesn't skew the issuer state in another.
    id_issuer: u32,

    /// Per-interp recursion limit (``Tcl_SetRecursionLimit`` /
    /// ``interp recursionlimit``).  Stored as ``u32`` for ABI
    /// stability; ``0`` is treated as "uninitialised" and seeded to
    /// the default (1000) on first read.  Enforcement lives in
    /// :fn:`tcl_interp.eval_command` which compares
    /// :attr:`num_levels` against this value and raises ``too many
    /// nested evaluations (infinite loop?)`` when exceeded
    /// (interp-29.3.x).
    recursion_limit: u32,

    /// Current call-stack depth inside this interp.  Mirrors C Tcl's
    /// ``Interp.numLevels`` — incremented at each ``eval_command``
    /// entry and decremented on the way out.  Read by the recursion-
    /// limit gate (above) and by the absorber for ``return -code N``
    /// (``TclUpdateReturnInfo`` runs only at ``numLevels == 0``).
    num_levels: u32,

    /// Head of the doubly-linked list of aliases registered in this
    /// interp (``Interp.child.aliasTable`` head in C Tcl, but a
    /// simple chain serves the same purpose for our scale).  Drives
    /// ``interp aliases <path>`` and survives ``rename`` of the
    /// source command (the chain is keyed by the original token).
    aliases_head: u32,

    /// Head of the doubly-linked list of aliases whose *target*
    /// lives in this interp.  Mirrors C Tcl's
    /// ``Interp.parent.targetsPtr``: when this interp is torn down
    /// :fn:`mark_deleted_subtree` walks the list and clears every
    /// dangling source-side alias so a later dispatch through the
    /// child can't crash on a freed parent.
    alias_targets_head: u32,
};

/// Root-interp singleton.  ``interp_root()`` allocates lazily and
/// adopts ``tcl_ns.ns_root()`` as its ``root_ns``.  After the first
/// call ``current_interp`` is also set.
var root_interp_addr: u32 = 0;

/// Currently-active interp.  Every ``tcl_ns.root_addr`` swap goes
/// through here too — the registry always sets the two together so
/// the ns tree and the per-interp state stay in sync.  Public so
/// cross-module callers (e.g. ``tcl_interp.eval_interp_eval``) can
/// save/restore around a swap without an extra accessor.
pub var current_interp: u32 = 0;

/// Allocate and zero-initialise a new ``Interp`` struct.
fn alloc_interp() u32 {
    const size: u32 = @sizeOf(Interp);
    const addr = alloc(size);
    const slice: [*]u8 = @ptrFromInt(addr);
    @memset(slice[0..size], 0);
    return addr;
}

/// Return the root interp, allocating it (and adopting the existing
/// ``tcl_ns`` root namespace) on first call.  Idempotent — a second
/// call returns the same address.  All the other APIs in this module
/// assume the root interp is initialised, so callers reach through
/// ``interp_current()`` which hits this on first use.
pub fn interp_root() u32 {
    if (root_interp_addr != 0) return root_interp_addr;
    root_interp_addr = alloc_interp();
    const i: *Interp = @ptrFromInt(root_interp_addr);
    i.root_ns = tcl_ns.ns_root();
    // Root interp has no parent, no simple name, no children yet.
    if (current_interp == 0) current_interp = root_interp_addr;
    return root_interp_addr;
}

/// Currently-active ``Interp`` address.  Zero means "no explicit
/// context set" — readers treat that as "root interp" and trigger
/// lazy root-interp init.
pub fn interp_current() u32 {
    if (current_interp != 0) return current_interp;
    return interp_root();
}

/// Look up a direct child of ``parent`` by simple name.  Returns 0
/// if the parent has no children table or the name doesn't match.
pub fn child_lookup(parent: u32, name_ptr: u32, name_len: u32) u32 {
    if (parent == 0) return 0;
    const p: *Interp = @ptrFromInt(parent);
    if (p.children.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (p.children.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
    }
    return 0;
}

/// Create a fresh child interp under ``parent`` with the given
/// simple name.  Allocates a new root namespace (via
/// ``tcl_ns.ns_alloc_root``), registers the child in the parent's
/// children table, and returns the new ``Interp*`` address.
///
/// Caller must have verified the name doesn't collide — a second
/// create with the same (parent, name) overwrites the old entry's
/// bucket-value slot with the new child, leaving the old child
/// unreachable from this parent (matches C Tcl's behaviour since
/// ``ChildCreate`` rejects collisions up front, which we do at the
/// callsite in ``tcl_interp.eval_interp_create``).
pub fn child_create(parent: u32, name_ptr: u32, name_len: u32, flags: u32) u32 {
    const p: *Interp = @ptrFromInt(parent);
    p.children.init(INITIAL_CAP);
    if (p.children.needs_grow()) p.children.grow();

    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = if (p.children.find(name_ptr, name_len, hash)) |b|
        b
    else
        p.children.insert_header(name_ptr, name_len, hash);

    const child = alloc_interp();
    const c: *Interp = @ptrFromInt(child);
    c.parent = parent;
    c.flags = flags;
    c.root_ns = tcl_ns.ns_alloc_root();
    // Inherit the parent's recursion limit (C Tcl's
    // ``Tcl_CreateChild`` copies ``parent->maxNestingDepth``).
    // interp-29.4.1/29.4.2 rely on this so a grandchild's
    // recursion gate fires at the parent's lowered limit.
    if (p.recursion_limit == 0) p.recursion_limit = DEFAULT_RECURSION_LIMIT;
    c.recursion_limit = p.recursion_limit;

    if (name_len > 0) {
        const nbuf = alloc(name_len);
        memcpy(nbuf, name_ptr, name_len);
        c.name_ptr = nbuf;
        c.name_len = name_len;
    }

    write_i32(bucket + tcl_ns.OFF_HANDLE, @bitCast(child));
    return child;
}

/// Delete a child interp.  Runs the full cascade:
///
/// 1. Recursively delete every grandchild (depth-first so
///    ``INTERP_DELETED`` propagates down the tree).
/// 2. Mark the Interp as ``INTERP_DELETED`` so any lingering
///    alias redirect Command whose ``OFF_IMPORT_REF_HEAD`` stashed
///    this Interp* sees the dead bit at dispatch time and raises
///    a clean "unknown command" instead of walking a zeroed ns.
/// 3. Tombstone the parent's children-table bucket so subsequent
///    lookups ( ``child_lookup`` / ``resolve_path``) miss.
///
/// The child's namespace subtree, cmd tables, hidden-commands
/// table, and the Interp struct itself stay live in bump memory —
/// the bump allocator's "never frees" contract hasn't changed.
/// What IS guaranteed: nothing routes back into the deleted
/// interp via its old address, because every live-path check
/// (alias dispatch, name-lookup) consults ``INTERP_DELETED``
/// and/or the parent's registry bucket.
pub fn child_delete(parent: u32, name_ptr: u32, name_len: u32) bool {
    if (parent == 0) return false;
    const p: *Interp = @ptrFromInt(parent);
    if (p.children.buf == 0) return false;
    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = p.children.find(name_ptr, name_len, hash) orelse return false;
    const handle: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
    if (handle == 0) return false;

    // Step 1 — recursive teardown.  ``mark_deleted_subtree`` walks
    // the children table (and any children's children) tagging each
    // with ``INTERP_DELETED`` depth-first, then tombstones the
    // bucket slots.
    mark_deleted_subtree(handle);

    // Step 3 — tombstone the entry in the parent.
    write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
    return true;
}

/// Recursively flag ``interp`` and every descendant as
/// ``INTERP_DELETED``.  Children's child-table buckets are
/// tombstoned along the way so re-reaching them via the parent
/// chain (e.g. a stale cached handle) misses cleanly.
fn mark_deleted_subtree(interp: u32) void {
    const i: *Interp = @ptrFromInt(interp);
    i.flags |= INTERP_DELETED;

    // Walk this interp's ``alias_targets_head`` chain — these are
    // AliasRecs in OTHER interps whose target lives here.  Clear
    // each one so a later dispatch through the source interp
    // doesn't try to invoke a command in this (now-dead) target.
    // Mirrors C Tcl's ``DeleteAlias`` cleanup that fires from
    // ``DeleteInterpProc`` walking ``Interp.parent.targetsPtr``.
    var cur_target: u32 = i.alias_targets_head;
    while (cur_target != 0) {
        const r: *@import("../cmds/tcl_alias.zig").AliasRec = @ptrFromInt(cur_target);
        const next: u32 = r.next_target_in_parent;
        // Find the live Command for this alias in its child interp
        // and clear it (deactivates dispatch + drops the
        // namespace-table bucket).  alias_clear unhooks from both
        // chains so the source interp's ``interp aliases`` no
        // longer reports a dangling entry.
        if (r.child_interp != 0 and r.token_len > 0) {
            const child: *Interp = @ptrFromInt(r.child_interp);
            // Walk the child's ns tree for a Command whose
            // params_obj == r.  Same shape as
            // ``find_command_by_rec`` in tcl_alias.zig but
            // inlined here to avoid the cross-module import
            // cycle.
            const found = find_alias_cmd_in_subtree(child.root_ns, cur_target);
            if (found.cmd != 0 and found.ns != 0) {
                // Clear the live cmd_table entry.  alias_clear is
                // called below to unhook chain entries; bucket
                // tombstoning happens here so the source name
                // resolves to "unknown command" on next dispatch.
                //
                // The Command's stored name is the FQN
                // (``::foo``); the cmd_table bucket is keyed by
                // the simple name (``foo``).  Strip the prefix
                // before clearing.
                const name_ptr: u32 = @bitCast(read_i32(found.cmd));
                const name_len: u32 = @bitCast(read_i32(found.cmd + 4));
                const simple = simple_of_fqn(name_ptr, name_len);
                _ = tcl_ns.ns_cmd_clear(found.ns, simple.ptr, simple.len);
            }
            // Always unhook from both chains, even when the
            // Command lookup missed (the alias may have been
            // ``rename``d off; the chain still holds it under
            // its original token).
            const alias_mod = @import("../cmds/tcl_alias.zig");
            alias_mod.alias_clear_rec(cur_target);
        }
        cur_target = next;
    }
    i.alias_targets_head = 0;

    if (i.children.buf == 0 or i.children.cap == 0) return;
    var k: u32 = 0;
    while (k < i.children.cap) : (k += 1) {
        const bucket = i.children.buf + k * BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) continue;
        const handle: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (handle == 0) continue;
        mark_deleted_subtree(handle);
        write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
    }
}

const AliasCmdAndNs = struct { cmd: u32, ns: u32 };

/// Strip the namespace prefix off a fully-qualified command name,
/// returning the trailing simple name.  ``::foo::bar`` → ``bar``,
/// bare ``foo`` → ``foo``.
fn simple_of_fqn(name_ptr: u32, name_len: u32) struct { ptr: u32, len: u32 } {
    if (name_len == 0) return .{ .ptr = name_ptr, .len = 0 };
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    var i: u32 = name_len;
    while (i >= 2) : (i -= 1) {
        if (sp[i - 2] == ':' and sp[i - 1] == ':') {
            return .{ .ptr = name_ptr + i, .len = name_len - i };
        }
    }
    return .{ .ptr = name_ptr, .len = name_len };
}

fn find_alias_cmd_in_subtree(ns: u32, target_rec: u32) AliasCmdAndNs {
    if (ns == 0 or target_rec == 0) return .{ .cmd = 0, .ns = 0 };
    // Heap-range guard: a torn-down interp's ``root_ns`` can leave
    // ``cmd_table.buf`` pointing at memory that was reclaimed or
    // never mapped (basic-10.1's ``namespace delete ::`` then
    // ``interp delete`` sequence).  Skip cleanly rather than
    // ``read_i32``-trap.
    const mem_pages: u32 = @intCast(@wasmMemorySize(0));
    const mem_bytes: u64 = @as(u64, mem_pages) * 65536;
    if (@as(u64, ns) + @sizeOf(tcl_ns.Namespace) > mem_bytes) return .{ .cmd = 0, .ns = 0 };
    const n: *const tcl_ns.Namespace = @ptrFromInt(ns);
    // Walk this ns's cmd_table.
    if (n.cmd_table.buf != 0 and n.cmd_table.cap > 0 and n.cmd_table.cap < (1 << 20)) {
        const table_bytes: u64 = @as(u64, n.cmd_table.cap) * @as(u64, tcl_ns.NS_BUCKET_SIZE);
        if (@as(u64, n.cmd_table.buf) + table_bytes <= mem_bytes) {
            var k: u32 = 0;
            while (k < n.cmd_table.cap) : (k += 1) {
                const bucket = n.cmd_table.buf + k * tcl_ns.NS_BUCKET_SIZE;
                const cmd: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
                if (cmd == 0) continue;
                // Bounds-check the Command pointer before reading
                // ``OFF_PARAMS_OBJ``.  Builtin forwards and alias
                // redirects sometimes hold sentinel handles outside
                // the heap range; reading through them would trap.
                const procs_mod = @import("tcl_procs.zig");
                if (@as(u64, cmd) + procs_mod.OFF_PARAMS_OBJ + 4 > mem_bytes) continue;
                const params: u32 = @bitCast(read_i32(cmd + procs_mod.OFF_PARAMS_OBJ));
                if (params == target_rec) return .{ .cmd = cmd, .ns = ns };
            }
        }
    }
    // Recurse into children namespaces.
    if (n.child_table.buf == 0 or n.child_table.cap == 0 or n.child_table.cap >= (1 << 20)) {
        return .{ .cmd = 0, .ns = 0 };
    }
    const child_bytes: u64 = @as(u64, n.child_table.cap) * @as(u64, tcl_ns.NS_BUCKET_SIZE);
    if (@as(u64, n.child_table.buf) + child_bytes > mem_bytes) return .{ .cmd = 0, .ns = 0 };
    var k: u32 = 0;
    while (k < n.child_table.cap) : (k += 1) {
        const cb = n.child_table.buf + k * tcl_ns.NS_BUCKET_SIZE;
        const child_ns: u32 = @bitCast(read_i32(cb + tcl_ns.OFF_HANDLE));
        if (child_ns == 0) continue;
        const r = find_alias_cmd_in_subtree(child_ns, target_rec);
        if (r.cmd != 0) return r;
    }
    return .{ .cmd = 0, .ns = 0 };
}

/// Predicate: has this Interp been torn down via ``interp delete``?
/// Dispatch paths that hold a stashed Interp* (cross-interp alias
/// redirects) call this before entering the interp.
pub fn is_deleted(interp: u32) bool {
    if (interp == 0) return true;
    const i: *Interp = @ptrFromInt(interp);
    return (i.flags & INTERP_DELETED) != 0;
}

/// Tcl_procs offsets / flag constants we reach into without
/// importing ``tcl_procs.zig`` (which imports ``tcl_ns.zig`` which
/// would create a cycle if we imported back).  Kept in sync with
/// ``tcl_procs.zig`` via the ``comptime`` assert below (using the
/// ``tcl_ns.tcl_procs_constants`` shadow that ``tcl_procs.zig``
/// already verifies against itself).
const COMMAND_SIZE: u32 = 52;
comptime {
    if (COMMAND_SIZE != tcl_ns.tcl_procs_constants.COMMAND_SIZE)
        @compileError("tcl_interp_registry.COMMAND_SIZE out of sync with tcl_ns.tcl_procs_constants.COMMAND_SIZE");
}
const OFF_CMD_NAME_PTR: u32 = 0;
const OFF_CMD_NAME_LEN: u32 = 4;
const OFF_CMD_FLAGS: u32 = 8;
const OFF_CMD_PARAMS_OBJ: u32 = 12;
const CMD_INTERP_CHILD: u32 = 0x200;

/// Allocate + populate a Command with the ``CMD_INTERP_CHILD`` flag
/// set, carrying ``child_interp`` in its ``params_obj`` slot.
/// Registered into the parent's ``cmd_table`` under the child's
/// simple name by ``eval_interp_create`` so ``myChild eval {...}``
/// resolves at the regular dispatch layer with one extra flag
/// check (same shape used by aliases).
pub fn alloc_child_command(
    parent_ns: u32,
    child_simple_ptr: u32,
    child_simple_len: u32,
    child_interp: u32,
) u32 {
    const cmd = alloc(COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(cmd);
    @memset(slice[0..COMMAND_SIZE], 0);

    // Stamp the FQN in the Command's name slot — same pattern
    // aliases follow.  This keeps ``info commands`` / ``namespace
    // which -command`` happy.
    const fqn = tcl_ns.ns_build_fqn(parent_ns, child_simple_ptr, child_simple_len);
    write_i32(cmd + OFF_CMD_NAME_PTR, @bitCast(fqn.ptr));
    write_i32(cmd + OFF_CMD_NAME_LEN, @bitCast(fqn.len));
    write_i32(cmd + OFF_CMD_FLAGS, @bitCast(CMD_INTERP_CHILD));
    write_i32(cmd + OFF_CMD_PARAMS_OBJ, @bitCast(child_interp));
    return cmd;
}

/// Extract the stashed ``Interp*`` from a ``CMD_INTERP_CHILD`` Command.
/// Caller should gate on the flag first.
pub fn cmd_child_interp(cmd: u32) u32 {
    return @bitCast(read_i32(cmd + OFF_CMD_PARAMS_OBJ));
}

/// Resolve an interp path (a Tcl list of simple names) relative to
/// ``base`` and return the target ``Interp*``.  Empty path returns
/// ``base`` unchanged ("current interp" shortcut).  Returns 0 if
/// any intermediate component misses.
///
/// The path list is parsed via ``tcl_obj.list_count_elements`` /
/// ``list_element_at`` so each element's simple-name span can have
/// an arbitrary origin (bump-allocated, braced, quoted — the raw
/// span is forwarded to ``child_lookup`` verbatim).  Callers that
/// need to accept qualified names (``::foo::bar``) would have to
/// pre-resolve them — which we don't: interp-path components are
/// simple names by construction (``interp0``, ``foo``, etc.).
pub fn resolve_path(base: u32, path_ptr: u32, path_len: u32) u32 {
    if (path_len == 0) return base;
    const count = obj.list_count_elements(path_ptr, path_len);
    var cur = base;
    var i: i64 = 0;
    while (i < count) : (i += 1) {
        const elem = obj.list_element_at(path_ptr, path_len, i);
        cur = child_lookup(cur, path_ptr + elem.start, elem.len);
        if (cur == 0) return 0;
    }
    return cur;
}

/// Insert (or update) a hidden-table entry in ``interp``'s hidden
/// slot.  Mirrors the former module-global ``tcl_ns.hidden_put``;
/// the move to per-interp storage lets cross-interp ``interp hide``
/// target the resolved child interp rather than a shared table.
pub fn hidden_put(interp: u32, name_ptr: u32, name_len: u32, value: u32) u32 {
    const i: *Interp = @ptrFromInt(interp);
    i.hidden_cmd_table.init(INITIAL_CAP);
    const hash = ht.fnv1a(name_ptr, name_len);
    if (i.hidden_cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        write_i32(bucket + tcl_ns.OFF_HANDLE, @bitCast(value));
        return bucket;
    }
    if (i.hidden_cmd_table.needs_grow()) i.hidden_cmd_table.grow();
    const bucket = i.hidden_cmd_table.insert_header(name_ptr, name_len, hash);
    write_i32(bucket + tcl_ns.OFF_HANDLE, @bitCast(value));
    return bucket;
}

/// Find a hidden-table entry in ``interp`` by simple name.  Returns
/// the stored Command handle or 0 if absent.
pub fn hidden_find(interp: u32, name_ptr: u32, name_len: u32) u32 {
    if (interp == 0) return 0;
    const i: *Interp = @ptrFromInt(interp);
    if (i.hidden_cmd_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (i.hidden_cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
    }
    return 0;
}

/// Tombstone a hidden-table bucket in ``interp``.  Returns true if
/// an entry was cleared.
pub fn hidden_clear(interp: u32, name_ptr: u32, name_len: u32) bool {
    if (interp == 0) return false;
    const i: *Interp = @ptrFromInt(interp);
    if (i.hidden_cmd_table.buf == 0) return false;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (i.hidden_cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        if (read_i32(bucket + tcl_ns.OFF_HANDLE) == 0) return false;
        write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
        return true;
    }
    return false;
}

/// Expose read-only access to an interp's hidden table for iterators
/// (e.g. ``interp hidden``).  The raw buffer + capacity form lets
/// callers walk buckets directly at the shared 16-byte layout.
pub fn hidden_table_buf(interp: u32) u32 {
    if (interp == 0) return 0;
    const i: *Interp = @ptrFromInt(interp);
    return i.hidden_cmd_table.buf;
}

pub fn hidden_table_cap(interp: u32) u32 {
    if (interp == 0) return 0;
    const i: *Interp = @ptrFromInt(interp);
    return i.hidden_cmd_table.cap;
}

/// Saved state captured by :func:`enter` and restored by
/// :func:`leave`.  Packed as plain ``u32`` fields so
/// save/restore is a trivial trio of loads + stores — no branch, no
/// ABI surface for the WASM caller.
pub const EnterSave = struct {
    prev_interp: u32,
    prev_root_addr: u32,
    prev_current_ns: u32,
};

/// Enter ``target`` — swap ``current_interp`` / ``tcl_ns.root_addr``
/// / ``tcl_ns.current_ns`` for the duration of a nested eval, and
/// return the prior values so :func:`leave` can restore them.
///
/// The child interp's ``current_ns`` is reset to 0 ("no explicit
/// context; use root") so top-level calls in the child script land
/// in the child's root ns, not the parent's.  Saving
/// ``tcl_ns.root_addr`` covers the case where the parent is itself
/// a child and we need to unwind correctly on return.
pub fn enter(target: u32) EnterSave {
    const save: EnterSave = .{
        .prev_interp = current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    const t: *Interp = @ptrFromInt(target);
    current_interp = target;
    tcl_ns.root_addr = t.root_ns;
    tcl_ns.current_ns = 0;
    return save;
}

/// Inverse of :func:`enter` — restore the saved state.  Callers that
/// match ``enter`` / ``leave`` as a save/restore pair get transparent
/// nesting: ``interp eval child1 { interp eval child2 {...} }`` will
/// walk into child2 from child1 and unwind back through both.
pub fn leave(save: EnterSave) void {
    current_interp = save.prev_interp;
    tcl_ns.root_addr = save.prev_root_addr;
    tcl_ns.current_ns = save.prev_current_ns;
}

// --- Cross-interp variable link primitives ---
//
// ``CrossInterpLink`` is the descriptor shape the side-registry in
// :file:`tcl_xlinks.zig` materialises for every link installed via
// ``interp share-variable``.  ``transfer_result`` ferries a TclObj
// across an interp boundary by adding a destination-side retain;
// ``var_resolve`` / ``var_set`` / ``global_get`` / ``global_set``
// all consult the xlink registry before falling through to local
// resolution, so a linked variable's reads and writes route to
// the target interp transparently.
//
// The ``Interp`` struct is a typed handle (``extern struct``)
// reachable by ``u32`` address; child interps are tracked through
// :func:`child_create` / :func:`child_lookup` / :func:`child_delete`.
// ``interp delete`` calls :func:`tcl_xlinks.drop_for_interp` so a
// dangling link can't outlive its endpoint.
//
// ``transfer_result`` is the refcount-bookkeeping primitive that
// the dispatcher uses to keep TclObj retains balanced when handing
// a value across interp boundaries.  The companion ``interp share``
// / ``interp transfer`` Tcl commands (channel handoff in reference
// Tcl) stay no-op stubs — channels are single-instance file
// descriptors in the WASM runtime so cross-interp channel transfer
// has no meaning here.

/// Cross-interp variable link target.  Stored as the value side of
/// every entry in :file:`tcl_xlinks.zig`'s registry; the registry
/// indirects to a heap-allocated record so each link can carry its
/// own retained ``target_name`` TclObj without inflating the
/// directory bucket.
pub const CrossInterpLink = extern struct {
    /// ``Interp*`` address of the owning interp.  Must be live;
    /// :func:`interp_delete` invalidates by setting ``INTERP_DELETED``
    /// so dispatchers can surface a clean diagnostic.
    target_interp: u32,
    /// ``::``-qualified name of the target variable.  Owned TclObj —
    /// the link record retains it for its lifetime.
    target_name: i32,
};

/// Hand a TclObj result from one interp to another by adding a
/// fresh retain for the destination interp's hold.  Routes through
/// the ``interp share-variable`` machinery: when ``var_resolve`` /
/// ``var_set`` follows an xlink to the target interp, the source-
/// side handle is borrowed for the call duration and the
/// destination retains via this primitive on store.
///
/// The caller is responsible for dropping the source-side hold —
/// this primitive doesn't do it because the source interp's
/// ``pending_result`` slot is the caller's contract, not ours.
/// Calling sequence is typically:
///
///     const dest_value = transfer_result(from, to, src_value);
///     // … stamp dest_value into dest's pending …
///     // … then on the source side, ``consume(.OK)`` or whatever
///     // releases the original hold.
///
/// When ``value == 0`` (the "no-result" handle) this is a no-op.
pub fn transfer_result(from_interp: u32, to_interp: u32, value: i32) i32 {
    _ = from_interp;
    _ = to_interp;
    if (value == 0) return 0;
    // The result handle's bytes live in the shared linear memory
    // — both interps see the same address space, so the "transfer"
    // is purely refcount bookkeeping.  Add the destination's hold;
    // caller releases the source's hold separately.
    obj.tcl_obj_retain(value);
    return value;
}
