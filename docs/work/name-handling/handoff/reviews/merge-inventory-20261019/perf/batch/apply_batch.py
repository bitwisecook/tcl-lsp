#!/usr/bin/env python3
"""Apply the perf batch to a tree root.

usage: apply_batch.py ROOT [--zz] [--only FILE,...] [--skip FILE,...]

--zz adds the experiment-only old/new toggles (tcl_lexer::ZZ_OLD / zz_old(),
tcl_compiler::realm::ZZ_NO_MEMO) used by the in-process equality probes.
Without --zz the result is the clean patch to land.
Every replacement asserts its anchor occurs exactly once.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
args = sys.argv[1:]
root = args[0]
zz = "--zz" in args
only = skip = None
if "--only" in args:
    only = set(args[args.index("--only") + 1].split(","))
if "--skip" in args:
    skip = set(args[args.index("--skip") + 1].split(","))


def block(name):
    path = os.path.join(HERE, name)
    if not os.path.exists(path):
        path += ".txt"
    with open(path) as f:
        return f.read()


def rep(s, old, new, path):
    n = s.count(old)
    if n != 1:
        sys.exit(f"anchor x{n} in {path}: {old[:120]!r}")
    return s.replace(old, new)


def between(s, start, end, new, path):
    if s.count(start) != 1 or s.count(end) != 1:
        sys.exit(f"region anchors not unique in {path}: {start[:80]!r} / {end[:80]!r}")
    a = s.index(start)
    b = s.index(end)
    return s[:a] + new + s[b:]


EDITS = {}


def edit(path):
    def deco(fn):
        EDITS[path] = fn
        return fn

    return deco


@edit("rust/tcl-lexer/src/lib.rs")
def lexer_lib(s, p):
    if not zz:
        return s
    return s + '''
/// ZZ perf probe toggle (experiment copy only): restore the old behaviour.
pub static ZZ_OLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// ZZ: toggle or env `ZZ_OLD_ENV`.
pub fn zz_old() -> bool {
    static ENV: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    ZZ_OLD.load(std::sync::atomic::Ordering::Relaxed)
        || *ENV.get_or_init(|| std::env::var_os("ZZ_OLD_ENV").is_some())
}
'''


@edit("rust/tcl-lexer/src/parse_cut.rs")
def parse_cut(s, p):
    toggle = (
        "        if crate::zz_old() {\n            image = None;\n        }\n" if zz else ""
    )
    s = rep(
        s,
        """    for (index, command) in commands.iter().enumerate() {
        if let Some((offset, term, kind)) =
            command_cut(command, tokens, src, config, channel, descent, anchor)?
        {""",
        """    // One image of the whole script serves every command: building it hashes
    // and line-indexes all of `src`, so a per-command image is quadratic.
    let mut image = None;
    for (index, command) in commands.iter().enumerate() {
"""
        + toggle
        + """        let image = image.get_or_insert_with(|| SourceImage::from_bytes(src, channel));
        if let Some((offset, term, kind)) =
            command_cut(command, tokens, image, config, channel, descent, anchor)?
        {""",
        p,
    )
    s = rep(
        s,
        """fn command_cut(
    command: &CommandSpan,
    tokens: &[Token],
    src: &[u8],
    config: LexerConfig,""",
        """fn command_cut(
    command: &CommandSpan,
    tokens: &[Token],
    image: &SourceImage,
    config: LexerConfig,""",
        p,
    )
    s = rep(
        s,
        "    let jobs = word_jobs(&command.words, tokens, src, 0, config);",
        "    let src = image.bytes();\n    let jobs = word_jobs(&command.words, tokens, src, 0, config);",
        p,
    )
    return rep(
        s,
        """        image: SourceImage::from_bytes(src, channel),
        jobs,
        base: 0,""",
        """        image: image.clone(),
        jobs,
        base: 0,""",
        p,
    )


@edit("rust/tcl-lexer/src/source_map.rs")
def source_map(s, p):
    fast = (
        "(!crate::zz_old() && std::sync::Arc::ptr_eq(&self.bytes, &other.bytes))"
        if zz
        else "std::sync::Arc::ptr_eq(&self.bytes, &other.bytes)"
    )
    return rep(
        s,
        "        self.channel == other.channel && self.bytes == other.bytes\n",
        """        // One shared allocation is trivially the same bytes; only distinct
        // allocations need the complete comparison.
        self.channel == other.channel
            && ("""
        + fast
        + """ || self.bytes == other.bytes)
""",
        p,
    )


@edit("rust/tcl-compiler/src/parsing/spectcl_source/original_word.rs")
def original_word(s, p):
    s = rep(
        s,
        """    pub fn in_document(&self) -> OriginalPackSourceWord {
        OriginalPackSourceWord {
            local: self.clone(),
            root: self.statement.image.clone(),
            root_config: self.statement.config,
            base: 0,
            parent: None,
            depth: 0,
        }
    }""",
        """    pub fn in_document(&self) -> OriginalPackSourceWord {
        let word = OriginalPackSourceWord {
            local: self.clone(),
            root: self.statement.image.clone(),
            root_config: self.statement.config,
            base: 0,
            parent: None,
            depth: 0,
        };
        debug_assert!(
            word.matches_source(&word.root, word.root_config),
            "a document word is its own complete original root"
        );
        word
    }""",
        p,
    )
    old_toggle = (
        """        if tcl_lexer::zz_old() {
            return candidate
                .matches_source(&candidate.root, candidate.root_config)
                .then_some(candidate);
        }
"""
        if zz
        else ""
    )
    s = rep(
        s,
        """        candidate
            .matches_source(&candidate.root, candidate.root_config)
            .then_some(candidate)
    }""",
        old_toggle
        + """        // `parent` was bound by `in_document` or `in_body`, which establish
        // `parent.matches_source(&parent.root, parent.root_config)`, and the
        // candidate shares that root and config: only the new parent link
        // needs checking. Re-proving the ancestor chain (or even one ancestor
        // link) per word compares that ancestor's whole body again; with debug
        // assertions load_embedded measured 6.9-7.0 s with such an assert vs
        // 1.9 s without. The root is debug-asserted in `in_document`, and
        // `every_bound_body_word_satisfies_the_full_chain_proof` asserts the
        // full chain on every bound word of nested packs.
        candidate.matches_parent_link().then_some(candidate)
    }""",
        p,
    )
    s = rep(
        s,
        """        if let Some(parent) = &self.parent {
            parent.matches_source(image, config)
                && parent.local.is_braced()
                && self.local.statement.config == parent.local.statement.config.nested()
                && parent.content_span().is_some_and(|span| {
                    span.start() == self.base
                        && image.bytes().get(span.as_range())
                            == Some(self.local.statement.image.bytes())
                })
        } else {
            self.base == 0
                && self.local.statement.image == self.root
                && self.local.statement.config == config
        }
    }
""",
        """        if let Some(parent) = &self.parent {
            parent.matches_source(image, config) && self.matches_parent_link()
        } else {
            self.base == 0
                && self.local.statement.image == self.root
                && self.local.statement.config == config
        }
    }

    /// The link to the immediate parent, against this word's own root image.
    /// A root word has no link and never satisfies it.
    fn matches_parent_link(&self) -> bool {
        let Some(parent) = &self.parent else {
            return false;
        };
        parent.local.is_braced()
            && self.local.statement.config == parent.local.statement.config.nested()
            && parent.content_span().is_some_and(|span| {
                span.start() == self.base
                    && self.root.bytes().get(span.as_range())
                        == Some(self.local.statement.image.bytes())
            })
    }
""",
        p,
    )
    s = rep(s, "absolute NativeWord or", "absolute `NativeWord` or", p)
    legacy = block("original_line_at.rs")
    body = legacy.split("{\n", 1)[1].rsplit("\n}", 1)[0]
    old = ""
    if zz:
        body = body.replace("    u32::try_from(", "    return u32::try_from(", 1)
        old = "    if tcl_lexer::zz_old() {\n" + "\n".join("    " + line for line in body.rstrip().splitlines()) + ";\n    }\n"
    replacement = "fn line_at(image: &SourceImage, offset: u32) -> Option<u32> {\n" + old + """    image.bytes().get(..usize::try_from(offset).ok()?)?;
    image.source_map().line_index().line_at(offset).checked_add(1)
}
"""
    s = rep(s, legacy, replacement, p)
    anchor = """mod tests {
    use super::*;
    use tcl_lexer::{LeadingBom, SourceSegmentationPurpose};
"""
    return rep(s, anchor, anchor + "\n" + block("ow_test.rs"), p)


@edit("rust/tcl-compiler/src/command_binding/original_name_value.rs")
def original_name_value(s, p):
    s = between(
        s,
        "    /// Unanimous genuine whole source word and its original parent head.",
        "#[cfg(test)]\nmod graph_tests {",
        block("onv_block.rs"),
        p,
    )
    return between(
        s,
        "    fn original_word_vector_config(",
        "    /// Same-vector source grammar from its original dispatch, compiler or",
        block("onv_config_block.rs"),
        p,
    )


@edit("rust/tcl-compiler/src/command_binding/formal_call_advice.rs")
def formal_call_advice(s, p):
    return between(
        s,
        "impl SourceCommandBindings {\n    /// Original callee formal operands used by registry-owned caller links.",
        "    /// Query a command-head formal through its original incoming/copy receipt,",
        block("fca_block.rs"),
        p,
    )


@edit("rust/tcl-compiler/src/command_binding/incomplete_body_metadata.rs")
def incomplete_body(s, p):
    toggle = (
        """    if tcl_lexer::zz_old() {
        return header_uncached(image, region, config).map(Arc::new);
    }
"""
        if zz
        else ""
    )
    s = rep(
        s,
        """/// The real parse cut and independently complete header words; no invented closer.
fn header(image: &SourceImage, region: Span, config: LexerConfig) -> Option<IncompleteHeader> {""",
        """/// [`header_uncached`], memoised per region of the most recent source image
/// and configuration. Each command's query walks every earlier declaration
/// site and asks for that site's header over the rest of the document, so
/// without the memo one analysis re-parses the document tail once per
/// (command, earlier site) pair. The result is a pure function of the image
/// bytes/channel, the region and the configuration. The key compares the
/// image by `SourceImage` equality (shared-allocation fast path, complete
/// byte comparison otherwise) and the memo retains the image clone, so its
/// bytes stay alive and no other image can alias them.
fn header(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
) -> Option<Arc<IncompleteHeader>> {
    type Memo = Option<(
        SourceImage,
        LexerConfig,
        std::collections::HashMap<(u32, u32), Option<Arc<IncompleteHeader>>>,
    )>;
    thread_local! {
        static MEMO: std::cell::RefCell<Memo> = const { std::cell::RefCell::new(None) };
    }
"""
        + toggle
        + """    let key = (region.start(), region.end());
    let current = |memo: &Memo| {
        memo.as_ref()
            .is_some_and(|(retained, retained_config, _)| {
                retained == image && *retained_config == config
            })
    };
    let retained = MEMO.with_borrow(|memo| {
        current(memo)
            .then(|| memo.as_ref().and_then(|(_, _, entries)| entries.get(&key).cloned()))
            .flatten()
    });
    if let Some(retained) = retained {
        return retained;
    }
    let prepared = header_uncached(image, region, config).map(Arc::new);
    MEMO.with_borrow_mut(|memo| {
        if !current(memo) {
            *memo = Some((image.clone(), config, std::collections::HashMap::new()));
        }
        if let Some((_, _, entries)) = memo.as_mut() {
            entries.insert(key, prepared.clone());
        }
    });
    prepared
}

/// The real parse cut and independently complete header words; no invented closer.
fn header_uncached(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
) -> Option<IncompleteHeader> {""",
        p,
    )

    s = rep(s, "_site: &CommandAllocationSite,", "site: &CommandAllocationSite,", p)
    s = rep(s, "_site.offset", "site.offset", p)
    return rep(s, "    if !target.prepended.is_empty() {",
               "    #[cfg(not(debug_assertions))]\n    let _ = site;\n    if !target.prepended.is_empty() {", p)


@edit("rust/tcl-compiler/src/realm.rs")
def realm(s, p):
    memo = "self.zz_memo()" if zz else "Some(&self.query_memo)"
    s = rep(
        s,
        """    source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache,
}""",
        """    source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache,
    query_memo: crate::command_binding::original_name_value::OriginalRealmQueryMemo,
}""",
        p,
    )
    s = rep(
        s,
        """            source_advice: Default::default(),
        };""",
        """            source_advice: Default::default(),
            query_memo: Default::default(),
        };""",
        p,
    )
    s = rep(
        s,
        """        source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache::default(),
    }""",
        """        source_advice: source_transition_advice::OriginalSourceTransitionAdviceCache::default(),
        query_memo: crate::command_binding::original_name_value::OriginalRealmQueryMemo::default(),
    }""",
        p,
    )
    s = rep(
        s,
        """        self.bindings
            .original_written_head_at_span_in_source(image, span, config)""",
        f"""        self.bindings.original_written_head_at_span_in_source_with(
            image,
            span,
            config,
            {memo},
        )""",
        p,
    )
    s = rep(
        s,
        """        let written = self
            .bindings
            .original_written_word_at_span_in_source(image, span, config);""",
        f"""        let written = self.bindings.original_written_word_at_span_in_source_with(
            image,
            span,
            config,
            {memo},
        );""",
        p,
    )
    zz_inv = (
        """        if self.zz_memo().is_none() {
            return self.bindings.invocation_at_source(head, offset);
        }
