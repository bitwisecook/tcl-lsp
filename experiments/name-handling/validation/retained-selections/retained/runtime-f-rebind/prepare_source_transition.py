#!/usr/bin/env python3
"""Read-only Git/source audit for the original Runtime174 packet; never invokes its runner."""
import datetime
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
REPO = Path('/workspace/tcl-lsp')
BASE = '2d456966b'
TARGET = '8df9a9169'


def git(*args):
    return subprocess.check_output(['git', *args], cwd=REPO)


def digest(value):
    return hashlib.sha256(value).hexdigest()


def function(source, name):
    matches = list(re.finditer(r'(?m)^\s*fn ' + re.escape(name) + r'\s*\(', source))
    assert len(matches) == 1, name
    start = matches[0].start()
    index = source.index('{', matches[0].end())
    depth = 0
    while index < len(source):
        if source.startswith('//', index):
            end = source.find('\n', index + 2)
            index = len(source) if end < 0 else end
            continue
        if source.startswith('/*', index):
            comment = 1
            index += 2
            while comment:
                if source.startswith('/*', index): comment += 1; index += 2
                elif source.startswith('*/', index): comment -= 1; index += 2
                else: index += 1
                assert index <= len(source)
            continue
        raw = re.match(r'(?:br|r)(#{0,255})"', source[index:])
        if raw:
            end = '"' + raw.group(1)
            index = source.index(end, index + raw.end()) + len(end)
            continue
        if source[index] == '"':
            index += 1
            while source[index] != '"':
                index += 2 if source[index] == '\\' else 1
            index += 1
            continue
        char = re.match(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\r\n])'", source[index:])
        if char:
            index += char.end()
            continue
        if source[index] == '{': depth += 1
        if source[index] == '}':
            depth -= 1
            if depth == 0: return source[start:index + 1].encode()
        index += 1
    raise AssertionError('Unclosed original function: ' + name)


def main():
    historical = json.loads((HERE / 'actual-source-binding.json').read_text())['actual_main_bindings']
    assert len(historical) == 82
    plan = json.loads((HERE / 'frozen-plan.json').read_text())
    compat = json.loads((HERE / 'jim-original-compatibility-manifest.json').read_text())
    groups = {'tcl_runtime': list(dict.fromkeys(plan['runtime'] + compat['exact'])), 'native_namespace_command_query': plan['query'], 'tcl_registry': plan['shared_registry']}
    assert {k:len(v) for k,v in groups.items()} == {'tcl_runtime':120,'native_namespace_command_query':51,'tcl_registry':3}
    roots = {'tcl_runtime':'runtime/rust/src/', 'native_namespace_command_query':'runtime/rust/tests/native_namespace_command_query.rs','tcl_registry':'rust/tcl-registry/src/'}
    selected = []
    sources = dict(historical)
    cached = {}
    for group, names in groups.items():
        leaves = [name.rsplit('::',1)[-1] for name in names]
        expression = r'^[[:space:]]*fn (' + '|'.join(re.escape(n) for n in leaves) + r')[[:space:]]*\('
        paths = git('grep','-l','-E',expression,BASE,'--',roots[group]).decode().splitlines()
        paths = [path.split(':',1)[1] for path in paths]
        old_sources = {path:git('show',BASE+':'+path).decode() for path in paths}
        for selector, leaf in zip(names, leaves):
            matches = [path for path,text in old_sources.items() if re.search(r'(?m)^\s*fn '+re.escape(leaf)+r'\s*\(',text)]
            assert len(matches)==1,(selector,matches)
            path=matches[0]
            before=function(old_sources[path],leaf)
            if path not in cached: cached[path]=git('show',TARGET+':'+path).decode()
            after=function(cached[path],leaf)
            selected.append({'build':group,'selector':selector,'path':path,'function':leaf,'before_function_sha256':digest(before),'after_function_sha256':digest(after),'function_byte_identical':before==after,'exact_unused_mut_only':path=='runtime/rust/src/interp/native_jim_lookup.rs' and leaf=='original_jim_namespace_receipt_does_not_keep_a_withdrawn_native_owner_alive' and before.replace(b'let mut interp = jim();', b'let interp = jim();', 1)==after})
            sources.setdefault(path,digest(old_sources[path].encode()))
    assert len(selected)==174
    assert all(row['function_byte_identical'] or row['exact_unused_mut_only'] for row in selected), [r['selector'] for r in selected if not r['function_byte_identical'] and not r['exact_unused_mut_only']]
    assert sum(not row['function_byte_identical'] for row in selected)==1
    changes=[]
    for path, expected in sources.items():
        old=git('show',BASE+':'+path); new=git('show',TARGET+':'+path)
        assert digest(old)==expected,path
        if old==new: continue
        before=HERE/'source-before'/path; after=HERE/'source-after'/path
        before.parent.mkdir(parents=True,exist_ok=True); after.parent.mkdir(parents=True,exist_ok=True)
        before.write_bytes(old); after.write_bytes(new)
        changes.append({'path':path,'before_sha256':digest(old),'after_sha256':digest(new),'before_git_blob':git('rev-parse',BASE+':'+path).decode().strip(),'after_git_blob':git('rev-parse',TARGET+':'+path).decode().strip(),'before_blob':str(before.relative_to(HERE)),'after_blob':str(after.relative_to(HERE))})
    source_diff=git('diff','--binary',BASE,TARGET,'--',*(r['path'] for r in changes))
    root_diff=git('diff','--binary',BASE,TARGET)
    (HERE/'source-transition.patch').write_bytes(source_diff)
    (HERE/'root-commit-transition.patch').write_bytes(root_diff)
    (HERE/'selected-function-preservation.json').write_text(json.dumps({'count':174,'groups':groups,'tests':selected},indent=2)+'\n')
    record={'prepared_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'historical_commit':git('rev-parse',BASE).decode().strip(),'target_commit':git('rev-parse',TARGET).decode().strip(),'historical_binding_sha256':digest((HERE/'actual-source-binding.json').read_bytes()),'source_transition_patch_sha256':digest(source_diff),'root_commit_transition_patch_sha256':digest(root_diff),'selected_function_audit_sha256':digest((HERE/'selected-function-preservation.json').read_bytes()),'historical_binding_count':len(historical),'guarded_source_count':len(sources),'historical_bindings':sources,'changed_live_source_count':len(changes),'changes':changes,'root_commit_chain':git('log','--reverse','--format=%H %s',BASE+'..'+TARGET).decode().splitlines(),'scope':'Exact original source successor only; all original packet guards, selectors and 180-second budgets remain. No runner/build/list/test/provider action.','runner_executed':False,'cargo_executed':False,'binary_list_executed':False,'test_executed':False}
    (HERE/'source-successor-2d-to-8df.json').write_text(json.dumps(record,indent=2)+'\n')
    print(json.dumps({'historical_binding_count':len(historical),'guarded_source_count':len(sources),'changed_source_count':len(changes),'selected_controls':len(selected),'changed_functions':[r['selector'] for r in selected if not r['function_byte_identical']]}))

if __name__=='__main__': main()
