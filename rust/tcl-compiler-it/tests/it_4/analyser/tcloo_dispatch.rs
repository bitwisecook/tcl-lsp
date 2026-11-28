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

//! `TclOO` dispatch: named objects, configurable property accessors and template-method self dispatch.

/// A named object (`CLASS create NAME`) must resolve its members like
/// `[CLASS new]` already does for W308 on an unknown method: the dispatch
/// resolver must consult `AnalysisResult::created_instance_commands` /
/// `instance_classes` for a bareword receiver too.
mod issue_1312_named_object_dispatch {
    use super::super::*;

    /// TP: `C create obj` then `obj nosuchmethod` must draw the same W308
    /// the handle form (`[C new]`) already does.
    #[test]
    fn named_object_draws_w308_on_an_unknown_method() {
        crate::install_surfaces();
        let src = "oo::class create C {\n\
                     method mrun {} { return 1 }\n\
                   }\n\
                   C create obj\n\
                   obj nosuchmethod\n";
        assert!(
            fires(src, "tcl9.0", "W308"),
            "named-object dispatch must draw W308 like the handle form: {:?}",
            codes(src, "tcl9.0")
        );
    }

    /// TP — parity check: the handle form and the named form must draw
    /// exactly the same W308 message for the identical class/method pair,
    /// so the two construction forms are provably treated alike.
    #[test]
    fn named_object_and_handle_object_draw_the_same_w308_message() {
        crate::install_surfaces();
        let handle = "oo::class create C { method mrun {} { return 1 } }\n\
                       set h [C new]\n\
                       $h nosuchmethod\n";
        let named = "oo::class create C { method mrun {} { return 1 } }\n\
                      C create obj\n\
                      obj nosuchmethod\n";
        let handle_msg = analyser_diags(handle, "tcl9.0")
            .into_iter()
            .find(|(c, _, _)| c == "W308")
            .map(|(_, m, _)| m)
            .expect("handle form draws W308");
        let named_msg = analyser_diags(named, "tcl9.0")
            .into_iter()
            .find(|(c, _, _)| c == "W308")
            .map(|(_, m, _)| m)
            .expect("named form draws W308");
        assert_eq!(handle_msg, named_msg);
    }

    /// TN — a named object calling a method its class actually declares
    /// must not draw a spurious W308.
    #[test]
    fn named_object_calling_a_real_method_draws_no_w308() {
        crate::install_surfaces();
        let src = "oo::class create C { method mrun {} { return 1 } }\n\
                   C create obj\n\
                   obj mrun\n";
        assert!(
            !fires(src, "tcl9.0", "W308"),
            "a real method must not draw W308: {:?}",
            codes(src, "tcl9.0")
        );
    }

    /// FP guard — a bareword command that merely *looks* like a named
    /// instance (an ordinary proc call, never bound via `CLASS create
    /// NAME`) must not be swept into the object-dispatch check and must
    /// never draw W307 "non-literal command name" — the whole point of
    /// gating the bareword site on `instance_classes`, not on bareword
    /// shape alone.
    #[test]
    fn an_ordinary_proc_call_never_draws_w307_or_w308() {
        crate::install_surfaces();
        let src = "proc obj {args} { return 1 }\nobj nosuchmethod\n";
        let got = codes(src, "tcl9.0");
        assert!(
            !got.iter().any(|c| c == "W307" || c == "W308"),
            "an ordinary proc call must draw neither: {got:?}"
        );
    }

    /// FP guard — `interp create child` binds `child` as a *command*
    /// (`created_instance_commands`), but never as a class instance
    /// (`instance_classes` stays unset for it) — the coroutine / `interp
    /// create` carve-out `record_bareword_instance_dispatch_site` documents.
    /// `child eval …` must draw neither W307 nor W308.
    #[test]
    fn interp_create_bareword_dispatch_draws_no_diagnostic() {
        crate::install_surfaces();
        let src = "interp create child\nchild eval { puts hi }\n";
        let got = codes(src, "tcl9.0");
        assert!(
            !got.iter().any(|c| c == "W307" || c == "W308"),
            "an `interp create` handle must draw neither: {got:?}"
        );
    }

