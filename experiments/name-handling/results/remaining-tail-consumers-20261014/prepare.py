from pathlib import Path
from hashlib import sha256
import json,difflib
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-remaining-tail-consumers-20261014');changes={}
def edit(path,old,new):
 before=root.joinpath(path).read_text();after=changes.get(path,(before,before))[1]
 assert after.count(old)==1,(path,after.count(old),old[:100]);changes[path]=(before,after.replace(old,new))
p='rust/tcl-compiler/src/dynamic_names.rs'
edit(p,'''    let tail = base.rsplit("::").next().unwrap_or(base);
    tail.contains('$') || tail.contains('[')''',
'''    let tail = tcl_syntax::naming::written_command_tail(base.as_bytes());
    tail.contains(&b'$') || tail.contains(&b'[')''')
p='rust/tcl-compiler/src/command_binding.rs'
edit(p,'''                let tail = text.rsplit("::").next().unwrap_or(text);
                rebound.insert(nqn(&join_ns(&target, tail)));''',
'''                let tail = core::str::from_utf8(tcl_syntax::naming::written_command_tail(
                    text.as_bytes(),
                ))
                .expect("tail of a Unicode pattern");
                rebound.insert(tcl_syntax::naming::qualify(&target, tail));''')
edit(p,'        tcl_registry::NamespaceTransitionTarget::Current => current.map(nqn),',
'        tcl_registry::NamespaceTransitionTarget::Current => current.map(str::to_owned),')
edit(p,'''/// `ns` + `tail` as one qualified name, without doubling the separator.
fn join_ns(ns: &str, tail: &str) -> String {
    if ns == "::" {
        format!("::{tail}")
    } else {
        format!("{ns}::{tail}")
    }
}

''','')
old=root.joinpath(p).read_text();start=old.index('/// Record a namespace transition\'s effect on command *resolution*.');end=old.index('fn apply_package_transition(',start)
edit(p,old[start:end],'')
edit(p,'fn collect_namespace_resolution(\n',
'''/// Project literal import tails under the retained current namespace key.
/// Unknown patterns or targets widen lookup advice without selecting a command.
fn collect_namespace_resolution(
''')
edit(p,'    fn namespace_resolution_transitions_widen_but_creation_and_export_do_not() {',
'''    fn namespace_resolution_projection_preserves_written_runs_and_constructed_colons() {
        for (pattern, expected) in [
            ("::source:::tail", ":::::tail"),
            ("::source::a:b", ":::::a:b"),
            ("::source::", ":::::"),
        ] {
            let transition = NamespaceTransition::Forget {
                namespace: tcl_registry::NamespaceTransitionTarget::Current,
                patterns: vec![TransitionSubject::Literal(pattern.to_owned())],
            };
            let mut rebound = std::collections::HashSet::new();
            let mut changed = false;
            let mut opaque = std::collections::HashSet::new();
            assert!(collect_namespace_resolution(
                &transition, Some(":::"), &mut rebound, &mut changed, &mut opaque,
            ));
            assert!(changed);
            assert!(opaque.is_empty());
            assert_eq!(rebound, std::collections::HashSet::from([expected.to_owned()]));
        }
    }

    #[test]
    fn namespace_resolution_transitions_widen_but_creation_and_export_do_not() {''')
p='rust/tcl-compiler/src/analyser/diagnostics/validity.rs'
edit(p,'        let bare = cmd_name.rsplit("::").next().unwrap_or(cmd_name);',
'''        let bare = core::str::from_utf8(tcl_syntax::naming::written_command_tail(
            cmd_name.as_bytes(),
        ))
        .expect("tail of a Unicode command");''')
edit(p,'            let bare = cand.cmd_name.rsplit("::").next().unwrap_or(&cand.cmd_name);',
'''            let bare = core::str::from_utf8(tcl_syntax::naming::written_command_tail(
                cand.cmd_name.as_bytes(),
            ))
            .expect("tail of a Unicode command");''')
