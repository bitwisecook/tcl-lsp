#!/usr/bin/env python3
"""Capture installed provider setup only; never build, install, or alter source trees."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else '/workspace/tcl-lsp').resolve()
OUT = Path(__file__).resolve().parent
NOW = lambda: datetime.datetime.now(datetime.timezone.utc).isoformat()

def sha(path):
    return hashlib.file_digest(Path(path).open('rb'), 'sha256').hexdigest()

def record_file(path):
    path = Path(path)
    return {'path': str(path), 'realpath': str(path.resolve()), 'bytes': path.stat().st_size, 'sha256': sha(path)}

def run(name, argv, env=None, stdin=None, cwd=None):
    before = NOW()
    result = subprocess.run(argv, input=stdin, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=cwd or OUT, env=env, timeout=45)
    channels = {}
    for channel in ['stdout', 'stderr']:
        target = OUT / (name + '.' + channel)
        target.write_bytes(getattr(result, channel))
        channels[channel] = {'file': target.name, 'bytes': target.stat().st_size, 'sha256': sha(target)}
    return {'utc_start': before, 'utc_end': NOW(), 'argv': [str(x) for x in argv], 'cwd': str(cwd or OUT), 'exit': result.returncode, **channels}, result

def dependencies(name, exe):
    record, raw = run(name + '-ldd', ['ldd', str(exe)])
    paths = []
    for line in raw.stdout.decode().splitlines():
        match = re.search(r'(?:=>\s+)?(/[^\s]+)', line)
        if match:
            path = Path(match[1])
            if path.is_file() and str(path.resolve()) not in {x['realpath'] for x in paths}:
                paths.append(record_file(path))
    return {'inspection': record, 'actual_shared_objects': paths}

manifest = ROOT / 'rust/tcl-dialect/data/reference-toolchains.tsv'
reference = []
for line in manifest.read_text().splitlines():
    if line and not line.startswith('#'):
        release, patchlevel, tag = line.split('\t')
        reference.append((release, patchlevel, tag))
# These package versions are the existing owner's selected values, rechecked below
# against the loaded actual package. No semantic test expectation is generated.
tcltest_versions = {'8.4': '2.2.11', '8.5': '2.3.8', '8.6': '2.5.11', '9.0': '2.5.11', '9.1': '2.6.0'}
report = {'schema_version': 1, 'utc_start': NOW(), 'question': 'Do installed original provider executables select their matching original script/test support under the shared source-tree execution environment?', 'scope': 'setup/pairing acceptance only; no semantic conformance or unmeasured capability claim', 'owner': {'source_tree_execution': 'tcl_test_support::run_script_from_source_tree -> tclsh_from_source_tree -> run_script_with_libraries', 'selection_test': 'tcl_test_support::tests::source_tree_execution_selects_matching_original_init_and_tcltest', 'execution_adapter': 'capture.py mirrors owner environment with original raw process channels; C scripts supplied on stdin, Jim script uses file mode so argv0 exists'}, 'harness': [record_file(OUT/'capture.py'), record_file(OUT/'c-probe.tcl'), record_file(OUT/'jim-probe.tcl')], 'repository_owners': [record_file(manifest), record_file(ROOT/'rust/tcl-test-support/src/lib.rs'), record_file(ROOT/'rust/tcl-test-support/src/jim.rs'), record_file(ROOT/'rust/tcl-test-support/jim-reference.txt'), record_file(ROOT/'scripts/dev/tcl-reference-toolchains.sh'), record_file(ROOT/'scripts/dev/ensure-jim-oracle.sh')], 'providers': []}

# This is a real-time upstream observation, separate from the pinned manifest.
network, head = run('jim-upstream-head', ['git', 'ls-remote', 'https://github.com/msteveb/jimtcl.git', 'HEAD'])
report['jim_upstream_recheck'] = network
report['jim_upstream_recheck']['head'] = head.stdout.decode().split()[0] if head.returncode == 0 else None

for release, patchlevel, tag in reference:
    tree = ROOT / 'tmp' / ('tcl' + patchlevel)
    exe = tree/'unix/tclsh'
    env = os.environ.copy()
    env['TCL_LIBRARY'] = str(tree/'library')
    env.pop('TCLLIBPATH', None)
    env['LD_LIBRARY_PATH'] = str(tree/'unix') + (os.pathsep + env['LD_LIBRARY_PATH'] if env.get('LD_LIBRARY_PATH') else '')
    env['PAIRING_EXPECTED_PATCHLEVEL'] = patchlevel
    env['PAIRING_EXPECTED_TCLTEST'] = tcltest_versions[release]
    evidence, result = run('c-' + patchlevel, [str(exe)], env, (OUT/'c-probe.tcl').read_bytes())
    selected = ['generic/tcl.h', 'generic/tclBasic.c', 'generic/tclNamesp.c', 'generic/tclCmdMZ.c', 'library/init.tcl', 'library/package.tcl', 'library/tm.tcl', 'library/tcltest/pkgIndex.tcl', 'library/tcltest/tcltest.tcl', 'tests/all.tcl', 'tests/tcltest.test', 'manifest.uuid', 'unix/tclConfig.sh', 'unix/config.status']
    row = {'provider': 'C Tcl', 'patchlevel': patchlevel, 'release': release, 'source_tag_from_owner': tag, 'source_root': str(tree), 'executable': record_file(exe), 'selected_source_and_support': [record_file(tree/p) for p in selected if (tree/p).is_file()], 'static_tcl_archives': [record_file(p) for p in sorted((tree/'unix').glob('libtcl*.a'))], 'runtime_dependencies': dependencies('c-' + patchlevel, exe), 'invocation': evidence, 'environment_changes': {'TCL_LIBRARY': env['TCL_LIBRARY'], 'TCLLIBPATH': 'removed', 'LD_LIBRARY_PATH': env['LD_LIBRARY_PATH'], 'PAIRING_EXPECTED_PATCHLEVEL': patchlevel, 'PAIRING_EXPECTED_TCLTEST': tcltest_versions[release]}, 'acceptance': result.returncode == 0 and result.stderr == b'' and result.stdout.endswith(b'pairing=accepted\n')}
    # Validate the existing immutable input inventory without duplicating its file list.
    integrity_path = ROOT / 'experiments/name-handling/upstream-integrity' / (tag + '.json')
    integrity = json.loads(integrity_path.read_text())
    digest = hashlib.sha256()
    mismatches = []
    matched = 0
    for relative, expected_blob in sorted(integrity['git_blob_sha1'].items()):
        path = tree/relative
        if not path.is_file():
            mismatches.append({'path': relative, 'reason': 'missing'})
            continue
        data = path.read_bytes()
        actual_blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
        digest.update((hashlib.sha256(data).hexdigest() + '  ' + relative + '\n').encode())
        if actual_blob == expected_blob:
            matched += 1
        else:
            mismatches.append({'path': relative, 'expected_git_blob': expected_blob, 'actual_git_blob': actual_blob})
    row['original_input_inventory_validation'] = {'inventory': record_file(integrity_path), 'immutable_commit': integrity['commit'], 'scope': integrity['scope'], 'expected_count': len(integrity['git_blob_sha1']), 'matched_count': matched, 'mismatch_count': len(mismatches), 'mismatches': mismatches, 'current_named_input_inventory_sha256': digest.hexdigest(), 'aggregate_recipe': 'SHA256 of UTF-8 lines: file SHA256, two spaces, original relative path, LF; paths sorted by existing immutable inventory', 'all_match': not mismatches}
    # Reuse immutable source-integrity records by reference, comparing current blobs.
    prior = ROOT / 'experiments/name-handling/namespace-which-compiler/upstream-source-integrity.json'
    matches = []
    for original in json.loads(prior.read_text())['rows']:
        if original['release'] == patchlevel:
            data = (tree/original['path']).read_bytes()
            actual_blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            matches.append({'original_record': record_file(prior), 'immutable_commit': original['ref'], 'source_path': original['path'], 'current_source_sha256': hashlib.sha256(data).hexdigest(), 'expected_git_blob': original['result']['sha'], 'actual_git_blob': actual_blob, 'matches': actual_blob == original['result']['sha']})
    row['existing_upstream_integrity_reuse'] = matches
    prior = ROOT / 'experiments/name-handling/rename-window-slot-identity' / patchlevel / 'provenance.json'
    if prior.is_file():
        old = json.loads(prior.read_text())
        row['existing_init_integrity_reuse'] = {'original_record': record_file(prior), 'expected_init_sha256': old['init_tcl_sha256'], 'matches': old['init_tcl_sha256'] == sha(tree/'library/init.tcl')}
    report['providers'].append(row)

jim_root = ROOT/'tmp/jimtcl-oracle'
manifest_values = dict(line.split('=', 1) for line in (ROOT/'rust/tcl-test-support/jim-reference.txt').read_text().splitlines() if '=' in line)
revision, raw_revision = run('jim-local-revision', ['git', '-C', str(jim_root), 'rev-parse', 'HEAD'])
status, raw_status = run('jim-tracked-status', ['git', '-C', str(jim_root), 'status', '--porcelain', '--untracked-files=no'])
env = os.environ.copy()
for key in ['TCL_LIBRARY', 'TCLLIBPATH', 'JIMLIB']:
    env.pop(key, None)
env['PAIRING_EXPECTED_PATCHLEVEL'] = manifest_values['patchlevel']
env['PAIRING_JIM_SUPPORT'] = str(jim_root/'tcltest.tcl')
evidence, result = run('jim', [str(jim_root/'jimsh'), str(OUT/'jim-probe.tcl')], env)
# Record the rejected exploratory -e surface rather than silently treating it as setup success.
rejected, rejected_raw = run('jim-e-tcltest-negative', [str(jim_root/'jimsh'), '-e', 'puts [info patchlevel]; puts [info nameofexecutable]; puts [info commands info]; puts [package require tcltest]'], env)
selected = ['jim.c', 'jim.h', 'jimsh.c', 'jim-namespace.c', 'initjimsh.tcl', '_initjimsh.c', 'tclcompat.tcl', '_tclcompat.c', 'tcltest.tcl', 'jim-config.h', 'jimautoconf.h', 'auto.def', 'config.log']
report['providers'].append({'provider': 'Jim', 'source_root': str(jim_root), 'reference': manifest_values, 'executable': record_file(jim_root/'jimsh'), 'static_library': record_file(jim_root/'libjim.a'), 'selected_source_and_support': [record_file(jim_root/p) for p in selected if (jim_root/p).is_file()], 'runtime_dependencies': dependencies('jim', jim_root/'jimsh'), 'local_revision': revision, 'tracked_source_status': status, 'matches_manifest_revision': raw_revision.stdout.strip().decode() == manifest_values['revision'], 'tracked_source_clean': raw_status.returncode == 0 and raw_status.stdout == b'', 'matches_latest_upstream_at_execution': raw_revision.stdout.strip() == head.stdout.split()[0] if head.returncode == 0 else False, 'invocation': evidence, 'environment_changes': {'TCL_LIBRARY': 'removed', 'TCLLIBPATH': 'removed', 'JIMLIB': 'removed', 'PAIRING_EXPECTED_PATCHLEVEL': env['PAIRING_EXPECTED_PATCHLEVEL'], 'PAIRING_JIM_SUPPORT': env['PAIRING_JIM_SUPPORT']}, 'support_selection': 'explicitly sources the original tree tcltest.tcl in file mode; embedded initjimsh/tclcompat input hashes retained separately', 'acceptance': result.returncode == 0 and result.stderr == b'' and result.stdout.endswith(b'pairing=accepted\n'), 'negative_setup_surface': {'invocation': rejected, 'expected_limitation': 'Jim -e has no argv0; original tcltest.tcl source refuses before setup completes', 'failed_as_expected': rejected_raw.returncode != 0 and b'argv0' in rejected_raw.stderr}})
report['utc_end'] = NOW()
report['all_pairings_accepted'] = all(p['acceptance'] for p in report['providers'])
report['all_retained_c_input_inventories_match'] = all(p['original_input_inventory_validation']['all_match'] for p in report['providers'] if p['provider'] == 'C Tcl')
report['limitations'] = ['Only setup, identity and explicit script/test support pairing were executed; no command-name or runtime semantics were tested.', 'Named source/support files and static archives are hashed; existing immutable C input inventories are validated by reference without duplicating their file lists; no source/binary copy is retained.', 'C source tags come from the shared reference manifest; existing immutable compiler-source integrity records are reused where available, and matching init hashes are checked independently.', 'Hashing a static archive records its installed build input; it is not a reproducible-build proof that the entire archive was linked.', 'Jim startup support is embedded in the executable; initjimsh/tclcompat generated-C and original Tcl inputs are hashed, without a separate native execution proof for each helper.', 'The historical shared-owner PASS log is independent supporting evidence, not substituted for these actual process captures.']
(OUT/'capture.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'accepted': report['all_pairings_accepted'], 'providers': [{'provider': p['provider'], 'patchlevel': p.get('patchlevel', p.get('reference', {}).get('patchlevel')), 'accepted': p['acceptance']} for p in report['providers']], 'jim_remote_head': report['jim_upstream_recheck']['head']}))