    /// TP — the per-item (incremental) analysis path must agree with the
    /// whole-file `analyse` path byte-for-byte on this shape, including when
    /// the class is declared inside a proc body the per-item shell pass
    /// defers (`Analyser::analyse_per_item`, `replay_deferred_instances`,
    /// `finalise_bareword_dispatch_sites`).
    #[test]
    fn per_item_analysis_agrees_with_whole_file_analysis() {
        crate::install_surfaces();
        let src = "proc def {} { oo::class create ::K { method poke {} {} } }\n\
                   def\n\
                   ::K create inst\n\
                   inst nosuchmethod\n";
        let whole = Analyser::new().analyse(src, "tcl9.0");
        let per_item = Analyser::new().analyse_per_item(src, "tcl9.0");
        assert_eq!(
            per_item, whole,
            "per_item != analyse for:\n{src}\nper_item diagnostics: {:?}\nwhole diagnostics: {:?}",
            per_item.diagnostics, whole.diagnostics
        );
        assert!(
            whole
                .diagnostics
                .iter()
                .any(|d| d.code.to_string() == "W308"),
            "expected W308 on the named object created inside a deferred proc body: {:?}",
            whole.diagnostics
        );
    }
}

/// `$obj configure` on an `oo::configurable` class must not draw
/// `Unknown method 'configure' on class '::UnifiedTest'; did you mean
/// 'configure'?`: the accessors a configurable class generates for its
/// `property` members are declared by no `method` body, so they reach
/// neither the class's member tables nor the hierarchy's `method_providers`
/// on their own.
///
/// Both the existence check and the W308 *suggestion* list must go through
/// `ClassHierarchy::configures_by_property`, so they cannot drift apart —
/// otherwise the diagnostic could suggest the very word it had just called
/// unknown.
mod issue_1362_configurable_property_accessors {
    use super::super::*;

    /// All three dispatch spellings: a handle
    /// (`[C new]`), a named object (`C create obj`), and self-dispatch
    /// (`my configure` inside a method body).
    #[test]
    fn configure_on_a_configurable_class_draws_no_w308() {
        crate::install_surfaces();
        for src in [
            "oo::configurable create UnifiedTest {\n    property name\n}\n\
             set t [UnifiedTest new]\n$t configure -name x\n",
            "oo::configurable create UnifiedTest {\n    property name\n}\n\
             UnifiedTest create t\nt configure -name x\n",
            "oo::configurable create UnifiedTest {\n    property name\n\
             method go {} { my configure -name x }\n}\n",
        ] {
            assert!(
                !fires(src, "tcl9.0", "W308"),
                "configure is generated by the property: {:?} for {src:?}",
                codes(src, "tcl9.0")
            );
        }
    }

    /// TP guard — `cget` is **not** generated. `configurable.n` documents
    /// only `configure` (plus the internal `<ReadProp-name>` /
    /// `<WriteProp-name>` accessors) in both 9.0 and 9.1, and this
    /// workspace's own VM implements the same single method
    /// (`tcl-vm/src/cmd_oo.rs`), so `$obj cget -name` is a real
    /// unknown-method error and must keep drawing W308.
    #[test]
    fn cget_on_a_configurable_class_still_draws_w308() {
        crate::install_surfaces();
        let src = "oo::configurable create UnifiedTest {\n    property name\n}\n\
                   set t [UnifiedTest new]\n$t cget -name\n";
        assert!(
            fires(src, "tcl9.0", "W308"),
            "oo::configurable generates no cget: {:?}",
            codes(src, "tcl9.0")
        );
    }

    /// A configurable class with no `property` of its own still answers the
    /// accessors — the metaclass supplies them, so the abstention must rest
    /// on `Traits::CONFIGURES_BY_PROPERTY` rather than on the property list
    /// being non-empty (tclsh 9.0.4: `[C new] configure` on an empty
    /// `oo::configurable` class returns the empty dict, not an error).
    #[test]
    fn configurable_class_without_properties_draws_no_w308() {
        crate::install_surfaces();
        let src = "oo::configurable create C {}\nset c [C new]\n$c configure\n";
        assert!(!fires(src, "tcl9.0", "W308"), "{:?}", codes(src, "tcl9.0"));
    }

