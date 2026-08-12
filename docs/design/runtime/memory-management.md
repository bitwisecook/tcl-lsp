# WASM runtime: memory management

The WASM runtime is the Rust crate `tcl-runtime` (`runtime/rust/`). Its object
lifecycle lives in `runtime/rust/src/obj.rs` and allocates through Rust's
global allocator. This document fixes the **refcount discipline** every
reference holder in that runtime follows, and the leak-detection machinery that
enforces it.

The per-export ownership rows are
[`refcount-contract.md`](refcount-contract.md); the C-API surface's rows are
[`c-api-ownership-contract.md`](c-api-ownership-contract.md).

## MM-B — the refcount discipline

**Every TclObj reference holder explicitly retains and releases.** Once that
holds, refcount-driven `free` is the canonical lifetime mechanism, leaks
become impossible, and the borrow-versus-own ambiguity has one answer: *if you
hold a reference, you took a retain.*

Every site falls into one of four categories, and each carries a one-line
comment stating which:

- **creates** a TclObj — refcount 0 at birth (`obj.rs` constructors return the
  `fresh_zero` C-API object), so the first holder retains;
- **stores** an existing TclObj into a slot it did not allocate — must retain;
- **evicts** an existing TclObj from a slot — must release;
- **transfers ownership** across an API boundary — one side retains, the other
  releases, stated at the boundary.

### The store shape

Every slot store has the same shape: retain the incoming value, then release
the one it displaces. The order matters — retain-before-release is what makes
a self-store (`set x $x`) safe when the two are the same object.

```rust
// VarTable::store_scalar, the Var::Scalar arm (frame.rs).
Some(Var::Scalar(slot)) => {
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(*slot);
    }
    *slot = obj;
    Ok(())
}
```

A cell is the `Var` enum, not a struct with a nullable value field: `Scalar`
owns **+1** of one object, `Array` owns **+1** of each element, and `Link` (an
`upvar`/`global`/`variable` alias, resolved by path rather than by pointer)
owns nothing at all. `store_elem` is the same shape over the element map, and
`InterpState`'s result slot is the same shape again in `set_obj_result`.

`VarTable` releases on `Drop`, matching C's `TclFreeVar`, so a dropped frame or
namespace cannot leak and every refcount move stays visible to the counters.
There is no hand-written per-frame cleanup path to forget.

One deliberate exception to "+1 per holder": a TIP 508 array default
(`array default set`) is pinned with **+2**. The extra reference makes every
read of the default see a *shared* object, so a read-modify-write
(`lappend`/`append`/`dict set`) copies instead of mutating the stored default
in place.

### Proc parameter binding

The proc-call path binds each parameter to its argument word, so the frame
entry holds a reference to that word. The caller may release the words array
after dispatch; the frame's hold keeps the value alive. This is correct *by
construction* once the local-set path retains — it needs no separate rule.

### MM-B.6 — borrowed bytes and the stale-slab hazard

The hazard this rule exists to prevent: something holds a `(ptr, len)` into a
buffer it does not own, the owner is freed, the allocator reissues the slab,
and the borrow reads someone else's data. The runtime closes it structurally
in three places rather than by convention.

**A TclObj always owns its string rep.** `set_owned_string` frees the previous
buffer and allocates a fresh NUL-terminated one, copying the bytes in; every
constructor path (`new_string_obj`, `new_string_bytes`, each type's
`update_string_proc`) goes through it. There is no zero-capacity
"points at someone else's buffer" mode for a TclObj to be in, so the implicit
unlinked borrow cannot be constructed.

**Releasing the argv after dispatch needs no aliasing scan.** A command that
makes one of its argument words the result does so through `set_obj_result`,
which takes an *independent* `+1` before releasing the slot's prior occupant.
The eval loop can therefore release every argv element unconditionally after
`dispatch` returns — the result's own reference is not one of the ones being
dropped. No result-aliases-a-word scan, no deferred free queue, and no
special case for the `{*}`-expansion path.