"""
        if zz
        else ""
    )
    s = rep(
        s,
        """    pub fn invocation_at_source(&self, head: &str, offset: u32) -> SourceInvocationBinding {
        self.bindings.invocation_at_source(head, offset)
    }""",
        """    pub fn invocation_at_source(&self, head: &str, offset: u32) -> SourceInvocationBinding {
"""
        + zz_inv
        + """        // The source query selects every dispatch point at `offset` whatever
        // the presentation head, so the realm's per-offset memo is the same
        // binding. Joining every activation's point is linear in the number
        // of activations; repeated queries must not repeat it.
        let _ = head;
        self.retained_original_invocation(offset).as_ref().clone()
    }""",
        p,
    )
    if zz:
        caller = """        let Some(memo) = self.zz_memo() else {
            return self.bindings.declaration_caller_alias_arguments(
                invocation,
                registry,
                self.bindings.source_metadata_owner(),
            );
        };
        self.bindings.declaration_caller_alias_arguments_with(
            invocation,
            registry,
            self.bindings.source_metadata_owner(),
            Some(memo),
        )
    }

    fn zz_memo(
        &self,
    ) -> Option<&crate::command_binding::original_name_value::OriginalRealmQueryMemo> {
        (!ZZ_NO_MEMO.load(std::sync::atomic::Ordering::Relaxed) && !tcl_lexer::zz_old())
            .then_some(&self.query_memo)
    }
