from pathlib import Path
import difflib,subprocess,json,hashlib
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-import-shadow-geometry-20261014')
paths=['rust/tcl-compiler/src/command_binding.rs','rust/tcl-compiler/src/optimiser/helpers/naming.rs','rust/tcl-compiler/src/optimiser/helpers/mod.rs']
for p in paths:
 before=(root/p).read_text();after=before
 if p.endswith('command_binding.rs'):
  old='''        let qualified = nqn(name);
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(&qualified);
        self.opaque_namespaces.contains(&namespace)'''
  new='''        let parts = tcl_syntax::naming::written_command_parts(name.as_bytes());
        let path = tcl_core_types::ByteNamespacePath::from_segments(parts.qualifiers);
        let Some(namespace) = tcl_syntax::naming::checked_namespace_path_key(&path) else {
            return true;
        };
        self.opaque_namespaces.contains(&namespace)'''
  assert old in after;after=after.replace(old,new,1)
  after=after.replace('''    /// Whether `name` sits in a namespace whose resolution this scan could
    /// not enumerate — see [`Self::opaque_namespaces`].''','''    /// The constructed command-key label contributes only MAY trust invalidation;
    /// its written empty tail remains part of the namespace geometry.''',1)
  for old,new in [('("inner::p", vec!["inner::p", "ns::inner::p"])','("inner::p", vec!["::inner::p", "::ns::inner::p"])'),('("p", vec!["ns::p", "p"])','("p", vec!["::ns::p", "::p"])'),('("::inner::p", vec!["inner::p"])','("::inner::p", vec!["::inner::p"])')]:
   assert old in after;after=after.replace(old,new,1)
  marker='''    #[test]
    fn the_trust_snapshot_round_trips_every_mutation_field()'''
  test='''    #[test]
    fn opaque_namespace_imports_preserve_empty_command_tails() {
        let mutations = ModuleCommandMutations {
            opaque_namespaces: ["::ns".to_owned()].into_iter().collect(),
            ..ModuleCommandMutations::default()
        };
        for name in ["ns::", "::ns::", "::ns::::", "::ns::ordinary"] {
            assert!(!mutations.trusts_proc_binding(name), "{name}");
        }
        assert!(mutations.trusts_proc_binding("::ns"));
        assert!(mutations.trusts_proc_binding("::other::"));
    }

''';assert marker in after;after=after.replace(marker,test+marker,1)
 elif p.endswith('helpers/naming.rs'):
  after=after.replace('//! Namespace and qualified-name helpers for the optimiser.','//! Constructed-key geometry fixtures for the shared Syntax naming owner.',1)
  after=after.replace('pub fn namespace_parts(','fn namespace_parts(',1)
  after=after.replace('pub fn namespace_from_qualified(','fn namespace_from_qualified(',1)
  start=after.index('    let parts = namespace_parts(qname);');end=after.index('\n}',start)
  after=after[:start]+'''    let parts = tcl_syntax::naming::written_command_parts(qname.as_bytes());
    let path = tcl_core_types::ByteNamespacePath::from_segments(parts.qualifiers);
    tcl_syntax::naming::checked_namespace_path_key(&path).expect("UTF-8 constructed key")'''+after[end:]
  after=after.replace('''        assert_eq!(namespace_from_qualified("::a::b::c"), "::a::b");''','''        assert_eq!(namespace_from_qualified("::a::b::c"), "::a::b");
        assert_eq!(namespace_from_qualified("::a::b::"), "::a::b");
        assert_eq!(namespace_from_qualified("a::b::::"), "::a::b");''',1)
 else:
  assert 'pub mod naming;' in after;after=after.replace('pub mod naming;','#[cfg(test)]\nmod naming;',1)
 for stage,text in [('before',before),('after',after)]:
  path=out/stage/p;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text)
 subprocess.run(['/workspace/.cargo/bin/rustfmt','--edition','2024','--config','skip_children=true',str(out/'after'/p)],env={'RUSTUP_HOME':'/workspace/.rustup','CARGO_HOME':'/workspace/.cargo','PATH':'/usr/bin:/bin'},check=True)
patch=''.join(''.join(difflib.unified_diff((out/'before'/p).read_text().splitlines(keepends=True),(out/'after'/p).read_text().splitlines(keepends=True),fromfile='a/'+p,tofile='b/'+p)) for p in paths);(out/'implementation.patch').write_text(patch)
r=subprocess.run(['git','apply','--check',str(out/'implementation.patch')],cwd=root,capture_output=True,text=True);manifest={'validation':'rustfmt parsed; Main apply-check only; no Cargo','apply_check':r.returncode,'patch_sha256':hashlib.sha256(patch.encode()).hexdigest(),'before':{p:hashlib.sha256((out/'before'/p).read_bytes()).hexdigest() for p in paths},'after':{p:hashlib.sha256((out/'after'/p).read_bytes()).hexdigest() for p in paths}};(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(len(patch),manifest['patch_sha256'],r.returncode,r.stderr)