**There is no parse cache to invalidate.** The parser's `WordPart::Text` /
`Variable` / `Command` borrow `&'s [u8]` straight from the source script, and a
`Text` run whose escapes had to be decoded owns its bytes as `Cow::Owned`.
Because the parse tree is lifetime-bound to the script it was parsed from, a
pointer-keyed cache outliving its buffer is a compile error rather than a
runtime hazard, and `parse.rs` is `#![forbid(unsafe_code)]`.

The structures that used to be listed here as borrow sites all hold owned
copies now, which is why they need no discipline of their own:

| Site | What it actually holds |
|---|---|
| namespace export patterns | `Namespace::exports: Vec<Vec<u8>>` — owned byte copies |
| alias descriptors | `Command::Alias { target: Vec<u8>, prefix: Vec<Vec<u8>> }` — owned byte copies, no TclObj reference |
| the `subst` concat pass | builds a fresh `Vec<u8>`; the single-substitution fast path returns the *object* with an owning `+1` |
| the frame argv used by `info level` | `Frame::words: Vec<Vec<u8>>` — owned byte copies, dropped with the frame |

## What the Rust runtime does not inherit

Two structural hazards from earlier designs cannot recur here, and it is worth
recording why so they are not reintroduced:

1. **Allocator incoherence.** A bump allocator growing upward from a fixed
   offset, alongside wasi-libc's dlmalloc growing upward from `__heap_base`,
   eventually collide and stomp each other's free-list metadata — which
   presents as an out-of-bounds access deep inside `malloc`. The Rust runtime
   never bump-allocates, and its regex engine is the pure-Rust `tcl-regex`
   crate (`runtime/rust/src/cmd_regex.rs`) rather than a C engine calling
   `MALLOC`/`FREE`, so there is only one allocator. The one C-shaped
   allocation surface left — `c_alloc.rs`, which supplies `malloc`/`free`/
   `realloc` to the libtommath archive on freestanding `wasm32-unknown` —
   forwards to Rust's global allocator, storing the size in a 16-byte header
   ahead of the user pointer because C's `free` does not carry Rust's layout.
   It is a shim over the same allocator, not a second one.
2. **Implicit shared buffers.** A TclObj with zero string capacity points into
   someone else's buffer with no link back to the lender. Either copy at every
   store site, or make the lender link explicit and refcounted (the borrowing
   object retains the lender at construction and releases it on its own
   release). What must not happen is an implicit borrow with no link, which is
   how the borrow outlives the lender. This runtime takes the first option
   unconditionally — see MM-B.6.

## MM-C — leak detection

Leak-check counters live in `runtime/rust/src/counters.rs`, exposed over the C
ABI in `capi.rs` as the `tcl_test_*` surface (`tcl_test_reset_counters`,
`tcl_test_alloc_count`, `tcl_test_double_free_count`, `tcl_test_finalize`).
They make every refcount move observable, which is what turns the discipline
above from a convention into something a test can assert: a balanced
round-trip leaves `tcl_test_finalize` at zero, and
`tcl_test_double_free_count` must be zero after any correct run.

Six counters are tracked: objects allocated and freed, string buffers
allocated and freed, releases of an already-zero-refcount object, and whether
the allocator has hit OOM. They are unconditional — the `leak-check` feature
gate that would compile them out of a production WASM build is not wired, so
the hot path pays for them on every target.

The counters are **thread-local**, not process-global. The production runtime
is a single-threaded WASM reactor, where per-thread and global are the same
thing; thread-local is what makes the native `cargo test` build correct under
parallel test execution, since each test thread checks its own counts instead
of racing another test's `reset`.

## Cross-references

- [`refcount-contract.md`](refcount-contract.md) — per-export ownership rows.
- [`c-api-ownership-contract.md`](c-api-ownership-contract.md) — the C-API
  surface's ownership and error categories.
- [`../contracts/runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md)
  — the cell model whose slots this discipline governs.
- [`../compiler/var-escape-analysis.md`](../compiler/var-escape-analysis.md) —
  the compile-side proof.