"""
    else:
        caller = """        self.bindings.declaration_caller_alias_arguments_with(
            invocation,
            registry,
            self.bindings.source_metadata_owner(),
            Some(&self.query_memo),
        )
    }
"""
    s = rep(
        s,
        """    pub(crate) fn source_bindings_ref(&self) -> &SourceCommandBindings {""",
        """    /// Original callee caller-alias arguments for one invocation, sharing the
    /// invocation-independent callee survey across this realm's queries.
    pub(crate) fn declaration_caller_alias_arguments(
        &self,
        invocation: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<Vec<String>> {
"""
        + caller
        + """
    pub(crate) fn source_bindings_ref(&self) -> &SourceCommandBindings {""",
        p,
    )
    if not zz:
        s = rep(s, "            query_memo: Default::default(),",
                "            query_memo: crate::command_binding::original_name_value::OriginalRealmQueryMemo::default(),", p)
        inventory_doc = """    /// Borrow the retained inventory for syntax navigation in the same source
    /// root. Reconstructed/entered sources must select their own origin first.
"""
        s = rep(s, inventory_doc, "", p)
        s = rep(s, "    pub(crate) fn source_bindings_ref(&self) -> &SourceCommandBindings {",
                inventory_doc + "    pub(crate) fn source_bindings_ref(&self) -> &SourceCommandBindings {", p)
    if zz:
        s += """
/// ZZ perf probe toggle (experiment copy only).
pub static ZZ_NO_MEMO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
"""
    return s


@edit("rust/tcl-compiler/src/interprocedural.rs")
def interprocedural(s, p):
    s = rep(
        s,
        """        output.extend(
            bindings
                .declaration_caller_alias_arguments(
                    tokens,
                    registry,
                    bindings.source_metadata_owner(),
                )
                .unwrap_or_default(),
        );""",
        """        output.extend(
            identities
                .declaration_caller_alias_arguments(tokens, registry)
                .unwrap_or_default(),
        );""",
        p,
    )
    return rep(
        s,
        """                output.extend(
                    bindings
                        .declaration_caller_alias_arguments(
                            &tokens,
                            registry,
                            bindings.source_metadata_owner(),
                        )
                        .unwrap_or_default(),
                );""",
        """                output.extend(
                    identities
                        .declaration_caller_alias_arguments(&tokens, registry)
                        .unwrap_or_default(),
                );""",
        p,
    )


results = {}
for path, fn in EDITS.items():
    if only and path not in only:
        continue
    if skip and path in skip:
        continue
    full = os.path.join(root, path)
    with open(full) as f:
        before = f.read()
    results[full] = fn(before, path)
for full, text in results.items():
    with open(full, "w") as f:
        f.write(text)
    print("edited", full)
