// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `SslicTcl` vocabulary is stated twice, and this holds the two
//! statements to each other.
//!
//! [`tcl_sslictcl::vocabulary::DECLARATIONS`] is the contract: what the loader
//! reads. `tcl_registry::commands::sslictcl` is the same vocabulary as command
//! specs and definition-body grammars, which is what gives an editor
//! completion, hover, signature help, semantic tokens, folding, and document
//! symbols. A word added to one and not the other is invisible in the other
//! half, and that is exactly what this gate catches — in both directions.
//!
//! registry-metadata: every assertion reads the loader's own table and
//! parsers, or registry data. `SslicTcl` is our own DSL, so the table is its
//! oracle; nothing here consults C Tcl.

use std::collections::BTreeSet;
use std::str::FromStr;

use tcl_registry::ArgRole;
use tcl_registry::definer::DefinerFamily;
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::registry::CommandRegistry;
use tcl_registry::spec::CommandSpec;

use tcl_sslictcl::estimate::{EstimateSeverity, Grade};
use tcl_sslictcl::model::TlsStatus;
use tcl_sslictcl::trust::ClientFamily;
use tcl_sslictcl::vocabulary::{DECLARATIONS, Declaration, ValueDomain};

/// Every declaration the vocabulary reaches, top-level and nested, in a stable
/// order. Nested declarations are only reachable through `Member::nested`, so
/// the walk is what makes the gate total.
fn all_declarations() -> Vec<&'static Declaration> {
    fn walk(declaration: &'static Declaration, out: &mut Vec<&'static Declaration>) {
        if out.iter().any(|seen| seen.name == declaration.name) {
            return;
        }
        out.push(declaration);
        for member in declaration.members {
            if let Some(nested) = member.nested {
                walk(nested, out);
            }
        }
    }
    let mut out = Vec::new();
    for declaration in DECLARATIONS {
        walk(declaration, &mut out);
    }
    out
}

fn pack() -> std::sync::Arc<CommandRegistry> {
    static_context_for("sslictcl").commands().clone()
}

fn spec_named<'a>(registry: &'a CommandRegistry, name: &str) -> &'a CommandSpec {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("`{name}` is in the loader's vocabulary but not in the pack"))
}

/// Every word the vocabulary declares — declarations and members alike — has a
/// spec in the pack, and every spec in the pack is named by the vocabulary.
/// Neither half may carry a word the other has never heard of.
#[test]
fn the_pack_and_the_vocabulary_name_the_same_words() {
    let registry = pack();
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for declaration in all_declarations() {
        declared.insert(declaration.name);
        for member in declaration.members {
            declared.insert(member.name);
        }
    }
    // Every declared word has a spec, and its keyword-ness comes from a
    // grammar rather than a global trait: the token walker honours
    // `LANGUAGE_KEYWORD` wherever a head appears, which would paint a
    // misplaced member row as valid and defeat the context sensitivity the
    // grammars exist for.
    for name in &declared {
        let spec = spec_named(&registry, name);
        assert!(
            !spec.traits.contains(tcl_registry::Traits::LANGUAGE_KEYWORD),
            "`{name}` must be painted by its enclosing grammar, not a global trait"
        );
    }
    // …so every declared word must be a member of some grammar, and the nine
    // top-level declarations are members of the *document* grammar.
    let document = registry
        .document_grammar()
        .expect("the sslictcl registry declares a document grammar");
    let top_level: Vec<&str> = DECLARATIONS.iter().map(|entry| entry.name).collect();
    assert_eq!(
        document
            .members
            .iter()
            .map(|member| member.keyword)
            .collect::<Vec<_>>(),
        top_level,
        "the document grammar is exactly the table's top-level declarations"
    );
    let packed: BTreeSet<&str> = tcl_registry::commands::sslictcl::sslictcl_command_specs()
        .iter()
        .map(|spec| spec.name)
        .collect();
    assert_eq!(
        packed, declared,
        "the pack and `vocabulary::DECLARATIONS` must name exactly the same words"
    );
}

/// A declaration with a body carries a definition-body grammar whose members
/// are **exactly** its member names — no extra, none missing — so the shared
/// walker offers and paints the same vocabulary the loader accepts. A
/// declaration without a body carries no grammar.
#[test]
fn every_body_grammar_is_exactly_its_declarations_members() {
    let registry = pack();
    for declaration in all_declarations() {
        let spec = spec_named(&registry, declaration.name);
        if !declaration.body {
            assert!(
                spec.definition_body.is_none(),
                "`{}` takes no body, so it must carry no grammar",
                declaration.name
            );
            continue;
        }
        let grammar = spec.definition_body.unwrap_or_else(|| {
            panic!(
                "`{}` takes a body, so its spec must carry that body's grammar",
                declaration.name
            )
        });
        assert_eq!(
            grammar.family,
            DefinerFamily::SslicTcl,
            "{}",
            declaration.name
        );
        let declared: Vec<&str> = declaration.members.iter().map(|m| m.name).collect();
        let grammar_members: Vec<&str> = grammar.members.iter().map(|m| m.keyword).collect();
        assert_eq!(
            grammar_members, declared,
            "`{}`'s grammar members must be exactly its vocabulary members, in order",
            declaration.name
        );
        for member in declaration.members {
            assert!(
                grammar.is_member(member.name),
                "`{}`'s grammar must know `{}`",
                declaration.name,
                member.name
            );
        }
    }
}

