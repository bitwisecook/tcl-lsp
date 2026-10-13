from pathlib import Path
import re
out=Path('/tmp/name-inline-fixtures-main-20261013');after=out/'after';root=Path('/workspace/tcl-lsp')
helper='''// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::borrow::Cow;
use crate::ir::ProcedureId;

pub(crate) trait FixtureBodyLabel {
    fn fixture_label(&self) -> Cow<'_, str>;
}
impl FixtureBodyLabel for crate::ir::Procedure {
    fn fixture_label(&self) -> Cow<'_, str> { Cow::Borrowed(&self.qualified_name) }
}
impl FixtureBodyLabel for crate::ir::MethodDef {
    fn fixture_label(&self) -> Cow<'_, str> { Cow::Owned(self.reporting_label()) }
}
impl FixtureBodyLabel for crate::compilation_unit::FunctionUnit {
    fn fixture_label(&self) -> Cow<'_, str> { Cow::Borrowed(&self.name) }
}
impl FixtureBodyLabel for super::ProcSummary {
    fn fixture_label(&self) -> Cow<'_, str> { Cow::Borrowed(&self.qualified_name) }
}

// A fixture label must identify one genuinely issued body before its ID is used.
pub(crate) fn fixture_id<'a, T: FixtureBodyLabel + 'a>(
    rows: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>, label: &str,
) -> ProcedureId {
    fixture_id_if_present(rows, label).unwrap_or_else(|| panic!("authored fixture body {label:?} absent"))
}
pub(crate) fn fixture_id_if_present<'a, T: FixtureBodyLabel + 'a>(
    rows: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>, label: &str,
) -> Option<ProcedureId> {
    let mut matching = rows.into_iter().filter(|(_, body)| body.fixture_label() == label);
    let first = matching.next().map(|(id, _)| *id);
    assert!(matching.next().is_none(), "authored fixture body {label:?} ambiguous");
    first
}

pub(crate) fn fixture_proc<'a>(analysis: &'a super::InterproceduralAnalysis, label: &str) -> Option<&'a super::ProcSummary> {
    let id = fixture_id_if_present(&analysis.procedures, label)?;
    analysis.procedures.get(&id)
}
pub(crate) fn edge_labels(analysis: &super::InterproceduralAnalysis, edges: &[ProcedureId]) -> Vec<String> {
    edges.iter().map(|id| analysis.procedures.get(id)
        .map(|body| body.qualified_name.clone())
        .or_else(|| analysis.methods.get(id).map(|body| body.base.qualified_name.clone()))
        .expect("fixture edge retains its exact body summary")).collect()
}
'''
path=after/'rust/tcl-compiler/src/test_body_identity.rs';path.write_text(helper)
p=after/'rust/tcl-compiler/src/interprocedural.rs';s=p.read_text();idx=s.index('#[cfg(test)]\nmod tests {');prod=s[:idx];t=s[idx:]
prod+='#[cfg(test)]\n#[path = "test_body_identity.rs"]\npub(crate) mod test_body_identity;\n\n'
t=t.replace('mod tests {','mod tests {\n    use super::test_body_identity::{fixture_id, fixture_id_if_present, fixture_proc, edge_labels};',1)
t=t.replace('mod effect_propagation_tests {','mod effect_propagation_tests {\n    use super::test_body_identity::fixture_proc;',1)
# Direct IA maps use the actual issued ID recovered uniquely from a known authored fixture.
t=re.sub(r'([A-Za-z_]\w*)\s*\.procedures\s*\.get\(("[^"\n]*"|caller|name)\)',lambda m:f'fixture_proc(&{m[1]}, {m[2]})',t)
# Direct inventory indexing uses exact ID, never a production String index implementation.
t=re.sub(r'([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\.procedures\[("[^"\n]*")\]',lambda m:f'{m[1]}.procedures.get(&fixture_id(&{m[1]}.procedures, {m[2]})).expect("authored fixture body")',t)
# Modules/units only: original source-point diagnostic helper.
t=t.replace('.get(body)','.get(&fixture_id(&unit.ir_module.procedures, body))',1)
t=t.replace('.or_else(|| unit.ir_module.methods.get(body).map(|body| &body.body));','.or_else(|| fixture_id_if_present(&unit.ir_module.methods, body).and_then(|id| unit.ir_module.methods.get(&id)).map(|body| &body.body));')
# Optional source-point selection keeps absence and method fallback.
t=t.replace('.get(&fixture_id(&unit.ir_module.procedures, body))\n            .map(|body| &body.body)', '.iter().find_map(|(id, row)| (fixture_id_if_present(&unit.ir_module.procedures, body) == Some(*id)).then_some(&row.body))')
# Calls are displayed only after exact ID lookup, retaining every original assertion label.
t=t.replace('.map(|s| s.calls.clone())','.map(|s| edge_labels(&ia, &s.calls))')
# Each borrowed summary's actual owning graph in its authored test.
edge_owners={'summary':'current','s':'ia','a':'ia','b':'ia','caller':'facts'}
# Source sections carry different owners for repeated local names.
for field in ['calls','direct_calls']:
 pattern=r'\b(summary|s|a|b|caller)\.'+field+r'\s*\.iter\(\)\s*\.any\(\|(\w+)\| \2 == ("::[^"\n]+")\)'
 def change(m):
  owner=edge_owners[m[1]]
  if m[1]=='caller' and m[3]=='"::ns::helper"':owner='ia'
  return f'edge_labels(&{owner}, &{m[1]}.{field}).iter().any(|{m[2]}| {m[2]} == {m[3]})'
 t=re.sub(pattern,change,t)
 t=re.sub(r'\b(a|b)\.'+field+r'\.contains\(&("::[^"\n]+")\.to_string\(\)\)',lambda m:f'edge_labels(&ia, &{m[1]}.{field}).contains(&{m[2]}.to_string())',t)