    /// The accessors are real methods on the configurable ancestor, so a
    /// plain `oo::class` subclass of one inherits them.
    #[test]
    fn subclass_of_a_configurable_class_draws_no_w308() {
        crate::install_surfaces();
        let src = "oo::configurable create Base {\n    property name\n}\n\
                   oo::class create Sub {\n    superclass Base\n}\n\
                   set s [Sub new]\n$s configure -name x\n";
        assert!(!fires(src, "tcl9.0", "W308"), "{:?}", codes(src, "tcl9.0"));
    }

    /// TP guard — the abstention is scoped to configurable classes. A plain
    /// `oo::class` instance really does fail with `unknown method
    /// "configure"` (tclsh 9.0.4), so W308 must keep firing there.
    #[test]
    fn configure_on_a_plain_oo_class_still_draws_w308() {
        crate::install_surfaces();
        let src = "oo::class create C {\n    method mrun {} { return 1 }\n}\n\
                   set c [C new]\n$c configure -name x\n";
        assert!(
            fires(src, "tcl9.0", "W308"),
            "a non-configurable class has no configure: {:?}",
            codes(src, "tcl9.0")
        );
    }

    /// TP guard — a genuinely unknown method on a configurable class is
    /// still unknown; the fix must not blanket-suppress W308 for the class.
    #[test]
    fn unknown_method_on_a_configurable_class_still_draws_w308() {
        crate::install_surfaces();
        let src = "oo::configurable create C {\n    property name\n}\n\
                   set c [C new]\n$c nosuchmethod\n";
        assert!(fires(src, "tcl9.0", "W308"), "{:?}", codes(src, "tcl9.0"));
    }

    /// A W308 must never suggest the word it just called unknown — the
    /// signature of the drift this fix removes, asserted generically so any
    /// future re-divergence between the existence check and the suggestion
    /// list is caught whatever the method name.
    #[test]
    fn w308_never_suggests_the_method_it_calls_unknown() {
        crate::install_surfaces();
        for src in [
            "oo::configurable create C {\n    property name\n}\nset c [C new]\n$c configure\n",
            "oo::configurable create C {\n    property name\n}\nset c [C new]\n$c cget -name\n",
            "oo::configurable create C {\n    property name\n}\nset c [C new]\n$c configur\n",
            "oo::class create C {\n    method mrun {} { return 1 }\n}\nset c [C new]\n$c mrunn\n",
        ] {
            for (code, msg, _) in analyser_diags(src, "tcl9.0") {
                if code != "W308" {
                    continue;
                }
                let unknown = msg
                    .split_once('\'')
                    .and_then(|(_, rest)| rest.split_once('\''))
                    .map(|(name, _)| name.to_owned())
                    .expect("W308 names the method in quotes");
                assert!(
                    !msg.contains(&format!("did you mean '{unknown}'?")),
                    "W308 suggested the method it called unknown: {msg}"
                );
            }
        }
    }
}

mod issue_1367_template_method_self_dispatch {
    use super::super::*;

    /// The ruff `Formatter` shape: an abstract base whose method bodies
    /// `my`-dispatch a method written only by concrete subclasses.  `my`
    /// late-binds on the actual receiver — always a subclass instance —
    /// and reaches unexported members, so the capitalised (unexported)
    /// subclass method is genuinely callable and W308 must abstain.
    #[test]
    fn my_dispatch_to_a_subclass_provided_method_draws_no_w308() {
        crate::install_surfaces();
        let src = "oo::class create Base {\n\
                   \x20   method run {} { my Render }\n\
                   }\n\
                   oo::class create Concrete {\n\
                   \x20   superclass Base\n\
                   \x20   method Render {} { return ok }\n\
                   }\n";
        assert!(
            !fires(src, "tcl8.6", "W308"),
            "a subclass defines the template method: {:?}",
            codes(src, "tcl8.6")
        );
    }

