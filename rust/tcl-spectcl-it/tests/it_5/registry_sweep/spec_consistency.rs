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

//! Cross-checks over every spec: synopses fit arity, replacements and aliases resolve, builtins are registered, traits and dialects nest.

use super::*;

/// Count the operand words a subcommand synopsis advertises after its
/// `command sub` head: `(required, optional)`, or `None` when the shape is
/// one this narrow reading declines to judge.
///
/// The gate below only wants the unambiguous case, so anything that cannot
/// be read as a flat list of operand words — an ellipsis, an option-shaped
/// word, a `?…?` group holding more than one word, a brace/bracket group —
/// declines. Those shapes legitimately disagree with a flat arity count.
fn synopsis_operand_counts(synopsis: &str, command: &str, sub: &str) -> Option<(u16, u16)> {
    let head = format!("{command} {sub} ");
    let tail = synopsis.strip_prefix(&head)?;
    let (mut required, mut optional) = (0u16, 0u16);
    let mut seen_optional = false;
    for word in tail.split_whitespace() {
        if word.contains("...")
            || word.contains('{')
            || word.contains('[')
            || word.contains('|')
            || word.starts_with('-')
            || word.starts_with("?-")
        {
            return None;
        }
        if let Some(inner) = word.strip_prefix('?').and_then(|w| w.strip_suffix('?')) {
            if inner.is_empty() || inner.contains('?') {
                return None;
            }
            seen_optional = true;
            optional += 1;
        } else {
            // A required word after an optional one is not a flat list.
            if seen_optional {
                return None;
            }
            required += 1;
        }
    }
    Some((required, optional))
}

/// A subcommand's synopsis is what hover and signature help show, so a
/// synopsis wider than the declared arity advertises a call the same spec
/// then reports as an arity error — `DIAMETER::header length ?value?` on an
/// `exact(0)` read-only field was the reported case (#2069). Nothing in the
/// compiler can see this drift: the two fields are independent data.
///
/// The check is deliberately narrow (see `synopsis_operand_counts`): it fires
/// only when a flat operand list cannot fit inside the declared bounds, in
/// either direction.
///
/// registry-metadata: both fields are registry data.
#[test]
fn sweep_subcommand_synopsis_fits_declared_arity() {
    crate::install_surfaces();
    let mut checked = 0usize;
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            for sub in spec.subcommands {
                if sub.synopsis.is_empty() {
                    continue;
                }
                // A versioned shape is judged against the union of its
                // windows, which the unversioned arity need not cover.
                if !sub.arity_windows.is_empty() {
                    continue;
                }
                let Some((required, optional)) =
                    synopsis_operand_counts(sub.synopsis, spec.name, sub.name)
                else {
                    continue;
                };
                checked += 1;
                assert!(
                    sub.arity.accepts(required),
                    "{dname}: `{}`'s synopsis `{}` shows {required} required operand(s), \
                     which its arity {:?} rejects",
                    sub.synopsis,
                    sub.synopsis,
                    sub.arity
                );
                let widest = required + optional;
                assert!(
                    sub.arity.accepts(widest),
                    "{dname}: `{}` advertises up to {widest} operand(s) but its arity {:?} \
                     rejects that count — hover offers a form the same spec reports as an \
                     arity error",
                    sub.synopsis,
                    sub.arity
                );
            }
        }
    }
    assert!(
        checked > 200,
        "the synopsis/arity gate unexpectedly shrank: {checked} subcommands checked"
    );
}

/// Every spec marking its deprecated replacement as a drop-in rename
/// (`deprecated_replacement_drop_in`) must (a) actually carry a
/// replacement, (b) name a single bare command word — no prose like
/// `"(removed)"`, no restructured multi-word form — and (c) resolve to a
/// registered command in the same dialect: the IRULE2002 head-swap quick
/// fix rewrites the call head to this name verbatim, so a stale or
/// unresolvable replacement would auto-fix code into an unknown command.
///
/// registry-metadata: the drop-in classification is registry data.
#[test]
fn sweep_deprecated_drop_in_replacements_resolve() {
    crate::install_surfaces();
    let mut drop_ins = 0usize;
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            if !spec.deprecated_replacement_drop_in {
                continue;
            }
            drop_ins += 1;
            let replacement = spec
                .deprecated_replacement
                .unwrap_or_else(|| panic!("{dname}/{name}: drop-in flag without a replacement"));
            assert!(
                !replacement.contains(char::is_whitespace)
                    && !replacement.contains('(')
                    && !replacement.contains('/'),
                "{dname}/{name}: drop-in replacement {replacement:?} is not a single command word"
            );
            assert!(
                reg.get(replacement).is_some(),
                "{dname}/{name}: drop-in replacement {replacement:?} is not a registered command"
            );
        }
    }
    // Sanity floor: the iRules pack stamps the 4.x-compat rename family.
    assert!(
        drop_ins >= 20,
        "expected the iRules drop-in rename family, got {drop_ins}"
    );
}

