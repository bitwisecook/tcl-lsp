from pathlib import Path
from hashlib import sha256
import difflib,json
from pygments.lexers import RustLexer
from pygments.token import Comment,Whitespace,Text,String
root=Path('/workspace/tcl-lsp'); out=Path('/tmp/name-cfg-summary-owner-docs-20261014')
changes={}
def edit(path,old,new):
 before=root.joinpath(path).read_text()
 after=changes.get(path,(before,before))[1]
 assert after.count(old)==1,(path,after.count(old),old[:80])
 changes[path]=(before,after.replace(old,new))
edit('docs/design/contracts/shared-utility-contracts-rust.md',
 '| Retained source body identity |',
 '| Original procedure summary ownership | `rust/tcl-compiler/src/cfg_builder/summary_candidates.rs`; `rust/tcl-compiler/src/cfg_builder/mod.rs`; `rust/tcl-compiler/src/compilation_unit.rs` | `OriginalProcedureSummaryOwners`; `OriginalProcedureSummaryOwners::for_module`; `CfgContext`; `PreparedCfgContext`; `PreparedCfgContext::from_source_input`; `LatticeRequest` | the fifth CFG context field retains the matching original Module and Registry; upvar, parameter and outer-write maps use issued ProcedureId coordinates; exact original call targets and effective operands select summaries, including retained child calls; missing, changed or foreign owners widen effects rather than joining display names; raw memo identity retains the Arc owner independently of certified registry-only templates; no Native entry, entered frame, committed write or completion grant | `xtask-owner-resolution` |\n| Retained source body identity |')
edit('docs/design/contracts/naming-consumer-matrix.md',
 '| Function-lattice request currency |',
 '| Original procedure effect summaries | `OriginalProcedureSummaryOwners::for_module`; fifth `CfgContext` field; `ProcedureId`-keyed upvar, parameter and global-write maps | CFG direct calls, command-substitution children and deferred own-frame scans select actual original call receipts and frozen effective operands through the retained Module. Missing/foreign/changed owners or unresolved targets widen caller/global effects; equal display names cannot select a body. Raw memo keys retain owner identity separately from registry-only certified reuse. No executed frame or committed write is established. |\n| Function-lattice request currency |')
edit('docs/design/contracts/naming-consumer-matrix.md',
 '| Lowered script and option metadata |',
 '| Tcllib list/set variable roles | Selected `struct::list`/`struct::set` subcommand schema; shared argument-role and possible-variable-name projections | Assign destinations, read-before-write variables, join `-keys` outputs and split outputs retain their selected positions and original effective-word origins. Conditional output roles remain MAY writes, callbacks retain separate reach, and purity belongs to the selected subcommand rather than the umbrella command. |\n| Lowered script and option metadata |')
