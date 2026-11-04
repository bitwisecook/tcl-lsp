import hashlib, json, pathlib, re
root = pathlib.Path('rust/tcl-compiler/tests/data/shimmer-940-source-owner/native-s102-2026-11-04')
lines = (root / 'probe.log').read_text().splitlines()
cases = {}
configs = {}
for line in lines:
    m = re.match(r'^(default|off|error) CONFIG (\{.*\}) effective_line_length=117$', line)
    if m:
        configs[m[1]] = json.loads(m[2]); continue
    m = re.match(r'^(default|off|error) (\w+) (RECEIPT |REBORROW |S102=|ACTIONS )(.*)$', line)
    if m:
        cases.setdefault(m[1] + '/' + m[2], {})[m[3].strip().rstrip('=')] = json.loads(m[4])
expected = {'default/positive': 1, 'default/invalid': 0, 'default/one_trip': 0, 'default/original': 0, 'default/unconditional': 0, 'default/font_braced': 0, 'default/font_list': 0, 'default/suppressed': 0, 'default/unrelated_noqa': 1, 'off/positive': 0, 'error/positive': 1}
assert set(cases) == set(expected), (set(cases), set(expected))
assert configs == {'default': {'lineLength':117}, 'off': {'lineLength':117, 'shimmer':{'enabled':False}}, 'error': {'lineLength':117, 'diagnosticSeverity':{'S102':'error'}}}
for name, count in expected.items():
    case = cases[name]
    receipt = case['RECEIPT']
    assert receipt['status'] == 'complete' and receipt['version'] == 1 and receipt['dialectId'] == 'tcl8.6'
    assert len(case['S102']) == count
    assert [x for x in receipt['diagnostics'] if x.get('code') == 'S102'] == case['S102']
    if not name.startswith('off/'):
        assert case['REBORROW']['status'] == 'current'
        for field in ('uri','version','source','dialectId','revision','inputsEpoch','resultId'):
            assert case['REBORROW'][field] == receipt[field]
    if count:
        d=case['S102'][0]; line = 1 if name.endswith('unrelated_noqa') else 0
        assert d['range'] == {'start':{'line':line,'character':43},'end':{'line':line,'character':45}}
        assert d['severity'] == (1 if name.startswith('error/') else 2)
        assert d['message'] == "variable 'x' may repeatedly convert between int and list in this loop"
        actions = case['ACTIONS']
        enabled = [a for a in actions if 'disabled' not in a]
        assert len(enabled) == 1 and enabled[0]['title'] == 'Suppress S102 with a noqa comment' and enabled[0]['kind'] == 'quickfix'
        changes = enabled[0]['edit']['documentChanges']
        assert len(changes) == 1 and changes[0]['textDocument'] == {'uri':receipt['uri'],'version':1}
        assert changes[0]['edits'] == [{'range':{'start':{'line':line,'character':0},'end':{'line':line,'character':0}},'newText':'# noqa: S102\n'}]
        assert all(not any(diag.get('code') == 'S102' for diag in a.get('diagnostics', [])) for a in actions)
        assert all('disabled' in a for a in actions if a is not enabled[0])
assert lines[-1].startswith('PASS ')
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
artifacts = {p.name:digest(p) for p in sorted(root.iterdir()) if p.is_file()}
out = {'tested_source_commit':'cab14d1a4fe76da7e88eb5225b9533408d35e95b','source_tree_clean_before_and_after_build':True,'server_binary_sha256':'e4e4153afadf5906975a94fe661d81ef1040ecce0c630416c4cde9c0742d2ab7','server_binary_hash_unchanged_after_probe':True,'build_command':'cargo build -p tcl-lsp-server --bin tcl-lsp-server --message-format=json','probe_command':'LSP_940_SERVER=/path/to/tcl-lsp-server python3 probe.py','protocol':'textDocument/publishDiagnostics, tcl-lsp.currentDiagnostics, tcl-lsp.validateDiagnosticPublication, textDocument/codeAction','controls':configs,'cases':cases,'artifacts_sha256':artifacts,'interpretation':{'action':'The enabled action is the generic versioned noqa suppression; the unrelated extract action is disabled. Conditional compiler diagnostics do not grant source-context rewrite authority.','original_940':'Original and unconditional accumulator examples are quiet, but coverage is inconclusive: if, lappend and index-one body operations were not all modelled in this snapshot.','scope':'Native server delivery tested with Tcl 8.6; five Tcl profile and compiler authority comparisons are separately recorded in phase8-same-cu-2026-11-04.'}}
path=pathlib.Path('docs/LSP_RELEASE_S102_PROTOCOL_RECEIPT.json'); path.write_text(json.dumps(out,indent=2)+'\n')
print('verified', len(cases), 'cases,', len(artifacts), 'artifacts')