/// Every spec's `alias_of`, when set, must name a command that actually
/// resolves *in the same registry* — the identity the loader's stamp
/// rejection rule trusts and codegen records at a specialised site. No
/// shipped spec declares one (`alias_of` is a pack command's field), so
/// the live half sweeps forward-looking; the synthetic half pins what the
/// live half would catch — an unknown target, and a target real only in
/// another dialect's family.
///
/// registry-metadata: `alias_of` is registry data with no codegen effect
/// yet.
#[test]
fn alias_of_names_a_shipped_command_of_the_same_family() {
    crate::install_surfaces();
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            let Some(target) = spec.alias_of else {
                continue;
            };
            assert!(
                reg.get(target).is_some(),
                "{dname}/{name}: alias_of {target:?} is not a registered command"
            );
        }
    }

    // Positive: a pack command naming a real shipped command resolves —
    // proving the live sweep's own `reg.get(target).is_some()` assertion
    // is meaningful, not vacuously true.
    let mut with_real_target = CommandRegistry::build_default();
    with_real_target.insert(tcl_registry::CommandSpec {
        name: "vendor::unpack",
        alias_of: Some("lsort"),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let real_target = with_real_target
        .get("vendor::unpack")
        .and_then(|spec| spec.alias_of)
        .expect("the fixture just declared it");
    assert!(
        with_real_target.get(real_target).is_some(),
        "lsort must be a real shipped command for this fixture to prove anything"
    );

    // Negative: an unknown target does not resolve — what the live sweep
    // above would fail on if a real spec ever declared one.
    let mut with_unknown_target = CommandRegistry::build_default();
    with_unknown_target.insert(tcl_registry::CommandSpec {
        name: "vendor::unpack",
        alias_of: Some("not_a_real_command_zzz"),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let unknown_target = with_unknown_target
        .get("vendor::unpack")
        .and_then(|spec| spec.alias_of)
        .expect("the fixture just declared it");
    assert!(
        with_unknown_target.get(unknown_target).is_none(),
        "an unknown alias_of target must not silently resolve"
    );

    // Negative: "of the same family" — a command real only in another
    // dialect does not resolve in the plain Tcl registry.
    let plain = CommandRegistry::build_default();
    assert!(
        plain.get("HTTP::header").is_none(),
        "HTTP::header is an f5-irules command; it must not resolve in the plain Tcl family"
    );
}

/// The core Tcl commands the runtime must back, as the WASM command-parity
/// gate (`rust/xtask-backing/src/command_backing.rs`) defines them: the Tcl family's
/// built-in specs (no `required_package`) available at Tcl 9.0 or later.
fn is_core_command(spec: &tcl_registry::CommandSpec) -> bool {
    spec.required_package.is_none()
        && spec
            .surface
            .is_none_or(|rows| tcl_dialect::model::surface_admits_from(rows, Family::Tcl, "9.0"))
}

/// The bare operator spelling of a `tcl::mathop` command (`+`, `<<`): grammar
/// read inside `expr`, not a command a runtime dispatches.
fn is_bare_operator_head(spec: &tcl_registry::CommandSpec) -> bool {
    spec.traits.contains(tcl_registry::Traits::OPERATOR_COMMAND) && !spec.name.contains("::")
}

/// A command outside the core set claims nothing it does not have. The WASM
/// command-parity gate asks the runtime about core commands only, so a row
/// outside that set declares no backing, or the one its command's core row
/// declares (one name is one command, as the gate holds it), or else the
/// shipped builtin of its own name: the builtin the runtime registers in the
/// release or Jim mode the row describes (Jim's `exists`, C 8.6's
/// `::tcl::clock::getenv`). The iRules test harness reads the backing of
/// `catch`'s Tcl 8.4/iRules row to keep the real `catch` unmocked.
///
/// registry-metadata: `runtime_backing` is registry data.
#[test]
fn a_spec_outside_the_core_set_declares_only_its_own_builtin() {
    crate::install_surfaces();
    use tcl_registry::RuntimeBacking;

    let specs = tcl_registry::installed().specs_of("tcl");
    let core: std::collections::BTreeMap<&str, RuntimeBacking> = specs
        .iter()
        // The bare operator spelling (`+`) is `expr` grammar that nothing
        // dispatches as a command, so it says nothing about the backing of a
        // real command spelled the same: Jim's `+ - * /` (jim.c) are builtins.
        .filter(|spec| is_core_command(spec) && !is_bare_operator_head(spec))
        .map(|spec| (spec.name, spec.runtime_backing))
        .collect();
    let mut outside = 0usize;
    let declared: Vec<_> = specs
        .iter()
        .filter(|spec| !is_core_command(spec) && !spec.runtime_backing.is_none())
        .inspect(|_| outside += 1)
        .filter(|spec| {
            core.get(spec.name).map_or_else(
                || spec.runtime_backing != RuntimeBacking::shipped(spec.name),
                |backing| *backing != spec.runtime_backing,
            )
        })
        .map(|spec| {
            format!(
                "{} {:?} {:?}",
                spec.name, spec.surface, spec.runtime_backing
            )
        })
        .collect();
    assert!(
        declared.is_empty(),
        "commands outside the core set must keep their backing `None` or their own builtin's: \
         {declared:#?}"
    );
    assert!(
        outside > 0,
        "the release and Jim rows of core commands are checked"
    );
}

/// Shipped identities name registered commands. A method-context spelling
/// can refer to a qualified helper; Tcl bodies name embedded library files.
///
/// registry-metadata: `runtime_backing` is registry data.
#[test]
fn a_shipped_builtin_names_its_registered_implementation() {
    crate::install_surfaces();
    use tcl_registry::{BodySource, RuntimeBacking};

    let registry = CommandRegistry::build_default();
    let mut shipped = 0;
    for spec in tcl_registry::installed().specs_of("tcl") {
        match spec.runtime_backing {
            RuntimeBacking::ShippedBuiltin { identity } => {
                let registered =
                    tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
                let name = tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name);
                if registered != name {
                    assert!(
                        spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT),
                        "{}",
                        spec.name
                    );
                    let helper = registry
                        .get(identity)
                        .expect("registered method helper spec");
                    assert_eq!(helper.name, registered, "{}", spec.name);
                    assert!(matches!(
                        helper.runtime_backing,
                        RuntimeBacking::ShippedBuiltin { .. }
                    ));
                }
                shipped += 1;
            }
            RuntimeBacking::TclBody {
                source: BodySource::PackageSource { relative_path },
                ..
            } => assert!(
                ["init.tcl", "package.tcl", "parray.tcl"].contains(&relative_path),
                "{}: {relative_path:?} is not a library file the runtime embeds",
                spec.name
            ),
            _ => {}
        }
    }
    assert!(shipped > 300, "the core commands declare {shipped}");
}

/// Sweep the whole trait lattice through `commands_with_trait` in a few
/// dialects, asserting the membership listing is self-consistent with each
/// command's own `traits.contains`.
///
/// registry-metadata: trait classification is registry data.
#[test]
fn sweep_trait_membership_is_self_consistent() {
    crate::install_surfaces();
    for &dname in &["tcl8.6", "tcl9.0", "f5-irules"] {
        let reg = registry_for_dialect(dname);
        for &t in ALL_TRAITS {
            let members = reg.commands_with_trait(t);
            // No duplicates in the membership listing.
            let unique: BTreeSet<&str> = members.iter().copied().collect();
            assert_eq!(
                unique.len(),
                members.len(),
                "{dname}: commands_with_trait({t:?}) has duplicates"
            );
            // Every listed member's *preferred* spec actually carries the trait.
            for m in &members {
                let spec = reg
                    .get(m)
                    .unwrap_or_else(|| panic!("{dname}: trait member {m:?} does not get"));
                assert!(
                    spec.traits.contains(t),
                    "{dname}: {m:?} listed for {t:?} but its spec lacks it"
                );
            }
        }
    }
}

/// `get_for_surface` is consistent with `supports_dialect` and with the
/// command's declared `dialects` set, swept over every command in the Tcl
/// dialects. This exercises the dialect-filtered resolution path on every spec.
///
/// registry-metadata (dialect membership is registry data; the *introduction
/// versions* of `dict`/`try`/etc. are tclsh history, covered in the sibling
/// port).
#[test]
fn sweep_dialect_resolution_is_consistent() {
    crate::install_surfaces();
    for (dname, bit) in [
        ("tcl8.4", Some(SurfaceQuery::core(Family::Tcl, "8.4"))),
        ("tcl8.5", Some(SurfaceQuery::core(Family::Tcl, "8.5"))),
        ("tcl8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        ("tcl9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
    ] {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let for_dialect = reg.get_for_surface(name, bit);
            if let Some(spec) = for_dialect {
                // A spec returned for `bit` must support `bit`.
                assert!(
                    spec.supports_dialect(bit),
                    "{dname}: get_for_surface({name:?}) returned a spec that rejects the dialect"
                );
            }
            // If *every* registered spec under this name rejects the dialect,
            // get_for_surface must return None (and vice-versa). Use
            // `spec_visible`, not the bare `supports_dialect` mask test:
            // `get_for_surface` also applies the profile's subtractive
            // disable list and (for a profile whose operators are not
            // command heads, e.g. tcl8.4/f5-irules) the
            // `Traits::OPERATOR_COMMAND` exclusion, so a naive mask-only
            // scan disagrees with it for exactly those specs.
            let any_supports = reg.specs(name).iter().any(|s| reg.spec_visible(s, bit));
            assert_eq!(
                for_dialect.is_some(),
                any_supports,
                "{dname}: get_for_surface/{name:?} disagrees with specs() support scan"
            );
        }
    }
}

/// Assert `child`'s dialect declaration is never *reachable* under a dialect
/// where its `parent` isn't. This is the reachability nesting rule, not a raw
/// bitwise subset: under the "universal command = `ALL_TCL`, vendor
/// availability via mask *intersection*" model, a sub-element may explicitly
/// list a vendor bit its `ALL_TCL` parent lacks and still be valid — e.g.
/// `variable`'s `EXPECT`-tagged form is reachable only under expect's
/// `TCL86 | EXPECT` mask, which the `ALL_TCL` parent also reaches via `TCL86`.
/// The invariant it guards is the real one: a child must never resolve under
/// a dialect profile its parent command cannot.
///
/// `None` on either side is trivially valid (`None` child = inherit; `None`
/// parent = universal).
fn assert_dialects_nest(
    parent: Option<&'static [SpecSurface]>,
    child: Option<&'static [SpecSurface]>,
    context: &str,
) {
    let (Some(child), Some(parent)) = (child, parent) else {
        return;
    };
    for profile in tcl_dialect::DialectProfile::all() {
        let mask = Some(profile.surface_query());
        assert!(
            !surface_admits(child, mask.as_ref()) || surface_admits(parent, mask.as_ref()),
            "{context}: child dialects {child:?} resolve under {} (mask \
             {mask:?}) but the parent's {parent:?} do not — the child would be \
             reachable where its parent command is not",
            profile.name
        );
    }
}

/// Dialect-nesting checks for one `CommandForm`/`SubCommandForm` and its
/// own options, relative to its `effective_parent` (already resolved by
/// the caller). Shared between the command-level and subcommand-level
/// callers since the two form kinds are the same type.
fn check_form_dialect_nesting(
    ctx: &str,
    effective_parent: Option<&'static [SpecSurface]>,
    form: &tcl_registry::forms::CommandForm,
) {
    assert_dialects_nest(
        effective_parent,
        form.surface,
        &format!("{ctx} form {}", form.name),
    );
    let effective = form.surface.or(effective_parent);
    for o in form.options {
        assert_dialects_nest(
            effective,
            o.surface,
            &format!("{ctx} form {} option {}", form.name, o.name),
        );
    }
}

/// Dialect-nesting checks for one `SubCommand`: itself against the parent
/// `CommandSpec`'s dialects, then its options / side effects / forms /
/// sub-subcommands against *its own* effective dialects.
fn check_subcommand_dialect_nesting(
    cmd_ctx: &str,
    parent: Option<&'static [SpecSurface]>,
    sub: &tcl_registry::SubCommand,
) {
    let ctx = format!("{cmd_ctx} {}", sub.name);
    assert_dialects_nest(parent, sub.surface, &ctx);
    let effective = sub.surface.or(parent);
    for o in sub.options {
        assert_dialects_nest(effective, o.surface, &format!("{ctx} option {}", o.name));
    }
    for se in sub.side_effects.lower_bound() {
        assert_dialects_nest(
            effective,
            se.surface,
            &format!("{ctx} side_effect {:?}", se.target),
        );
    }
    for scf in sub.subcommand_forms {
        check_form_dialect_nesting(&ctx, effective, scf);
    }
    for ssc in sub.sub_subcommands {
        assert_dialects_nest(effective, ssc.surface, &format!("{ctx} {}", ssc.name));
    }
}

/// Dialect-nesting checks for everything hung directly off one
/// `CommandSpec`: its flat options, side effects, forms, structured
/// command forms, and subcommands (recursing into the latter).
fn check_command_dialect_nesting(dname: &str, name: &str, spec: &tcl_registry::CommandSpec) {
    let ctx = format!("{dname}/{name}");
    for opt in spec.options {
        assert_dialects_nest(
            spec.surface,
            opt.surface,
            &format!("{ctx} option {}", opt.name),
        );
    }
    for se in spec.side_effects.lower_bound() {
        assert_dialects_nest(
            spec.surface,
            se.surface,
            &format!("{ctx} side_effect {:?}", se.target),
        );
    }
    for f in spec.forms {
        assert_dialects_nest(
            spec.surface,
            f.surface,
            &format!("{ctx} form {}", f.synopsis),
        );
    }
    for cf in spec.command_forms {
        check_form_dialect_nesting(&ctx, spec.surface, cf);
    }
    for sub in spec.subcommands {
        check_subcommand_dialect_nesting(&ctx, spec.surface, sub);
    }
}

/// Every nested `dialects` declaration (`OptionSpec`, `SideEffect`,
/// `FormSpec`, `CommandForm`/`SubCommandForm`, `SubCommand`,
/// `SubSubCommand`) must be a subset of its governing parent's *effective*
/// dialects — a child claiming a dialect its parent lacks can never
/// actually be reached, since the parent itself is never selected outside
/// its own dialects, so the child would silently dead-code instead of
/// failing loudly. This is the whole-registry backstop for the
/// `nested surfaces nest` invariant: a single command file
/// *can* self-check at compile time with a `const { assert!(...) }` (see
/// `tcl-dialect`'s `dialect_set` module for the pattern), but nothing
/// forces every command file to opt in — this sweep covers all of them,
/// present and future, uniformly.
///
/// registry-metadata: dialect membership is registry data; this is a
/// self-consistency invariant, not a C-Tcl fact.
#[test]
fn sweep_nested_dialects_are_subsets_of_parent() {
    crate::install_surfaces();
    let mut checked = 0usize;
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else {
                continue;
            };
            checked += 1;
            check_command_dialect_nesting(dname, name, spec);
        }
    }
    assert!(checked > 1000, "sweep unexpectedly small: {checked} specs");
}

// Dialect-catalogue sweep (the name surface this crate owns).

/// Every catalogued dialect name parses-or-is-config-only, and each loadable
/// dialect's cached registry round-trips a core lookup. Touches the cache /
/// parse / available-dialects surface.
///
/// registry-metadata: dialect vocabulary.
#[test]
fn sweep_dialect_catalogue() {
    crate::install_surfaces();
    for d in tcl_dialect::DialectProfile::all()
        .iter()
        .map(|profile| profile.name)
    {
        // Every cached dialect registry has the Tcl core.
        let reg = registry_for_dialect(d);
        assert!(!reg.is_empty(), "{d}: empty registry");
        assert!(reg.get("set").is_some(), "{d}: missing core `set`");
        // The name either names a catalogue profile, or is a config-only
        // name that collapses to plain Tcl (still a usable registry).
        let _ =
            tcl_dialect::DialectProfile::find(d).map(tcl_dialect::DialectProfile::surface_query);
    }
    // An unparseable dialect collapses to a plain-Tcl registry.
    let junk = registry_for_dialect("definitely-not-a-dialect");
    assert!(junk.get("set").is_some());
    assert!(
        junk.get("HTTP::header").is_none(),
        "junk fallback is plain Tcl"
    );
}
