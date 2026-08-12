// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The compiled-pack cache, in the OS cache directory.
//!
//! `docs/design/spec-packs.md`: on first load a pack's compiled form is
//! written to `$XDG_CACHE_HOME/tcl-lsp/spectcl/` (and the platform
//! equivalents) keyed by an **xxhash-class** digest of the pack source *plus
//! the `SpecTcl` vocabulary version and loader build*, so an edited pack or an
//! upgraded server recompiles exactly once and everything else is a hash check
//! and a fast read.
//!
//! ## Disposable by contract
//!
//! This is the property the whole module is built around, and it is a
//! *contract*, not an implementation detail: **delete the cache and nothing
//! breaks but first-load time.** Every failure path — a missing directory, an
//! unreadable file, a truncated write, a format from a future server, a
//! garbage byte in the middle — falls back to a fresh parse and, where it can,
//! rewrites the entry. No cache condition is ever surfaced to the user as an
//! error, because none of them is one. `cargo test` exercises exactly that:
//! [`tests::a_corrupt_entry_falls_back_to_a_fresh_parse`] truncates, transposes
//! and re-keys entries and asserts the load is byte-identical every time.
//!
//! ## What is cached
//!
//! The pack's **segmented statement tree** — the output of the CST route that
//! `docs/design/spec-packs.md` measures at 4.28 ms for a 2,000-line pack, and
//! the only part of loading that is expensive today.
//!
//! It is worth being precise about what is *not* here, because the design
//! promises more: "resolved drafts plus hook bytecode". Hook bodies are
//! carried as text and every pack hook installs as an abstaining function
//! pointer (see [`crate::loader`]) — there is no bytecode to cache until hook
//! bodies run on the VM (phase 5). And a resolved draft is a `CommandSpec`,
//! which holds function pointers into this binary and so cannot be written to
//! disk at all. What *is* here is the layer under both: the key scheme, the
//! directory, the atomic write, and the fallback contract, all of which the
//! richer payload will reuse unchanged.
//!
//! ## How it hooks into the loader
//!
//! [`crate::loader::statements`] is a pure function of `(source, base_line)`
//! and is the single door every level of a pack reaches the CST through. So
//! the cache is a **memo on that one function**, installed for the duration of
//! one [`load_pack_cached`] call. The loader neither knows nor can tell: with
//! no memo installed it parses exactly as before, which is why
//! [`crate::loader::load_pack`] stays the plain, cache-free entry point the
//! studio and the CLI use.

use std::cell::RefCell;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use xxhash_rust::xxh3::Xxh3;

use crate::VOCABULARY_VERSION;
use crate::loader::{Pack, Stmt, Word};

/// Set to `1` to bypass the on-disk cache entirely (still correct, just
/// slower). For bisecting a suspected cache bug without deleting a user's
/// directory.
pub const DISABLE_ENV: &str = "TCL_LSP_SPEC_CACHE_DISABLE";

/// Override for the cache directory, for an operator who wants the cache
/// somewhere other than the platform default.
pub const DIR_ENV: &str = "TCL_LSP_SPEC_CACHE_DIR";

/// In-process override of [`dir`] and the disable switch, set by
/// [`redirect_for_test`].
///
/// The env vars above are read *by* the server and cannot be set *from* it:
/// `std::env::set_var` is `unsafe` on edition 2024 and this workspace forbids
/// `unsafe`. So the test seam is a plain mutex rather than an environment
/// mutation — which is the better seam anyway, since it is scoped to the
/// process rather than inherited by every child it spawns.
static OVERRIDE: std::sync::Mutex<Option<(PathBuf, bool)>> = std::sync::Mutex::new(None);