/// A declaration's key word and body claim the roles a consumer reads to find
/// them, so document symbols and folding land on the right words.
///
/// A key word claims `ArgRole::Name` exactly when the declaration has a body:
/// that is what makes it the name of a declared entity another declaration can
/// refer to. The one bodiless declaration is the `sslictcl VERSION` header,
/// whose key declares nothing and is the table's only `Int` key — painting a
/// version number as a declaration name would put it in the document outline.
#[test]
fn key_and_body_words_claim_their_roles() {
    let registry = pack();
    for declaration in all_declarations() {
        let name = declaration.name;
        let names_an_entity = declaration.key.is_some() && declaration.body;
        assert_eq!(
            declaration.key == Some(ValueDomain::Int),
            !declaration.body,
            "{name}: the bodiless header is the table's only Int key"
        );
        let mut call: Vec<&str> = Vec::new();
        if declaration.key.is_some() {
            call.push("a-key");
        }
        if declaration.body {
            call.push("{ }");
        }
        let body_index = call.len().saturating_sub(1);
        assert_eq!(
            registry.arg_indices_for_role(name, &call, ArgRole::Name),
            if names_an_entity { vec![0] } else { vec![] },
            "{name}: the key word"
        );
        assert_eq!(
            registry.arg_indices_for_role(name, &call, ArgRole::Body),
            if declaration.body {
                vec![body_index]
            } else {
                vec![]
            },
            "{name}: the body word"
        );
    }
}

/// A `SCRIPT` member is retained verbatim and never evaluated, so its spec
/// carries [`ArgRole::OpaqueScript`] — script-shaped data that folds like a
/// body and that nothing analyses — and **no** grammar, so the definition-body
/// walker drops out of declaration context for it too.
#[test]
fn script_members_are_opaque_not_bodies() {
    let registry = pack();
    let mut seen = 0usize;
    for declaration in all_declarations() {
        for member in declaration.members {
            if member.value != ValueDomain::Script {
                continue;
            }
            seen += 1;
            let spec = spec_named(&registry, member.name);
            assert!(
                spec.definition_body.is_none(),
                "`{}` is a retained script, not a declaration block",
                member.name
            );
            assert_eq!(
                spec.arg_role_at(0),
                Some(ArgRole::OpaqueScript),
                "`{}` is retained data, not an analysed body",
                member.name
            );
            assert!(
                !ArgRole::OpaqueScript.carries_script(),
                "nothing may descend into a retained predicate"
            );
            assert!(
                ArgRole::OpaqueScript.folds_as_block(),
                "a reader still collapses it"
            );
        }
    }
    assert_eq!(seen, 1, "vocabulary 1 has exactly one SCRIPT member");
}

/// A member's declared domain fixes its arity: every member of vocabulary 1
/// takes exactly one word, and a nested block member is the nested
/// declaration's own statement.
#[test]
fn member_arities_follow_their_domains() {
    let registry = pack();
    for declaration in all_declarations() {
        for member in declaration.members {
            let spec = spec_named(&registry, member.name);
            if member.value == ValueDomain::Block {
                let nested = member.nested.expect("a Block member names its declaration");
                assert_eq!(nested.name, member.name);
                continue;
            }
            assert!(
                spec.arity.accepts(1),
                "`{}` takes one value word",
                member.name
            );
        }
    }
    // The header takes its version word and nothing else.
    let header = spec_named(&registry, "sslictcl");
    assert!(header.arity.accepts(1) && !header.arity.accepts(2));
}

