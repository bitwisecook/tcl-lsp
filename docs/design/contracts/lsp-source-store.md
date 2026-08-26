# The closed-file source store

Every file the language server reads that the editor does **not** have open —
the sibling a `source` edge points at, the files the workspace scan indexes, the
`pkgIndex.tcl` the package database is built from, the `config.ini` a session
layers under the editor's settings, the `.tclspec` a pack is loaded from — comes
through one seam: `tcl_lsp_core::vfs::SourceStore`. Read this before changing
where the server gets bytes from, or before adding a `std::fs` call to the
server or to a crate the server drives.

Open documents never touch it. They arrive as `didOpen` / `didChange`, live in
`Backend::documents`, and are authoritative over any on-disk copy.

## Why the seam exists

Natively the store is `std::fs` and nothing else. The seam exists for a target
that has no `std::fs` at all: the browser worker (`rust/tcl-lsp-server-wasm`),
which runs the same `LspService<Backend>` the native binary runs and gets its
bytes from the page over `postMessage`. Without the seam, that build starts,
answers single-document requests, and silently reports an empty workspace —
every whole-workspace path `.ok()`s its I/O and folds a failure into "nothing
here", so nothing fails loudly and nothing works.

## The trait and its two implementations

| | `NativeStore` | `MemoryStore` |
|---|---|---|
| Backing | `std::fs` | a `PathBuf`-keyed byte map the host fills |
| Directories | the real tree | implied: a path is a directory when some stored file is under it |
| A path it does not have | whatever `std::fs` says | `io::ErrorKind::NotFound` |
| URI aliasing | none | a URI → path table, so a file registered under the client's spelling is found under the server's |

**`NativeStore` is a literal delegation and that is its entire specification.**
A native server built with this module behaves byte-identically to one built
without it. Any cleverness added to it — caching, path rewriting, normalisation
— breaks that, and the native test suite is the only thing that proves it.

Two details of the trait are load-bearing because they encode what the call
sites did before it existed:

- **`read_dir` skips an unreadable entry rather than failing the listing.**
  Every walk in the tree was written `std::fs::read_dir(dir)?.flatten()`, so a
  mid-listing error costs that one entry and never the whole directory. Only the
  *open* error propagates.
- **`DirEntry` carries `is_dir` **and** `is_file`, both false for a symlink** —
  the two questions `std::fs::FileType` answers. The walks asked
  `file_type().is_dir()` / `.is_file()`, so a symlink was neither descended into
  nor indexed. Collapsing them to one flag would silently start indexing
  symlinked sources. A caller that *wants* the link target's kind asks
  `is_dir(path)` / `is_file(path)`, which go through `metadata` and do follow —
  which is what the package-database subdirectory walk needs, because C Tcl's
  `auto_path` descends a symlinked package directory.

## Where the trait lives, and why

`tcl_lsp_core::vfs`, re-exported as `tcl_lsp_server::vfs`.

It started in `tcl-lsp-server`, where every call site was. Completing the seam
moved the whole-workspace paths onto it, and two of those live below the server:
the package database (`tcl_lsp_core::package_resolver`) and `.tclspec` discovery
(`tcl_spectcl::discovery`). The trait therefore has to be visible to both.