/// Serialises every test that touches the process-global cache directory —
/// the ones in this module that *swap* it, and the ones elsewhere in the crate
/// that merely load a pack through it. Both have to be held apart: a load
/// writes a cache entry, and a redirected test counts the entries it wrote.
#[cfg(test)]
pub(crate) static REDIRECT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Point the cache at `dir` (and optionally switch it off) for the rest of the
/// process, or pass [`None`] to restore the configured behaviour.
///
/// Public because the LSP server's end-to-end tests need a cache that is
/// private to the test run; production configuration goes through [`DIR_ENV`].
pub fn redirect_for_test(target: Option<(PathBuf, bool)>) {
    if let Ok(mut guard) = OVERRIDE.lock() {
        *guard = target;
    }
}

fn override_of<T>(read: impl Fn(&(PathBuf, bool)) -> T) -> Option<T> {
    OVERRIDE.lock().ok()?.as_ref().map(read)
}

/// The on-disk format version. Bumped when the byte layout below changes; an
/// entry written by any other version is simply not read.
const FORMAT: u8 = 1;

/// File magic, so a stray file in the directory is never mistaken for an entry.
const MAGIC: &[u8; 7] = b"TCLSPKC";

/// The **loader build** half of the cache key.
///
/// Bump this whenever a change to [`crate::loader`] alters what a given
/// `.tclspec` source produces, in a release where the crate version does not
/// move (during development, that is every time). The workspace version is in
/// the key too, so a shipped upgrade already invalidates; this constant is what
/// keeps a working tree honest between releases.
///
/// It is deliberately a hand-maintained constant rather than, say, an
/// `include_str!` digest of the loader: hashing 120 KB of embedded source on
/// every startup to defend against a case only developers hit is a bad trade,
/// and the cost of forgetting is a stale parse of a file the developer is
/// actively editing — recovered by `rm -rf` on a directory the contract
/// already says is disposable.
const LOADER_BUILD: u32 = 1;

// ---------------------------------------------------------------------------
// The key
// ---------------------------------------------------------------------------

/// Mix the vocabulary version, crate version, loader build and format version
/// into `hasher` — everything about *this server* that changes what a pack
/// source means.
///
/// Shared with [`crate::pack`]'s pack-set key so the two cannot disagree about
/// what constitutes "the same build".
pub(crate) fn stamp_build(hasher: &mut Xxh3) {
    hasher.update(MAGIC);
    hasher.update(&[FORMAT]);
    hasher.update(VOCABULARY_VERSION.as_bytes());
    hasher.update(&[0]);
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(&[0]);
    hasher.update(&LOADER_BUILD.to_le_bytes());
}

/// The cache key for one pack source.
#[must_use]
pub fn key_for(source: &str) -> u64 {
    let mut hasher = Xxh3::new();
    stamp_build(&mut hasher);
    hasher.update(source.as_bytes());
    hasher.digest()
}

/// The compiled-pack cache directory: [`DIR_ENV`] when set, else
/// `<OS cache dir>/spectcl`.
#[must_use]
pub fn dir() -> PathBuf {
    if let Some(dir) = override_of(|(dir, _)| dir.clone()) {
        return dir;
    }
    if let Some(dir) = std::env::var_os(DIR_ENV)
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    tcl_userdirs::cache_dir().join("spectcl")
}

/// The entry path for `key`, fanned out one level so a workspace with many
/// packs does not build a single flat directory of thousands of files.
fn entry_path(key: u64) -> PathBuf {
    let hex = format!("{key:016x}");
    dir().join(&hex[..2]).join(format!("{hex}.pack"))
}

fn disabled() -> bool {
    override_of(|(_, off)| *off).unwrap_or(false)
        || std::env::var_os(DISABLE_ENV).is_some_and(|v| v == "1")
}

// ---------------------------------------------------------------------------
// The public entry point
// ---------------------------------------------------------------------------

