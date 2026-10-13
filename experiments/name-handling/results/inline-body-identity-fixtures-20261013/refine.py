from pathlib import Path
import re
out=Path('/tmp/name-inline-fixtures-main-20261013');base=out/'before';after=out/'after';p=after/'rust/tcl-compiler/src/interprocedural.rs';s=p.read_text();idx=s.index('#[cfg(test)]\nmod tests {');prefix=s[:idx];t=s[idx:]
# Format-independent edge assertions preserve labels by following actual IDs to their summaries.
pattern=r'\b(caller|s|summary)\s*\.\s*(calls|direct_calls)\s*\.\s*iter\(\)\s*\.\s*any\(\|(\w+)\| \3 == ("::[^"\n]+")\)'
def adapt(m):
 owner='facts' if m[1]=='caller' else 'ia'
 if m[1]=='caller' and m[4]=='"::ns::helper"':owner='ia'
 if m[1]=='summary':owner='actual'
 return f'edge_labels(&{owner}, &{m[1]}.{m[2]}).iter().any(|{m[3]}| {m[3]} == {m[4]})'
t=re.sub(pattern,adapt,t)
# The negative provider fixture belongs to a separate Module, so its IDs must
# be resolved through that same Module's graph, never the earlier positive one.
start=t.index('            !fixture_proc(unprovided.interproc');end=t.index('            "an unloaded graph',start)
t=t[:start]+t[start:end].replace('edge_labels(actual,', 'edge_labels(unprovided.interproc.as_ref().expect("interproc"),')+t[end:]
t=t.replace('''            fixture_proc(&cu, "::build")
                .into_iter()''','''            cu.procedures.get(&fixture_id(&cu.ir_module.procedures, "::build"))
                .into_iter()''')
start=t.index('        let statements = unit\n');end=t.index('        statements.into_iter()',start)
t=t[:start]+'''        let statements = fixture_id_if_present(&unit.ir_module.procedures, body)
            .and_then(|id| unit.ir_module.procedures.get(&id))
            .map(|row| &row.body)
            .or_else(|| fixture_id_if_present(&unit.ir_module.methods, body)
                .and_then(|id| unit.ir_module.methods.get(&id)).map(|row| &row.body));
'''+t[end:]
p.write_text(prefix+t)