# Withheld loop and hosted callback use different actual graph producers.
start=t.index('        for withheld in [');end=t.index('\n    }',start)
region=t[start:end].replace('edge_labels(&current,','edge_labels(&result,');t=t[:start]+region+t[end:]
t=t.replace('let summary = &cu.interproc.as_ref().expect("interproc").procedures["::build"];','let actual = cu.interproc.as_ref().expect("interproc");\n        let summary = fixture_proc(actual, "::build").expect("authored build summary");')
# Hosted callback assertions were transformed with local summary.qualified_name expected labels only.
t=t.replace('edge_labels(&current, &summary.direct_calls).iter().any(|call| call == "::onNode")','edge_labels(actual, &summary.direct_calls).iter().any(|call| call == "::onNode")')
t=t.replace('''!unprovided
                .interproc
                .as_ref()
                .expect("interproc")
                .procedures
                .get("::build")''','''!fixture_proc(unprovided.interproc.as_ref().expect("interproc"), "::build")''')
t=t.replace('edge_labels(&current, &summary.direct_calls).iter().any(|call| call == "::onNode")','edge_labels(unprovided.interproc.as_ref().expect("interproc"), &summary.direct_calls).iter().any(|call| call == "::onNode")',1) if False else t
# Exact named function row for diagnostic rendering, keyed by its Module's ID.
t=t.replace('''cu.procedures
                .get("::build")''','''cu.procedures.get(&fixture_id(&cu.ir_module.procedures, "::build"))''')
# Temporary producer helpers now retain IA long enough to recover its genuine body ID.
t=t.replace('''        build(&src)
            .procedures
            .get("::p")
            .is_some_and(|s| edge_labels(&ia, &s.calls).iter().any(|c| c == "::q"))''','''        let ia = build(&src);
        fixture_proc(&ia, "::p").is_some_and(|s| edge_labels(&ia, &s.calls).iter().any(|c| c == "::q"))''')
t=t.replace('''            build(&src)
                .procedures
                .get("::p")''','''            let ia = build(&src);
            fixture_proc(&ia, "::p")''')
# Immutable optional borrowed rows should not acquire an extra reference after Index replacement.
t=re.sub(r'&([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\.procedures\.get\(&fixture_id\(&[^\n]+?\)\)\.expect\("authored fixture body"\))',r'\1',t)
p.write_text(prod+t)
p=after/'rust/tcl-compiler/src/optimiser/propagation.rs';s=p.read_text();idx=s.index('#[cfg(test)]\nmod tests {');prod=s[:idx];t=s[idx:]
t=t.replace('mod tests {','mod tests {\n    use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present, fixture_proc};',1)
t=t.replace('ia.procedures.get(name)','fixture_proc(ia, name)')
t=re.sub(r'(ia\d?)\.procedures\.contains_key\(("[^"\n]+")\)',lambda m:f'fixture_id_if_present(&{m[1]}.procedures, {m[2]}).is_some()',t)
t=t.replace('cu.ir_module.procedures["::f"]','cu.ir_module.procedures.get(&fixture_id(&cu.ir_module.procedures, "::f")).expect("authored fixture body")')
p.write_text(prod+t)
