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

- **creates** a TclObj — refcount 1 at birth, no retain needed;
- **stores** an existing TclObj into a slot it did not allocate — must retain;
- **evicts** an existing TclObj from a slot — must release;
- **transfers ownership** across an API boundary — one side retains, the other
  releases, stated at the boundary.

### The store shape

Every slot store has the same shape: retain the incoming value, release the one
it displaces.

```rust
fn set_scalar(slot: &mut Var, new: *mut TclObj) {
    let old = slot.value;
    slot.value = new;
    incr_ref_count(new);
    if !old.is_null() { decr_ref_count(old); }
}
```

The same shape applies to frame-local set (`frame.rs`), namespace variable set
(`namespace.rs`), and array element set. Getting this one shape right at every
slot is most of the discipline.

`VarTable` releases on `Drop`, matching C's `TclFreeVar`, so a dropped frame or
namespace cannot leak and every refcount move stays visible to the counters.

### Proc parameter binding

The proc-call path binds each parameter to its argument word, so the frame
entry holds a reference to that word. The caller may release the words array
after dispatch; the frame's hold keeps the value alive. This is correct *by
construction* once the local-set path retains — it needs no separate rule.

### Releasing the words array after dispatch

After a command completes, the parser-produced words array goes out of scope
and each entry must be released exactly once — with one exception that is easy
to get wrong and expensive to get wrong:

**When the dispatch result *aliases* one of the words** (`return $x`, a builtin
returning a word verbatim), the word-release loop has already decremented the
result's refcount. An unconditional trailing `release(result)` then pushes it to
zero, queueing the object for free while the caller still holds the handle; the
slab is reissued and the caller's `str_ptr` ends up pointing at the object's
own recycled refcount field. The rule is therefore: **scan the words for the
result and retain only if it aliases**, so the retain exactly cancels the
word-release decrement while a non-aliased result keeps its original refcount.
Both the fast path and the `{*}`-expansion path apply the same pattern.

### Borrowed `(ptr, len)` pairs

A structure that stores a borrowed `(ptr, len)` into a TclObj's buffer must
either **retain the source TclObj for as long as the borrow is held**, or
**copy the bytes into a fresh owned buffer at storage time**. There is no third
option, and every site that gets this wrong fails the same way: the source is
freed, its slab is reissued, and the borrow reads someone else's data.

Sites that carry such borrows:

| Site | Discipline |
|---|---|
| namespace export patterns | patterns stored as raw `(ptr, len)` — must not outlive their source |
| alias descriptors | `alias_alloc` retains each prefix TclObj |
| the `subst` concat pass | source TclObjs are retained in a scratch list until the concat completes |
| the parse cache | keyed on `(body_ptr, body_len)`, so it **must** be invalidated when a body buffer is freed |
| the frame argv used by `info level` | retained for the frame's lifetime, released on pop |

The parse cache deserves its own note, because a pointer-keyed cache with no
invalidation hook is a stale-slab generator: a body buffer is freed, libc
reissues the slab, and a later lookup with the recycled `(body_ptr, body_len)`
pair returns tokens pointing into freed memory. The hash table therefore
supports tombstones (`find` skips them, insertion reuses them, deletion marks
live entries) and the cache exposes `invalidate_for_buffer(buf_ptr)`, which
walks every bucket and tombstones any entry matching that buffer. The object
release path calls that hook immediately before freeing.

## What the Rust runtime does not inherit

Two structural hazards from earlier designs cannot recur here, and it is worth
recording why so they are not reintroduced:

1. **Allocator incoherence.** A bump allocator growing upward from a fixed
   offset, alongside wasi-libc's dlmalloc growing upward from `__heap_base`,
   eventually collide and stomp each other's free-list metadata — which
   presents as an out-of-bounds access deep inside `malloc`. The Rust runtime
   never bump-allocates, and its regex engine is the pure-Rust `tcl-regex`
   crate (`runtime/rust/src/cmd_regex.rs`) rather than a C engine calling
   `MALLOC`/`FREE`, so there is only one allocator.
2. **Implicit shared buffers.** A TclObj with zero string capacity points into
   someone else's buffer with no link back to the lender. Either copy at every
   store site, or make the lender link explicit and refcounted (the borrowing
   object retains the lender at construction and releases it on its own
   release). What must not happen is an implicit borrow with no link, which is
   how the borrow outlives the lender.

## Leak detection

Leak-check counters live in `runtime/rust/src/counters.rs`, exposed over the C
ABI in `capi.rs` as the `tcl_test_*` surface. They make every refcount move
observable, which is what turns the discipline above from a convention into
something a test can assert.

## Cross-references

- [`refcount-contract.md`](refcount-contract.md) — per-export ownership rows.
- [`c-api-ownership-contract.md`](c-api-ownership-contract.md) — the C-API
  surface's ownership and error categories.
- [`../contracts/runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md)
  — the cell model whose slots this discipline governs.
- [`../compiler/var-escape-analysis.md`](../compiler/var-escape-analysis.md) —
  the compile-side proof.