/// Load a pack source, reading and writing the on-disk parse cache.
///
/// Semantically identical to [`crate::loader::load_pack`] — same [`Pack`],
/// same notices, same order — for every input, every cache state, and every
/// I/O failure. The only observable difference is how long it takes.
#[must_use]
pub fn load_pack_cached(source: &str) -> Pack {
    if disabled() {
        return crate::loader::load_pack(source);
    }
    let key = key_for(source);
    let path = entry_path(key);
    let seeded = read_entry(&path, key).unwrap_or_default();
    let had_entry = !seeded.is_empty();

    let (pack, memo) = with_memo(seeded, || crate::loader::load_pack(source));

    // Rewrite when the load parsed something the entry did not have: a cold
    // cache, an entry from a loader that visited fewer blocks, or a partial
    // read. An unchanged entry is left alone so a read-only cache directory
    // (or a read-only filesystem) costs nothing after the first miss.
    if !had_entry || memo.parsed_something_new {
        write_entry(&path, key, &memo);
    }
    pack
}

/// Delete the whole cache directory. Nothing depends on it existing, so this
/// is always safe; it exists for `tcl spec` maintenance verbs and for tests.
pub fn clear() -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir()) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// The memo — one segmentation table, live for one load
// ---------------------------------------------------------------------------

/// A memoised segmentation table.
///
/// Keyed by `(base_line, byte length, xxh3 of the text)`. The length is in the
/// key alongside the digest because a memo hit returns statements *without*
/// re-comparing the text: two different sources colliding would be a silent
/// wrong parse, so the key carries a second, independent discriminator.
#[derive(Debug, Default)]
struct Memo {
    entries: FxHashMap<(u32, u32, u64), Vec<Stmt>>,
    /// Insertion order, so a written entry replays in the order the loader
    /// asked for it — a rewrite of an unchanged pack is then byte-identical.
    order: Vec<(u32, u32, u64)>,
    /// Whether anything was segmented for real during this load.
    parsed_something_new: bool,
}

impl Memo {
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

thread_local! {
    /// The memo for the load running on this thread, if any. `None` — the
    /// default, and what [`crate::loader::load_pack`] sees — means every
    /// segmentation is done for real, which is the pre-cache behaviour.
    static MEMO: RefCell<Option<Memo>> = const { RefCell::new(None) };
}

fn memo_key(source: &str, base_line: u32) -> (u32, u32, u64) {
    (
        base_line,
        u32::try_from(source.len()).unwrap_or(u32::MAX),
        xxhash_rust::xxh3::xxh3_64(source.as_bytes()),
    )
}

/// Look up a segmentation. `None` whenever no memo is installed.
pub(crate) fn memo_get(source: &str, base_line: u32) -> Option<Vec<Stmt>> {
    MEMO.with(|cell| {
        let memo = cell.borrow();
        memo.as_ref()?.entries.get(&memo_key(source, base_line)).cloned()
    })
}

/// Record a segmentation. A no-op whenever no memo is installed.
pub(crate) fn memo_put(source: &str, base_line: u32, stmts: &[Stmt]) {
    MEMO.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some(memo) = guard.as_mut() else {
            return;
        };
        let key = memo_key(source, base_line);
        memo.parsed_something_new = true;
        if memo.entries.insert(key, stmts.to_vec()).is_none() {
            memo.order.push(key);
        }
    });
}

/// Run `body` with `seed` installed as this thread's memo, returning the
/// result and the memo as it ended up.
///
/// Restores whatever memo was installed before (there is none in practice —
/// loads do not nest — but leaving the slot as we found it keeps this a safe
/// thing to call from anywhere).
fn with_memo<T>(seed: Memo, body: impl FnOnce() -> T) -> (T, Memo) {
    let previous = MEMO.with(|cell| cell.borrow_mut().replace(seed));
    let out = body();
    let memo = MEMO.with(|cell| std::mem::replace(&mut *cell.borrow_mut(), previous));
    (out, memo.unwrap_or_default())
}

// ---------------------------------------------------------------------------
// The on-disk format
// ---------------------------------------------------------------------------
//
// A flat, length-prefixed little-endian encoding. Hand-rolled rather than
// serde because the shape is four structs deep and the reader's real job is
// not decoding but *refusing* to decode: every length is checked against the
// bytes actually remaining, so a truncated or scrambled file yields `None`
// rather than a panic or a wrong parse.
//
//   header   MAGIC | FORMAT | key:u64 | entry_count:u32
//   entry    base_line:u32 | text_len:u32 | text_hash:u64 | stmts
//   stmts    count:u32 | Stmt*
//   Stmt     line:u32 | word_count:u32 | Word*
//   Word     line:u32 | braced:u8 | len:u32 | bytes