edit(p,'#[cfg(test)]\nmod selected_formal_tests {',
'''#[cfg(test)]
mod stub_name_geometry_tests {
    use super::UserResolutionFacts;

    #[test]
    fn document_stub_suppression_uses_written_tail_geometry() {
        let facts = UserResolutionFacts {
            non_proc_qnames: Default::default(),
            proc_offsets: Default::default(),
            rename_offsets: Default::default(),
            stub_names: ["known", "a:b"].into_iter().map(str::to_owned).collect(),
        };
        for name in ["known", "::ns:::known", "::ns::::known", "::ns::a:b"] {
            assert!(facts.resolves_to_user(name, "::other", &[], true, 0), "{name}");
        }
        for name in ["::ns::other", "::ns::", "::ns::a::b"] {
            assert!(!facts.resolves_to_user(name, "::other", &[], true, 0), "{name}");
        }
    }
}

#[cfg(test)]
mod selected_formal_tests {''')
p='rust/tcl-compiler/src/lowering/mod.rs'
source=root.joinpath(p).read_text();start=source.index('    /// `apply {{params} body ?ns?} ?arg ...?` — an anonymous lambda.');end=source.index('    fn register_body_unit(',start)
edit(p,source[start:end],'''    /// Retain a synthetic analysis body under its issued Module ID.
    /// Labels are displays; the body retains its own namespace/source receipt.
    /// Lambda parameters bind a fresh frame, while namespace bodies do not.
''')
edit(p,'''        // `name` stays the short leaf marker (`"apply"` / `"namespace-eval"`)
        // even when `label` carries a namespace prefix — matching every
        // other `Procedure::name`'s "short name" contract.
        let short_name = prefix.rsplit("::").next().unwrap_or(prefix).to_string();''',
'''        let short_name = tcl_syntax::naming::key_tail(label).to_owned();''')
p='rust/tcl-compiler/examples/mro_interproc.rs'
edit(p,'    let q = format!("::{head}");','    let q = tcl_syntax::naming::qualify("::", head);')
edit(p,'    let bare = head.rsplit("::").next().unwrap_or(head);',
'''    let bare = core::str::from_utf8(tcl_syntax::naming::written_command_tail(head.as_bytes()))
        .expect("tail of a Unicode command");''')
edit(p,'        let bare = qn.rsplit("::").next().unwrap_or(qn).to_string();',
'        let bare = tcl_syntax::naming::key_tail(qn).to_owned();')
edit(p,'''        let bare = head.trim_start_matches("::");
        let cand = format!("::{bare}");''',
'''        let cand = tcl_syntax::naming::qualify("::", head);''')
edit(p,'''            let cu_units = std::iter::once(("::top", &u.cu.top_level))
                .chain(u.cu.procedures.iter().map(|(q, fu)| (q.as_str(), fu)))
                .chain(u.cu.methods.iter().map(|(q, fu)| (q.as_str(), fu)));
            for (_qn, fu) in cu_units {''',
'''            let cu_units = std::iter::once(&u.cu.top_level)
                .chain(u.cu.procedures.values())
                .chain(u.cu.methods.values());
            for fu in cu_units {''')
edit(p,'/// Resolve a call head to a qualified proc name in the corpus registry.',
'''/// Select a corpus reporting label, with a unique-leaf fallback for the ceiling.
/// This approximation supplies no current source target or executable identity.''')
changes[p]=(changes[p][0],changes[p][1]+'''
#[cfg(test)]
mod tests {
    use super::resolve_call;
    use std::collections::HashMap;

    #[test]
    fn corpus_labels_keep_constructed_colons_and_refuse_ambiguous_leafs() {
        let parameters = HashMap::from([
            ("::ns::command".to_owned(), Vec::new()),
            (":::".to_owned(), Vec::new()),
        ]);
        let leaves = HashMap::from([
            ("command".to_owned(), vec!["::ns::command".to_owned()]),
            (":".to_owned(), vec![":::".to_owned()]),
            ("ambiguous".to_owned(), vec!["::a::ambiguous".to_owned(), "::b::ambiguous".to_owned()]),
        ]);
        assert_eq!(resolve_call("ns:::command", &parameters, &leaves).as_deref(), Some("::ns::command"));
        assert_eq!(resolve_call(":", &parameters, &leaves).as_deref(), Some(":::"));
        assert_eq!(resolve_call(":::", &parameters, &leaves).as_deref(), Some(":::"));
        assert!(resolve_call("ambiguous", &parameters, &leaves).is_none());
    }
}
''')
p='rust/tcl-compiler/src/interprocedural.rs'
edit(p,'    /// Per-procedure summaries keyed by qualified name.','    /// Procedure summaries keyed by their issued Module body IDs.')
edit(p,'    /// Per-method summaries keyed by qualified name.','    /// Method summaries keyed by their issued Module body IDs.')
manifest=[]
for path,(before,after) in changes.items():
 for side,value in [('before',before),('after',after)]:
  dest=out/side/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(value)
 manifest.append({'path':path,'before_sha256':sha256(before.encode()).hexdigest(),'after_sha256':sha256(after.encode()).hexdigest()})
(out/'manifest.json').write_text(json.dumps({'files':manifest,'compiled_acceptance':'unexecuted; Root owns all Cargo/Main builds','purpose':'written-tail lexical/diagnostic advice versus constructed-key displays; retained original current namespace remains verbatim; no Native authority'},indent=2)+'\n')
print(json.dumps({'files':len(manifest)}))
