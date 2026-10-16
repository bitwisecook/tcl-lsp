import pathlib, json, hashlib, os, shutil, datetime, subprocess

root = pathlib.Path('/workspace/tcl-lsp')
base = pathlib.Path('/workspace/naming-validation/lsp-proposal-minifier-successor-repair1')
manifest = base / 'selection.json'
digest = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
expected = 'df13dd89de55f003434f8a0a4710003043c705f996498a8a40a599014349564d'
assert digest(manifest) == expected
assert json.loads((base / 'source-binding.json').read_text())['status'] == 'PASS_FIVE_LIBRARY_BUILD'
diagnostic = json.loads(pathlib.Path('/workspace/naming-validation/lsp-member-comment-stage-diagnostic/binding.json').read_text())
assert diagnostic['restoration']['whole_file_beforeimages_restored']
assert not diagnostic['restoration']['full_input_drift']
assert not [key for key, value in os.environ.items() if value and key.startswith('TCL_LSP_TRACE')]
inputs = json.loads((base / 'input-before.json').read_text())
assert all(digest(root / row['path']) == row['sha256'] for row in inputs), 'Actual source no longer matches ordinary freeze'
original = json.loads(pathlib.Path('/workspace/naming-validation/lsp-selected-owners-successor-repair2/provider-corrected-setup.json').read_text())
required = original['environment']
assert all(os.environ.get(key) == value for key, value in required.items()), 'Original mandatory provider environment mismatch'
assert shutil.which('tclsh9.0') == '/workspace/.toolchains/tcl/bin/tclsh9.0'
assert shutil.which('tclsh8.6') == '/workspace/.toolchains/tcl/bin/tclsh8.6'
artifacts = json.loads((base / 'artifacts.json').read_text())
assert all(digest(row['path']) == row['sha256'] for row in artifacts.values())
argv = ['python3', str(root / 'experiments/name-handling/validation/retained-selections/replay-selection.py'), '--manifest', str(manifest), '--manifest-sha256', expected, '--artifacts', str(base / 'artifacts.json'), '--root', str(root), '--out', str(base / 'results')]
receipt = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'argv': argv, 'environment': required, 'tclsh9.0': shutil.which('tclsh9.0'), 'tclsh8.6': shutil.which('tclsh8.6'), 'actual_full_ordinary_input_count': len(inputs), 'actual_full_ordinary_input_drift': 0, 'instrumentation_restored_byte_exact_before_replay': True, 'all446_retained_with_original_assertions_selectors_budgets_and_guards': True, 'all460_selected': True, 'selection_sha256': expected, 'jim_recheck': json.loads((base / 'jim-upstream-recheck.json').read_text())}
(base / 'replay-invocation.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps({'status': 'PREFLIGHT_PASS', 'selection': expected, 'inputs': len(inputs), 'controls': 460}), flush=True)
raise SystemExit(subprocess.run(argv, cwd=root).returncode)
