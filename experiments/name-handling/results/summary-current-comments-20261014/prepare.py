from pathlib import Path
import difflib,json
from hashlib import sha256
from pygments.lexers import RustLexer
from pygments.token import Comment,String,Text,Whitespace
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-summary-current-comments-20261014'); changes={}
def edit(path,old,new):
 before=root.joinpath(path).read_text(); after=changes.get(path,(before,before))[1]
 assert after.count(old)==1,(path,old[:60])
 changes[path]=(before,after.replace(old,new))
p='rust/tcl-compiler/src/cfg_builder/global_write_info.rs'
old=root.joinpath(p).read_text();start=old.index('//! Per-proc summary');end=old.index('\nuse std::collections',start)
edit(p,old[start:end],'''//! Body-keyed MAY writes to global and namespace variables.
//!
//! The flow-insensitive scan collects every possible outer-place declaration
//! before projecting writes, so disjoint branches conservatively share alias
//! facts. Exact original procedure calls then propagate these summaries to a
//! monotone fixpoint. Unknown targets widen the outer frame instead of selecting
//! a body by its display name.
//!
//! Registry variable roles and binding-transition descriptors own recognition;
//! dynamic target and caller-frame obligations remain opaque.
''')
start=old.index('/// Detect, for every proc');end=old.index('pub(crate) fn detect_global_write_bodies_with_registry',start)
edit(p,old[start:end],'''/// Close body-keyed outer-place MAY facts over exact original call edges.
/// Missing source ownership or a missing callee summary widens the outer frame.
''')
edit(p,'''    /// Procedures sharing a short spelling both contribute to that spelling's
    /// may-summary. A call through `run` may resolve to either definition as
    /// namespaces and aliases change, so last-writer selection is unsound.''',
 '''    /// Equal leaf names retain independent body summaries.''')
edit(p,'        // Registered under both the qualified and short forms.','        // The fixture selector accepts either display for this single body.')
p='rust/tcl-compiler/src/compilation_unit.rs'
edit(p,'''    /// Module-wide `proc -> upvar summary` context (from
    /// [`crate::cfg_builder::prepare_cfg_context`]), the resolved command surface,
    /// and whether command dispatch is trace-visible.''',
 '''    /// Body-keyed caller-frame summaries from [`crate::cfg_builder::prepare_cfg_context`].''')
edit(p,'''    /// Module-wide `proc -> params` context (from
    /// [`crate::cfg_builder::prepare_cfg_context`]).''',
 '''    /// Original parameter lists keyed by the same issued body IDs.''')
edit(p,'''    /// Module-wide `proc -> outer-scope write summary` context (from
    /// [`crate::cfg_builder::prepare_cfg_context`]).''',
 '''    /// Outer-place MAY summaries keyed by the same issued body IDs.''')
def tokens(text):
 result=[]
 for _,kind,value in RustLexer(stripnl=False,ensurenl=False).get_tokens_unprocessed(text):
  if kind in String.Doc:
   assert value.startswith(('///','//!','/**','/*!'));continue
  if (kind in Comment and not kind in Comment.Preproc) or kind in Whitespace or (kind in Text and value.isspace()):continue
  result.append((str(kind),value))
 return result
assert tokens('#[test]\nfn x(){}')!=tokens('fn x(){}')
manifest=[];patch=[]
for path,(before,after) in changes.items():
 assert tokens(before)==tokens(after),path
 for side,text in [('before',before),('after',after)]:
  dest=out/side/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text)
 patch.extend(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
 manifest.append({'path':path,'before_sha256':sha256(before.encode()).hexdigest(),'after_sha256':sha256(after.encode()).hexdigest(),'meaningful_tokens_unchanged':len(tokens(before))})
patch=''.join(patch);(out/'implementation.patch').write_text(patch)
(out/'manifest.json').write_text(json.dumps({'files':manifest,'patch_sha256':sha256(patch.encode()).hexdigest()},indent=2)+'\n')
print(json.dumps({'files':len(manifest),'patch_sha256':sha256(patch.encode()).hexdigest(),'rust_tokens_unchanged':sum(m['meaningful_tokens_unchanged'] for m in manifest)}))