/// The pack's offered spellings for each enumerated domain are exactly the
/// loader's own canonical spellings, checked through the loader's parsers
/// rather than a second hand-written list.
#[test]
fn enumerated_domains_offer_the_loaders_canonical_spellings() {
    let registry = pack();
    for declaration in all_declarations() {
        for member in declaration.members {
            let spec = spec_named(&registry, member.name);
            let offered: Vec<&str> = spec.arg_values_at(0).iter().map(|v| v.value).collect();
            match member.value {
                ValueDomain::Bool => {
                    assert_eq!(
                        offered,
                        ["true", "false", "yes", "no", "on", "off", "1", "0"],
                        "`{}` offers the loader's boolean spellings",
                        member.name
                    );
                }
                // Each of the three named enums round-trips: the pack's
                // spelling parses, and the parsed value spells itself the same
                // way, so a spelling the loader would canonicalise differently
                // (`open-jdk` for `openjdk`) cannot creep into completion.
                ValueDomain::Client => {
                    assert_canonical(&offered, member.name, |word| {
                        ClientFamily::from_str(word).ok().map(ClientFamily::as_str)
                    });
                    assert_eq!(offered.len(), 6, "`{}`: six root programs", member.name);
                }
                ValueDomain::Status => {
                    assert_canonical(&offered, member.name, |word| {
                        TlsStatus::from_str(word).ok().map(TlsStatus::as_str)
                    });
                    assert_eq!(offered.len(), 4, "`{}`: four statuses", member.name);
                }
                ValueDomain::Severity => {
                    assert_canonical(&offered, member.name, |word| {
                        EstimateSeverity::from_str(word)
                            .ok()
                            .map(EstimateSeverity::as_str)
                    });
                    assert_eq!(offered.len(), 4, "`{}`: four severities", member.name);
                }
                // `Grade` has no `as_str`, so it is pinned by rank: every
                // offered spelling parses, the ranks are distinct and
                // descending, and they span the whole declarable range.
                ValueDomain::Grade => {
                    let ranks: Vec<u8> = offered
                        .iter()
                        .map(|word| {
                            Grade::from_str(word)
                                .unwrap_or_else(|_| {
                                    panic!("`{}` offers undeclarable grade `{word}`", member.name)
                                })
                                .rank()
                        })
                        .collect();
                    assert_eq!(ranks, [7, 6, 5, 4, 3, 2, 1], "`{}`: A+ … F", member.name);
                    for outcome in ["T", "M", "?"] {
                        assert!(
                            Grade::from_str(outcome).is_err(),
                            "`{outcome}` is an estimator outcome, not a declarable floor"
                        );
                    }
                }
                ValueDomain::Literal(word) => {
                    assert_eq!(offered, [word], "`{}` offers its literal", member.name);
                    assert_eq!(
                        spec.closed_value_args,
                        &[0],
                        "`{}` is an exact-match literal, so it is the one closed domain",
                        member.name
                    );
                }
                _ => {}
            }
            // Only an exact-match literal domain may be closed: every other
            // enumerated domain is matched case-insensitively by the loader,
            // and `closed_value_args` is an exact-match check, so closing one
            // would report a value the loader accepts as invalid (W127).
            if !matches!(member.value, ValueDomain::Literal(_)) {
                assert!(
                    spec.closed_value_args.is_empty(),
                    "`{}`'s domain is wider than its canonical spellings",
                    member.name
                );
            }
        }
    }
}

/// Every offered spelling is the one the loader canonicalises it to.
fn assert_canonical(
    offered: &[&str],
    member: &str,
    canonical: impl Fn(&str) -> Option<&'static str>,
) {
    for word in offered {
        let parsed = canonical(word)
            .unwrap_or_else(|| panic!("`{member}` offers `{word}`, which the loader rejects"));
        assert_eq!(
            parsed, *word,
            "`{member}` offers `{word}`, which the loader spells `{parsed}`"
        );
    }
    let distinct: BTreeSet<&str> = offered.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        offered.len(),
        "`{member}` offers a duplicate"
    );
}

/// The `VERSION` domain is the deliberate open one: the pack offers the
/// canonical spellings, the loader also accepts documented aliases, and so the
/// argument must not be closed.
#[test]
fn the_version_domain_is_offered_but_not_closed() {
    let registry = pack();
    for word in ["protocol", "protocols"] {
        let spec = spec_named(&registry, word);
        let offered: Vec<&str> = spec.arg_values_at(0).iter().map(|v| v.value).collect();
        assert_eq!(
            offered,
            ["ssl2", "ssl3", "tls1.0", "tls1.1", "tls1.2", "tls1.3"],
            "{word}"
        );
        assert!(spec.closed_value_args.is_empty(), "{word}");
    }
    // The `protocol` declaration's key really is that domain, so the spelling
    // set above is the one an editor offers for its key word.
    let protocol = DECLARATIONS
        .iter()
        .find(|entry| entry.name == "protocol")
        .expect("the vocabulary declares `protocol`");
    assert_eq!(protocol.key, Some(ValueDomain::Version));
}

/// The vocabulary's openness is a loader rule about unknown *members*, so the
/// registry states the member set identically for an open and a closed block.
/// This pins that reading: nothing in the pack varies with `Declaration::open`.
#[test]
fn openness_does_not_change_the_declared_member_set() {
    let registry = pack();
    let (open, closed): (Vec<&'static Declaration>, Vec<&'static Declaration>) = all_declarations()
        .into_iter()
        .filter(|declaration| declaration.body)
        .partition(|declaration| declaration.open);
    assert!(!open.is_empty() && !closed.is_empty(), "both kinds exist");
    for declaration in open.iter().chain(closed.iter()) {
        let grammar = spec_named(&registry, declaration.name)
            .definition_body
            .expect("a body carries its grammar");
        assert_eq!(
            grammar.members.len(),
            declaration.members.len(),
            "`{}`: the pack declares the member set, not the openness",
            declaration.name
        );
    }
}