`tcl-lsp-core` is the lowest crate that can hold the trait **and both
implementations undivided**: `SourceStore::read_source` is the shared lossy
decoder (`tcl_lsp_core::source_decode`, issue #1326), so a home below this crate
would split the decode default off the trait. The alternatives were considered
and rejected:

| Candidate | Why not |
|---|---|
| `tcl-core-types` | `#![no_std]` — it cannot name `std::fs`, `std::io`, or `std::path` at all. |
| `tcl-platform` | Bans syscalls by charter ("only traits + error/capability types"), so the impls could not live there; and it already owns `Filesystem`, the host-capability seam, so a second filesystem trait would put two owners of one axis in one crate. |
| A new crate | Not needed once `tcl-lsp-core` works, and `tcl-spectcl` already reaches `tcl-compiler`, so the graph gains no new depth. |

The cost is one new edge, `tcl-spectcl → tcl-lsp-core`. It introduces no cycle
(`tcl-lsp-core` reaches neither `tcl-spectcl` nor anything that does), and the
only crates that newly build `tcl-lsp-core` are `tcl-explorer` and
`tcl-registry`'s dev target.

## What is routed, and what is not

Routed — every path that reads a closed file:

| Path | Entry point |
|---|---|
| Closed-file read behind `read_document`, decode-report comparison, `scan_disk_file` | `Backend::store` |
| `config.ini` / `.tcl-lsp.ini` layers, the `exportConfig` write, the notice line-range read | `Backend::store` |
| Workspace folder scan (candidate walk + per-file read) | `collect_tcl_files`, `Backend::build_package_db_and_candidates` |
| Package database (`pkgIndex.tcl` / `tclIndex`, the `auto_path` config layers) | `PackageResolver::scan_path_in` / `scan_tree_in`, `effective_auto_path` |
| W120 transitive package scan, W123 package-defined-command scan, the recovery-widening package read | `refine_workspace_w120` / `refine_workspace_w123` |
| `.tclspec` discovery and pack reading | `tcl_spectcl::discovery::discover_in`, `tcl_spectcl::bundled::load_discovered_in` |
| The APL sibling-`implementation` lookup | `find_sibling_impl_vars` |

Deliberately still `std::fs`, because each is a capability a byte map does not
have and a browser has no equivalent of:

- **`tcl_lsp_core::tcl_install`** — probes the machine for an installed Tcl
  interpreter and its `auto_path`. A browser has no interpreter to find; it
  contributes an empty list, which is the honest answer.
- **`tcl_spectcl::cache`** — the compiled-pack disk cache, a native-only
  speed-up that already no-ops when its directory is unwritable.
- **`tcl_spectcl::discovery::normalise`** (`std::fs::canonicalize`) — resolving
  `..`, symlinks, and a relative path against the process's working directory.
  It already falls back to the path as given, which is the right answer for a
  store-supplied path: its spelling is the only one it has.
- **The `TCL_LSP_NUDGE_LOG` debug writer** — a developer diagnostic, off unless
  the environment variable is set.

Both `scan_path` / `scan_tree` and `discover` / `load_discovered` keep their
original no-store signatures as thin `NativeStore` wrappers, the same shape
`Backend::new` and `Backend::with_store` use. Native callers and their tests did
not move, so the store-taking form cannot change native behaviour by accident.

## The virtual spec-pack mount

`tcl_spectcl::discovery::bundled_dir` answers "the `specs/` directory beside the
running executable". A browser worker has no executable, no `specs/`, and no
filesystem to hold either.

So when the bundled tier finds nothing beside the executable, discovery walks
the store at one well-known prefix instead:

```rust
tcl_spectcl::discovery::VIRTUAL_PACK_MOUNT  // "/\0.tcl-lsp/specs"
```

Re-exported as `tcl_lsp_server::vfs::VIRTUAL_PACK_MOUNT`, because it is a
contract between the server and its host, not an internal detail.

The leading NUL is what makes it safe to consult on every session: no real
filesystem can name a path containing one, so a native `NativeStore` can only
ever answer "not found" there, and the mount cannot collide with, shadow, or be
shadowed by anything a user actually has on disk. The `.tcl-lsp` component keeps
it self-describing in the one place it *is* visible — a pack notice naming a
file the host supplied.

Two rules govern it:

1. **A real `specs/` directory wins.** The mount is consulted only when the
   bundled tier is otherwise empty, the same rule an on-disk `specs/` already
   applies to the embedded fallback.
2. **The mount is additive to the shipped loadables.** Files found there carry
   `Origin::HostMount`, and `bundled::load_discovered_in` decides its
   embedded-pack fallback on "no bundled file came from a *shipped directory*"
   rather than "the bundled tier is empty". A real `specs/` holds the eight
   shipped EDA loadables, so finding one means they are accounted for; a host
   that upserts one vendor pack has said nothing about the EDA libraries and
   must not silently lose them.

`MemoryStore`'s implied directories are what make the walk work: upserting
`<mount>/eda/xilinx.tclspec` makes `<mount>` and `<mount>/eda` listable with no
declaration from the host, at any depth.

## The host contract

`rust/tcl-lsp-server-wasm/worker.js` speaks raw LSP JSON-RPC, plus three object
messages that are not protocol traffic:

```js
{ tclLsp: "upsert", uri, text }           // a closed file, keyed by file: URI
{ tclLsp: "delete", uri }                 // forget one
{ tclLsp: "upsertSpecPack", name, text }  // a .tclspec, under VIRTUAL_PACK_MOUNT
```

`upsertSpecPack` is separate from `upsert` because it does not key on a URI —
the mount deliberately is not a path a `file:` URI can spell.
`LspWorker.spec_pack_mount()` reports the prefix for a host that would rather
build the paths itself.

Its `name` is relative to the mount and must stay inside it: a rooted name (which
`Path::join` would let replace the mount outright) or one carrying a `..`
component is refused and logged, returning `false`, so a pack upsert can never
shadow an unrelated store path. The guard tests `has_root`, not `is_absolute` —
see the first wasm fault below for why the latter would admit exactly what it
exists to refuse.

**Ordering is the host's job, and the protocol already says it:** send all three
before `initialize`, because `initialized` is what loads the pack set and runs
the workspace scan. A file that appears later needs no new message — upsert it
and post an ordinary `workspace/didChangeWatchedFiles`, the same notification an
editor sends for a file changed outside it.

Declaring `workspaceFolders` in `initialize` is what turns the store into a
workspace: the scan walks each folder through the store exactly as it walks a
directory natively.

## Two wasm-only faults the seam exposed

Neither is about the store, and both only appear once a session has more than
one file — which is why they survived until the whole-workspace paths were
routed.

- **`Path::is_absolute` is `false` for every path on
  `wasm32-unknown-unknown`.** `std` requires `unix`, `wasi`, or a Windows path
  *prefix*, and that target is none of them. `ls_types::Uri::from_file_path`
  gates on it, takes its relative-path branch, and tries to canonicalise against
  a filesystem that is not there — so it returned `None` for every path the
  server derived itself, and the scan dropped each file before reading it.
  `uri_norm::rooted_file_uri` is the fallback, gated by `cfg!` (not `#[cfg]`) so
  it stays type-checked and unit-tested on every host, and it produces the same
  spelling `from_file_path`'s non-Windows branch does.

  It percent-encodes the path **unconditionally**, with `ls_types`' own rule
  (keep alphanumeric plus `-._~` and the `/` separator, escape everything else).
  Encoding only when the naive `file://<path>` fails to parse is not enough and
  cannot be made enough: `#`, `?`, and a literal `%XX` are *valid* URI syntax, so
  `/ws/a#1.tcl` parses happily as `file:///ws/a` with a fragment and aliases a
  different file. `repair_uri_string` could never catch it either — `is_uri_legal`
  admits `?` and the gen-delims by design, because its job is repairing a URI a
  client sent, not spelling one from a path. The equivalence test drives `#`, `?`,
  `%20`, `:`, a space, non-ASCII, and `&=;` names and asserts both a round trip
  through `to_file_path` and byte equality with `from_file_path`.
- **`crate::rt`'s browser `JoinSet` used to be inert until drained.** The
  workspace scan bounds its concurrency by taking a semaphore permit *before*
  spawning and releasing it *inside* the task, so a set whose tasks only
  advanced during `join_next` deadlocked the moment a workspace held more files
  than permits. The browser arm now detaches each task as it is added, matching
  Tokio's own contract, which is what every call site assumes — and, because the
  tasks are detached, it carries an `abort_all`-equivalent `Drop` so a set
  dropped mid-drain stops them, which is what Tokio's `JoinSet` does and what
  the set's own `FuturesUnordered` used to do for free.

## Tests

- `rust/tcl-lsp-core/src/vfs.rs` — the store's own semantics, including that
  `NativeStore` really is `std::fs`.
- `rust/tcl-lsp-server/src/lib.rs` — `collect_tcl_files` over a `MemoryStore`
  workspace, and `uri_norm`'s rooted-path URI fallback.
- `rust/tcl-spectcl/src/discovery.rs` — the virtual mount over a `MemoryStore`,
  its precedence against a real bundled directory, that it cannot name a real
  file, and that a host pack does not displace the shipped loadables.
- `rust/tcl-lsp-server-wasm/test/e2e.mjs` (`make lsp-server-wasm-test`) — the
  end-to-end proof: a workspace served entirely from the store, where the scan
  indexes upserted siblings, `source`d and auto-loaded definitions resolve, and
  a host-supplied `.tclspec` loads beside the shipped ones.
- The native suites are the equivalence proof for everything above:
  `cargo test -p tcl-lsp-server` (unit + the `tests/*_e2e.rs` suites),
  `cargo test -p tcl-lsp-core`, `cargo test -p tcl-spectcl`.
