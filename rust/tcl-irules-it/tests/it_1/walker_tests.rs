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

#[cfg(test)]
mod tests {
    use tcl_irules::*;

    /// Extract object references from `source` against the profile-stamped
    /// iRules registry (`pool` / `snatpool` / `class` are dialect commands).
    fn refs(source: &str) -> Vec<IrulesObjectReference> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(
            tcl_dialect::DialectProfile::irules(),
        )
        .commands();
        extract_irules_object_references(source, None, registry)
    }

    #[test]
    fn object_refs_follow_only_reachable_valid_execution_regions() {
        crate::support::install_surfaces();
        let source = concat!(
            "pool /Common/top\n",
            "proc dormant {} { pool /Common/dormant }\n",
            "proc leaf {} { pool /Common/leaf }\n",
            "proc malformed {} { pool /Common/malformed } extra\n",
            "when BOGUS_EVENT { pool /Common/unknown }\n",
            "when HTTP_REQUEST { call leaf; pool /Common/live }\n",
        );
        let names: Vec<_> = refs(source)
            .into_iter()
            .map(|reference| reference.name)
            .collect();
        assert_eq!(names, ["/Common/leaf", "/Common/live"]);
    }

    #[test]
    fn extracts_pool_snatpool_and_datagroup_refs() {
        crate::support::install_surfaces();
        let source = "\n\
            when HTTP_REQUEST {\n\
            \x20   if [class match -- [HTTP::host] equals /Common/host_dg] {\n\
            \x20       snatpool /Common/sp1\n\
            \x20       pool /Common/web_pool\n\
            \x20   }\n\
            }\n";
        let by_name: Vec<(String, String)> = refs(source)
            .iter()
            .map(|r| (r.command.clone(), r.name.clone()))
            .collect();
        assert!(by_name.contains(&("class".to_owned(), "/Common/host_dg".to_owned())));
        assert!(by_name.contains(&("snatpool".to_owned(), "/Common/sp1".to_owned())));
        assert!(by_name.contains(&("pool".to_owned(), "/Common/web_pool".to_owned())));
    }

    /// Return readonly candidates; configuration object selection is independent.
    fn ref_names(source: &str) -> Vec<String> {
        refs(source).into_iter().map(|r| r.name).collect()
    }

    /// A `when` body invoking `caller /Common/web_pool`, after `prelude`.
    fn pool_rule(prelude: &str, caller: &str) -> String {
        format!("{prelude}when HTTP_REQUEST {{\n    {caller} /Common/web_pool\n}}\n")
    }

    #[test]
    fn unavailable_irules_alias_does_not_create_object_ref_command() {
        crate::support::install_surfaces();
        // F5 K36322151 disables `interp`, so this cannot create `p`.
        assert_eq!(
            ref_names(&pool_rule("interp alias {} p {} pool\n", "p")),
            [] as [String; 0]
        );
        assert_eq!(ref_names(&pool_rule("set y 1\n", "p")), [] as [String; 0]);
        assert_eq!(
            ref_names(&pool_rule("interp alias {} p {} pool\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
            "a failed alias cannot take the real pool command away"
        );
    }

    #[test]
    fn nominal_alias_alternatives_preserve_only_the_same_original_object_projection() {
        crate::support::install_surfaces();
        for prefix in ["", " -list"] {
            let source = format!(
                "interp alias {{}} members {{}} active_members{prefix}\nwhen HTTP_REQUEST {{set n [members /Common/active]}}"
            );
            assert_eq!(ref_names(&source), ["/Common/active"], "{source}");
        }
        for prelude in [
            // Equal bytes captured at the alias are a different original word.
            "interp alias {} members {} pool /Common/active",
            "interp alias {} members {} active_members $dynamic",
            "interp alias {} members {} members",
            "interp alias {} active_members {} members; interp alias {} members {} active_members",
            "proc active_members args {}; interp alias {} members {} active_members",
            "interp alias {} members {} unknown_handler",
            "interp alias {} members {} active_members; interp alias {} members {} puts",
        ] {
            let source = format!("{prelude}\nwhen HTTP_REQUEST {{set n [members /Common/active]}}");
            assert!(ref_names(&source).is_empty(), "{source}");
        }
    }

    #[test]
    fn categories_follow_registry_returned_object_kinds() {
        crate::support::install_surfaces();
        let source = concat!(
            "when HTTP_REQUEST {\n",
            " set n [::active_members /Common/active]\n",
            " LB::reselect pool /Common/fallback\n",
            " set h [HSL::open -proto UDP -pool /Common/logging]\n",
            " class match x equals /Common/hosts\n",
            "}\n",
        );
        let refs = refs(source);
        for name in ["/Common/active", "/Common/fallback", "/Common/logging"] {
            let reference = refs
                .iter()
                .find(|reference| reference.name == name)
                .unwrap();
            assert_eq!(
                reference.category,
                IrulesObjectReferenceCategory::Pool,
                "{reference:?}"
            );
        }
        let datagroup = refs
            .iter()
            .find(|reference| reference.name == "/Common/hosts")
            .unwrap();
        assert_eq!(datagroup.category, IrulesObjectReferenceCategory::DataGroup);
    }

    #[test]
    fn authored_nominal_tombstone_withdraws_object_source_schema() {
        crate::support::install_surfaces();
        assert_eq!(
            ref_names(&pool_rule("rename pool p\n", "p")),
            [] as [String; 0]
        );
        assert!(
            ref_names(&pool_rule("rename pool p\n", "pool")).is_empty(),
            "source metadata cannot donate the old nominal schema after an explicit source cell barrier"
        );
    }

    #[test]
    fn object_refs_abstain_for_a_command_shadowed_by_a_user_proc() {
        crate::support::install_surfaces();
        assert!(
            ref_names(&pool_rule("proc pool {args} { return 1 }\n", "pool")).is_empty(),
            "a user `proc pool` takes the name over; its argument is not a pool name"
        );
        // Guard: the unshadowed command still resolves the reference.
        assert_eq!(
            ref_names(&pool_rule("set y 1\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
        );
    }

    #[test]
    fn object_refs_abstain_for_a_dynamic_binding() {
        crate::support::install_surfaces();
        assert!(
            ref_names(&pool_rule("rename $old p\n", "p")).is_empty(),
            "a dynamic rename must not make `p` an object-reference command"
        );
        assert_eq!(
            ref_names(&pool_rule("rename $old p\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
            "a dynamic rename must not take `pool`'s grammar away either"
        );
    }

    /// An unavailable `interp alias` cannot make `assign` a `set` command.
    #[test]
    fn unavailable_irules_alias_does_not_change_constant_propagation() {
        crate::support::install_surfaces();
        let source = "interp alias {} assign {} set\n\
                      when HTTP_REQUEST {\n\
                      assign p /Common/web_pool\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), [] as [String; 0]);
    }

    #[test]
    fn command_substitution_set_effects_stay_in_the_live_scope() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      puts [set p /Common/new]\n\
                      pool $p\n\
                      }\n";
        assert!(
            ref_names(source).is_empty(),
            "source values do not certify runtime set/read continuity"
        );
    }

    #[test]
    fn ordered_sibling_command_substitutions_share_effects() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      puts [set p /Common/first] [set p /Common/second]\n\
                      pool $p\n\
                      }\n";
        assert!(
            ref_names(source).is_empty(),
            "source values do not certify sibling observer/mutation ordering"
        );
    }

    #[test]
    fn body_role_command_substitution_runs_once_in_the_live_scope() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      catch [set p /Common/new]\n\
                      pool $p\n\
                      }\n";
        assert!(
            ref_names(source).is_empty(),
            "source values do not certify runtime set/read continuity"
        );
    }

    #[test]
    fn expr_role_substitutions_run_live_without_duplicate_references() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      if [string length [set p /Common/new]] { set seen 1 }\n\
                      if [string length [active_members /Common/expr_pool]] { set seen 1 }\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), vec!["/Common/expr_pool".to_owned()]);
    }

    #[test]
    fn braced_expr_text_does_not_invent_object_references() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {\n\
                      if {class match [HTTP::host] equals /Common/inert_dg} { set seen 1 }\n\
                      if [class match [HTTP::host] equals /Common/live_dg] { set seen 1 }\n\
                      }\n";
        let names = ref_names(source);
        assert_eq!(names, ["/Common/live_dg"]);
    }

    #[test]
    fn braced_expr_command_substitution_keeps_its_data_group_reference() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {\n\
                      if {[class match [HTTP::host] equals /Common/braced_dg]} { set seen 1 }\n\
                      }\n";
        assert_eq!(ref_names(source), ["/Common/braced_dg"]);
    }

    /// Regression coverage: `walk`/`recurse_token`'s mutual recursion over
    /// nested command-substitution bodies is capped at `MAX_WALK_DEPTH`
    /// (128). 300 nested `[…]` command substitutions is
    /// comfortably past the cap; the assertion is that extraction returns
    /// at all, not what it returns.
    #[test]
    fn deeply_nested_command_substitution_does_not_crash() {
        crate::support::install_surfaces();
        const DEPTH: usize = 300;
        let mut source = "when HTTP_REQUEST {\n    set x ".to_owned();
        for _ in 0..DEPTH {
            source.push('[');
        }
        source.push_str("pool /Common/p");
        for _ in 0..DEPTH {
            source.push(']');
        }
        source.push_str("\n}\n");
        let _ = refs(&source);
    }

    #[test]
    fn extracts_refs_nested_in_body_and_command_substitution() {
        crate::support::install_surfaces();
        let source = "\n\
            when HTTP_REQUEST {\n\
            \x20   set count [active_members /Common/app_pool]\n\
            \x20   LB::reselect pool /Common/fallback_pool\n\
            }\n";
        let names: Vec<String> = refs(source).iter().map(|r| r.name.clone()).collect();
        assert!(names.contains(&"/Common/app_pool".to_owned()));
        assert!(names.contains(&"/Common/fallback_pool".to_owned()));
    }
}