edit('docs/design/compiler/name-resolution-implementer-guide.md',
 'Prepare module facts once and share the same `PreparedCfgContext` across its\nordinary and method CFGs.',
 '''`CfgContext` carries five fields: `ProcedureId`-keyed upvar summaries,
parameter lists and global-write summaries, the module command bindings, and
`OriginalProcedureSummaryOwners`. Obtain the tuple from
`prepare_cfg_context_with_registry`; keep its fifth field intact when passing
it through `PreparedCfgContext` or a `LatticeRequest`.
`OriginalProcedureSummaryOwners::for_module` retains only a Module whose
original source bindings match that Module and Registry. A default owner is
unavailable and cannot recover procedure identity from a display name.

CFG direct calls and their retained command-substitution children select the
actual original procedure allocation and frozen effective arguments through
this owner. Caller-frame parameter binding and outer-write propagation use the
same issued `ProcedureId`. Deferred own-frame scans join the same original
summaries. Unresolved calls or missing, changed and foreign source owners widen
effects; a genuine selected registry operation has no procedure-summary edge.
Raw memo equality retains the actual owner allocation; registry-only certified
templates apply their independent reuse boundary. These are conditional source
effects, not proof of an entered frame, committed write or Native call.

Prepare module facts once and share the same `PreparedCfgContext` across its
ordinary and method CFGs.''')
edit('docs/design/compiler/name-resolution-implementer-guide.md',
 'An unknown caught command is a separate uncertainty:',
 '''Tcllib list/set calls use this same selected schema. `struct::list assign`
declares repeated destinations; `shift`, `swap`, `delete` and
`struct::set include`/`exclude` declare read-before-write variables. Join
`-keys` operands and `split` result variables are conditional outputs; `split`
also retains its independent callback prefix. Consume the shared role/layout
queries with original effective operands, including alias captures, rather than
recognising those spellings in a consumer. Purity is declared on individual
subcommands: the list/set umbrella supplies no blanket purity, and writer or
callback roles cannot establish normal assignment or safe erasure.

An unknown caught command is a separate uncertainty:''')
edit('rust/tcl-compiler/src/cfg_builder/mod.rs',
 '''/// Module-wide CFG-determining context: the upvar-procs map, the
/// parameter-list map, the global-write-procs map, and the closed command
/// binding state
/// ([`global_write_info::detect_global_write_procs`]) [`prepare_cfg_context`]
/// returns. The single canonical definition [`crate::compilation_unit`]
/// reuses for its own `CfgContext` alias, so the two never drift apart.''',
 '''/// Body-keyed caller-frame, parameter and outer-write summaries, closed
/// command bindings, and the original Module owner selecting summary edges.
/// [`crate::compilation_unit`] shares this canonical tuple.''')
edit('rust/tcl-compiler/src/cfg_builder/mod.rs',
 '''    /// Resolve the caller-frame effects of every retained user procedure a
    /// direct source invocation may reach. Command identity and alias-prefix
    /// composition come exclusively from [`ModuleCommandBindings`].''',
 '''    /// Join caller-frame summaries through exact original call receipts and
    /// their frozen effective arguments.''')
def tokens(source):
 result=[]
 for _,kind,value in RustLexer(stripnl=False,ensurenl=False).get_tokens_unprocessed(source):
  if kind in String.Doc:
   assert value.startswith(('///','//!','/**','/*!'))
   continue
  if (kind in Comment and not kind in Comment.Preproc) or kind in Whitespace or (kind in Text and value.isspace()): continue
  result.append((str(kind),value))
 return result
assert tokens('#[test]\nfn x(){}') != tokens('fn x(){}')
assert tokens('let x=r#"a"#;') != tokens('let x=r#"b"#;')
manifest=[]; patches=[]
for path,(before,after) in changes.items():
 lexical=None
 if path.endswith('.rs'):
  assert tokens(before)==tokens(after),path
  lexical=len(tokens(before))
 target=out/'after'/path; target.parent.mkdir(parents=True,exist_ok=True);target.write_text(after)
 original=out/'before'/path;original.parent.mkdir(parents=True,exist_ok=True);original.write_text(before)
 patches.extend(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
 manifest.append({'path':path,'before_sha256':sha256(before.encode()).hexdigest(),'after_sha256':sha256(after.encode()).hexdigest(),'meaningful_rust_tokens_unchanged':lexical})
patch=''.join(patches);(out/'implementation.patch').write_text(patch)
(out/'manifest.json').write_text(json.dumps({'files':manifest,'patch_sha256':sha256(patch.encode()).hexdigest(),'checks':'Exact unique replacement; original public declarations and consumers read; RustLexer preserves attributes and all non-comment tokens. No compiled acceptance claimed.'},indent=2)+'\n')
print(json.dumps({'files':len(manifest),'patch_sha256':sha256(patch.encode()).hexdigest(),'rust_token_counts':[m['meaningful_rust_tokens_unchanged'] for m in manifest if m['meaningful_rust_tokens_unchanged'] is not None]}))