    /// The subclass may itself get the template method from a mixin — the
    /// descendant check resolves through the subclass's whole MRO, not
    /// just its own member table.
    #[test]
    fn my_dispatch_to_a_mixin_provided_template_method_draws_no_w308() {
        crate::install_surfaces();
        let src = "oo::class create Renderer {\n\
                   \x20   method Render {} { return ok }\n\
                   }\n\
                   oo::class create Base {\n\
                   \x20   method run {} { my Render }\n\
                   }\n\
                   oo::class create Concrete {\n\
                   \x20   superclass Base\n\
                   \x20   mixin Renderer\n\
                   }\n";
        assert!(
            !fires(src, "tcl8.6", "W308"),
            "the subclass inherits the template method from its mixin: {:?}",
            codes(src, "tcl8.6")
        );
    }

    /// TP guard — a `my` typo stays a warning even when subclasses exist:
    /// the abstention needs a subclass that *defines* the method, not
    /// merely a subclass.
    #[test]
    fn my_dispatch_to_a_method_no_subclass_defines_still_draws_w308() {
        crate::install_surfaces();
        let src = "oo::class create Base {\n\
                   \x20   method run {} { my Rendr }\n\
                   }\n\
                   oo::class create Concrete {\n\
                   \x20   superclass Base\n\
                   \x20   method Render {} { return ok }\n\
                   }\n";
        assert!(
            fires(src, "tcl8.6", "W308"),
            "no subclass defines 'Rendr': {:?}",
            codes(src, "tcl8.6")
        );
    }

    /// TP guard — the abstention is `my`-only.  `[self] M` dispatches
    /// through the object's own command, where the capitalised subclass
    /// method is unexported and genuinely unreachable (the same reach
    /// split as `my varname` vs `[self] varname`, tclsh 9.0.4), so the
    /// identical template shape keeps its W308 on the `[self]` spelling.
    #[test]
    fn self_command_dispatch_to_a_subclass_method_still_draws_w308() {
        crate::install_surfaces();
        let src = "oo::class create Base {\n\
                   \x20   method run {} { [self] Render }\n\
                   }\n\
                   oo::class create Concrete {\n\
                   \x20   superclass Base\n\
                   \x20   method Render {} { return ok }\n\
                   }\n";
        assert!(
            fires(src, "tcl8.6", "W308"),
            "[self] cannot reach an unexported subclass method: {:?}",
            codes(src, "tcl8.6")
        );
    }

    /// Baseline stays intact: with no subclass in the index at all, an
    /// unresolved `my` dispatch is still a warning — the pattern needs
    /// evidence, and a single-file view of only the base class has none.
    #[test]
    fn my_dispatch_with_no_subclass_in_view_still_draws_w308() {
        crate::install_surfaces();
        let src = "oo::class create Base {\n\
                   \x20   method run {} { my Render }\n\
                   }\n";
        assert!(
            fires(src, "tcl8.6", "W308"),
            "no defining subclass in view: {:?}",
            codes(src, "tcl8.6")
        );
    }

    /// The cross-file half (the ruff corpus's actual shape): the defining
    /// subclass lives in another document, delivered as the host-injected
    /// [`SubclassProvidedMethods`] view.  Same source as the test above —
    /// the view is the only difference, so together they pin that the
    /// abstention keys on the evidence, not the source shape.
    #[test]
    fn an_injected_workspace_subclass_view_clears_the_w308() {
        crate::install_surfaces();
        use tcl_compiler::analyser::{Analyser, SubclassProvidedMethods};
        let src = "oo::class create Base {\n\
                   \x20   method run {} { my Render }\n\
                   \x20   method oops {} { my Rendr }\n\
                   }\n";
        let mut view = SubclassProvidedMethods::new();
        view.entry("::Base".to_owned())
            .or_default()
            .insert("Render".to_owned());
        let mut analyser =
            Analyser::new().with_workspace_subclass_methods(Some(std::sync::Arc::new(view)));
        let result = analyser.analyse(src, "tcl8.6");
        let w308_methods: Vec<&str> = result
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W308")
            .filter_map(|d| {
                d.message
                    .split_once('\'')
                    .and_then(|(_, rest)| rest.split_once('\''))
                    .map(|(name, _)| name)
            })
            .collect();
        assert_eq!(
            w308_methods,
            vec!["Rendr"],
            "the injected view clears 'Render' and leaves the typo: {:?}",
            result.diagnostics
        );
    }
}