fn write_entry(path: &Path, key: u64, memo: &Memo) {
    if memo.is_empty() {
        return;
    }
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    buf.extend_from_slice(MAGIC);
    buf.push(FORMAT);
    buf.extend_from_slice(&key.to_le_bytes());
    buf.extend_from_slice(&u32::try_from(memo.order.len()).unwrap_or(u32::MAX).to_le_bytes());
    for entry_key in &memo.order {
        let Some(stmts) = memo.entries.get(entry_key) else {
            continue;
        };
        let (base_line, text_len, text_hash) = *entry_key;
        buf.extend_from_slice(&base_line.to_le_bytes());
        buf.extend_from_slice(&text_len.to_le_bytes());
        buf.extend_from_slice(&text_hash.to_le_bytes());
        put_stmts(&mut buf, stmts);
    }
    let _ = write_atomically(path, &buf);
}

fn put_stmts(buf: &mut Vec<u8>, stmts: &[Stmt]) {
    buf.extend_from_slice(&u32::try_from(stmts.len()).unwrap_or(u32::MAX).to_le_bytes());
    for stmt in stmts {
        buf.extend_from_slice(&stmt.line.to_le_bytes());
        buf.extend_from_slice(&u32::try_from(stmt.words.len()).unwrap_or(u32::MAX).to_le_bytes());
        for word in &stmt.words {
            buf.extend_from_slice(&word.line.to_le_bytes());
            buf.push(u8::from(word.braced));
            buf.extend_from_slice(
                &u32::try_from(word.text.len()).unwrap_or(u32::MAX).to_le_bytes(),
            );
            buf.extend_from_slice(word.text.as_bytes());
        }
    }
}

/// Write via a uniquely-named sibling and rename, so a reader never sees a
/// half-written entry and two servers writing the same pack cannot interleave.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        std::process::id()
    ));
    {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    match std::fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = std::fs::remove_file(&temp);
            Err(err)
        }
    }
}

/// A bounds-checked reader. Every `take` is fallible, so the decoder below can
/// be written as a chain of `?`s and cannot read past the buffer.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Option<u64> {
        self.take(8).map(|b| {
            u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
        })
    }

    /// A length-prefixed count, refused when it could not possibly fit in what
    /// remains. Without this a corrupt `u32::MAX` count would ask for a 4-
    /// billion-element `Vec` before failing.
    fn count(&mut self, min_bytes_each: usize) -> Option<usize> {
        let n = self.u32()? as usize;
        let remaining = self.bytes.len().checked_sub(self.at)?;
        (n.checked_mul(min_bytes_each)? <= remaining).then_some(n)
    }

    fn string(&mut self) -> Option<String> {
        let len = self.count(1)?;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).ok()
    }
}

fn read_entry(path: &Path, key: u64) -> Option<Memo> {
    let bytes = std::fs::read(path).ok()?;
    let mut reader = Reader {
        bytes: &bytes,
        at: 0,
    };
    if reader.take(MAGIC.len())? != MAGIC || reader.u8()? != FORMAT {
        return None;
    }
    // The key is in the file as well as the filename: a hash-named file that
    // does not contain its own key is somebody else's, or a rename gone wrong.
    if reader.u64()? != key {
        return None;
    }
    let entries = reader.count(16)?;
    let mut memo = Memo::default();
    for _ in 0..entries {
        let base_line = reader.u32()?;
        let text_len = reader.u32()?;
        let text_hash = reader.u64()?;
        let stmts = get_stmts(&mut reader)?;
        let entry_key = (base_line, text_len, text_hash);
        if memo.entries.insert(entry_key, stmts).is_none() {
            memo.order.push(entry_key);
        }
    }
    // Trailing bytes mean the file is not what it claims to be.
    (reader.at == bytes.len()).then_some(memo)
}

fn get_stmts(reader: &mut Reader<'_>) -> Option<Vec<Stmt>> {
    let count = reader.count(8)?;
    let mut stmts = Vec::with_capacity(count);
    for _ in 0..count {
        let line = reader.u32()?;
        let words = reader.count(9)?;
        let mut list = Vec::with_capacity(words);
        for _ in 0..words {
            let word_line = reader.u32()?;
            let braced = reader.u8()? != 0;
            let text = reader.string()?;
            list.push(Word {
                text,
                braced,
                line: word_line,
            });
        }
        stmts.push(Stmt { words: list, line });
    }
    Some(stmts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pack big enough to have real nesting: a `values` table, descriptors,
    /// subcommands and hover blocks all segment separately.
    const SOURCE: &str = r"
speclib mylib 1 {
    values sortmodes {
        value -ascii  {doc {byte order}}
        value -dict   {doc {dictionary order}}
    }
    command mylib::sort {
        synopsis {mylib::sort ?options? list}
        arity 1..
        option -mode -takes enum -values sortmodes
        option --
        subcommand indices {
            arity 1
            hover -summary {Return indices rather than elements.}
        }
        hover -summary {Sort a list.} -returns {The sorted list.}
    }
    command mylib::with_var {
        arity 2..3
        arg 0 -role varwrite
        arg 1 -role body
        traits {EVALUATES_CODE}
    }
}
";

    struct CacheDir(PathBuf);

    impl CacheDir {
        /// Point the cache at a private directory for the duration of a test.
        ///
        /// The redirect is process-global, so every test that takes one runs
        /// under [`LOCK`].
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tcl-spectcl-cache-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            redirect_for_test(Some((dir.clone(), false)));
            Self(dir)
        }

        /// Switch the cache off without moving it.
        fn disable(&self) {
            redirect_for_test(Some((self.0.clone(), true)));
        }

        fn entries(&self) -> Vec<PathBuf> {
            let mut out = Vec::new();
            let mut stack = vec![self.0.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(read) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in read.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.extension().is_some_and(|e| e == "pack") {
                        out.push(path);
                    }
                }
            }
            out.sort();
            out
        }
    }

    impl Drop for CacheDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
            redirect_for_test(None);
        }
    }

    use super::REDIRECT_LOCK as LOCK;

    /// The comparable shape of a load: what the pack says, and everything the
    /// loader said about it. `CommandSpec` is not `PartialEq`, so equality is
    /// asserted over this projection.
    fn shape(pack: &Pack) -> String {
        use std::fmt::Write as _;
        let mut out = format!("{} v{}\n", pack.name, pack.dsl_version);
        for command in &pack.commands {
            let _ = writeln!(
                out,
                "  {} arity {}..{} traits {:?} subs {}",
                command.spec.name,
                command.spec.arity.min,
                command.spec.arity.max,
                command.spec.traits,
                command.spec.subcommands.len()
            );
        }
        for notice in &pack.notices {
            let _ = writeln!(out, "  notice {notice}");
        }
        out
    }

    #[test]
    fn a_cold_load_writes_an_entry_and_a_warm_load_matches_it() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = CacheDir::new("roundtrip");

        let cold = load_pack_cached(SOURCE);
        assert_eq!(cache.entries().len(), 1, "a cold load writes one entry");

        let warm = load_pack_cached(SOURCE);
        assert_eq!(shape(&cold), shape(&warm));
        assert_eq!(
            shape(&crate::loader::load_pack(SOURCE)),
            shape(&warm),
            "the cache never changes what a load produces"
        );
        assert_eq!(cache.entries().len(), 1, "a warm load rewrites nothing");
    }

    #[test]
    fn an_edit_writes_a_second_entry_and_leaves_the_first() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = CacheDir::new("edit");

        let _ = load_pack_cached(SOURCE);
        let edited = SOURCE.replace("arity 2..3", "arity 2..4");
        let after = load_pack_cached(&edited);

        assert_eq!(cache.entries().len(), 2, "an edit is a different key");
        assert_eq!(shape(&after), shape(&crate::loader::load_pack(&edited)));
    }

    #[test]
    fn a_corrupt_entry_falls_back_to_a_fresh_parse() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = CacheDir::new("corrupt");
        let expected = shape(&crate::loader::load_pack(SOURCE));

        let _ = load_pack_cached(SOURCE);
        let entry = cache.entries().pop().expect("one entry");
        let good = std::fs::read(&entry).expect("read entry");

        let mutations: Vec<(&str, Vec<u8>)> = vec![
            ("empty", Vec::new()),
            ("header only", good[..4].to_vec()),
            ("truncated mid-entry", good[..good.len() / 2].to_vec()),
            ("trailing garbage", [good.clone(), vec![0xff; 8]].concat()),
            ("wrong magic", [b"XXXXXXX".to_vec(), good[7..].to_vec()].concat()),
            ("wrong key", {
                let mut bytes = good.clone();
                bytes[8] ^= 0xff;
                bytes
            }),
            ("bogus count", {
                let mut bytes = good.clone();
                bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
                bytes
            }),
            ("scrambled body", {
                let mut bytes = good.clone();
                let tail = bytes.len() - 1;
                bytes[24..tail].reverse();
                bytes
            }),
        ];

        for (what, bytes) in mutations {
            std::fs::write(&entry, &bytes).expect("write mutation");
            let loaded = load_pack_cached(SOURCE);
            assert_eq!(shape(&loaded), expected, "{what} must not change the load");
        }
    }

    #[test]
    fn a_write_failure_is_not_an_error() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = CacheDir::new("unwritable");
        // A *file* where the cache directory should be: every create_dir_all
        // under it fails, so nothing can ever be written.
        if let Some(parent) = cache.0.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&cache.0, b"not a directory").expect("place blocker");

        let loaded = load_pack_cached(SOURCE);
        assert_eq!(shape(&loaded), shape(&crate::loader::load_pack(SOURCE)));
        let _ = std::fs::remove_file(&cache.0);
    }

    #[test]
    fn the_key_covers_the_build_not_just_the_source() {
        let a = key_for(SOURCE);
        let b = key_for(&SOURCE.replace("arity 1..", "arity 1..2"));
        assert_ne!(a, b, "different sources key differently");
        assert_eq!(a, key_for(SOURCE), "the same source keys the same");

        // The build stamp is genuinely mixed in: a digest of the source alone
        // would be a different number.
        let bare = xxhash_rust::xxh3::xxh3_64(SOURCE.as_bytes());
        assert_ne!(a, bare, "the key is not a plain digest of the source");
    }

    #[test]
    fn disabling_the_cache_changes_nothing_but_speed() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = CacheDir::new("disabled");
        cache.disable();
        let loaded = load_pack_cached(SOURCE);
        assert_eq!(shape(&loaded), shape(&crate::loader::load_pack(SOURCE)));
        assert!(cache.entries().is_empty(), "nothing was written");
    }

    #[test]
    fn the_cache_directory_is_under_the_os_cache_dir() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        redirect_for_test(None);
        if std::env::var_os(DIR_ENV).is_some_and(|v| !v.is_empty()) {
            // An operator-configured cache directory is by definition not the
            // platform default; there is nothing to assert about it.
            return;
        }
        let dir = dir();
        assert!(dir.ends_with("spectcl"), "{}", dir.display());
        assert!(dir.starts_with(tcl_userdirs::cache_dir()), "{}", dir.display());
    }

    #[test]
    fn clearing_an_absent_cache_is_not_an_error() {
        let _guard = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let cache = CacheDir::new("clear");
        let _ = load_pack_cached(SOURCE);
        assert!(!cache.entries().is_empty());
        clear().expect("clear a populated cache");
        assert!(cache.entries().is_empty());
        clear().expect("clear an absent cache");
    }
}
