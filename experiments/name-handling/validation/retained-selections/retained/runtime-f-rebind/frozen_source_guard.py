"""Exact reviewed Runtime174 source successor and execution-time provider guards."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import time

TRANSITION_SHA256 = '8c8a13da096ee70a5e579387edf375176ffa40fe8feb6a0fa12f315a84ef902c'
ORIGINAL_CHECKSUMS_SHA256 = '77978acf2418f347fa16ff42d157ce47f2bf35ed0031fe3a9fbbd555ae1cac8f'
PROVIDER_INPUT_REFERENCE_SHA256 = '6acd86458f251c29afeec31ae1fa35feaf8a61efc6f71ff08a528d6035ea4686'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate(packet, root):
    transition_path = packet / 'source-successor-2d-to-8df.json'
    original_path = packet / 'original-packet-checksums.json'
    if digest(transition_path) != TRANSITION_SHA256 or digest(original_path) != ORIGINAL_CHECKSUMS_SHA256:
        raise ValueError('Reviewed Runtime174 source transition or original packet checksums changed')
    for name, expected in json.loads(original_path.read_text())['files'].items():
        retained = 'original-run-main-native.py' if name == 'run-main-native.py' else 'original-README.md' if name == 'README.md' else name
        if digest(packet / retained) != expected:
            raise ValueError('Immutable original Runtime174 input changed: ' + name)
    transition = json.loads(transition_path.read_text())
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if head != transition['target_commit'] or head != '8df9a91699c0907ae3e0aefa1b10396feb967e8f':
        raise ValueError('Runtime174 requires the exact frozen F source commit')
    if transition['historical_commit'] != '2d456966b4619af28250e73e8585e4dac49be454':
        raise ValueError('Historical source commit changed')
    if digest(packet / 'actual-source-binding.json') != transition['historical_binding_sha256']:
        raise ValueError('Historical source binding changed')
    original_bindings = json.loads((packet / 'actual-source-binding.json').read_text())['actual_main_bindings']
    bindings = transition['historical_bindings']
    changes = transition['changes']
    if len(original_bindings) != 82 or len(bindings) != 97 or any(bindings.get(path) != value for path, value in original_bindings.items()) or len(changes) != 13 or len({row['path'] for row in changes}) != 13:
        raise ValueError('Exact reviewed source successor inventory changed')
    changed = {row['path']: row for row in changes}
    for path, before in bindings.items():
        old = subprocess.check_output(['git', 'show', transition['historical_commit'] + ':' + path], cwd=root)
        new = subprocess.check_output(['git', 'show', head + ':' + path], cwd=root)
        if hashlib.sha256(old).hexdigest() != before:
            raise ValueError('Original source binding changed: ' + path)
        expected = changed[path]['after_sha256'] if path in changed else before
        if hashlib.sha256(new).hexdigest() != expected or digest(root / path) != expected:
            raise ValueError('Frozen live source differs: ' + path)
    for row in changes:
        path = row['path']
        if bindings[path] != row['before_sha256']:
            raise ValueError('Source successor does not continue exact original: ' + path)
        for stage, commit in [('before', transition['historical_commit']), ('after', head)]:
            blob = subprocess.check_output(['git', 'rev-parse', commit + ':' + path], cwd=root, text=True).strip()
            if digest(packet / row[stage + '_blob']) != row[stage + '_sha256'] or blob != row[stage + '_git_blob']:
                raise ValueError('Retained source blob differs: ' + path)
    for name, expected in [('source-transition.patch', transition['source_transition_patch_sha256']), ('root-commit-transition.patch', transition['root_commit_transition_patch_sha256']), ('selected-function-preservation.json', transition['selected_function_audit_sha256'])]:
        if digest(packet / name) != expected:
            raise ValueError('Retained source transition evidence differs: ' + name)
    audit = json.loads((packet / 'selected-function-preservation.json').read_text())
    changed_functions = [row for row in audit['tests'] if not row['function_byte_identical']]
    if audit['count'] != 174 or len(audit['tests']) != 174 or len(changed_functions) != 1 or any(not (row['function_byte_identical'] or row['exact_unused_mut_only']) for row in audit['tests']):
        raise ValueError('Exact selected function preservation receipt changed')
    delta = changed_functions[0]
    if delta['path'] != 'runtime/rust/src/interp/native_jim_lookup.rs' or delta['function'] != 'original_jim_namespace_receipt_does_not_keep_a_withdrawn_native_owner_alive':
        raise ValueError('Unexpected selected function delta')
    traces = [key for key, value in __import__('os').environ.items() if value and (key.startswith('TCL_LSP_TRACE_') or key == 'TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')]
    if traces:
        raise ValueError('Ordinary Runtime replay requires diagnostic traces disabled: ' + ', '.join(sorted(traces)))
    return changed


def live_hash(changes, path, historical):
    successor = changes.get(path)
    if successor is None:
        return historical
    if successor['before_sha256'] != historical:
        raise ValueError('Live-source guard does not continue its exact original digest/purpose: ' + path)
    return successor['after_sha256']


def validate_fresh_jim(packet, tree, out):
    """Read original provider inputs and recheck upstream at actual execution."""
    reference_path = packet / 'provider-input-reference.json'
    if digest(reference_path) != PROVIDER_INPUT_REFERENCE_SHA256:
        raise ValueError('Reviewed original Jim provider/configuration reference changed')
    old_path = packet / 'provider-check.json'
    historical = json.loads(old_path.read_text())
    reference = json.loads(reference_path.read_text())
    for relative, expected in reference['inputs'].items():
        if digest(tree / relative) != expected:
            raise ValueError('Original Jim source/configuration/harness input changed: ' + relative)
    command = historical['remote_command']
    if command != ['git', 'ls-remote', 'https://github.com/msteveb/jimtcl.git', 'HEAD']:
        raise ValueError('Original upstream provider command changed')
    started = time.monotonic()
    remote = subprocess.run(command, capture_output=True, timeout=30, check=False)
    (out / 'fresh-upstream.stdout').write_bytes(remote.stdout)
    (out / 'fresh-upstream.stderr').write_bytes(remote.stderr)
    revision = subprocess.check_output(['git', '-C', str(tree), 'rev-parse', 'HEAD'], text=True).strip()
    current = dict(historical)
    current.update(checked_at=datetime.datetime.now(datetime.timezone.utc).isoformat(), remote_exit_code=remote.returncode, remote_stdout=remote.stdout.decode(), remote_stderr=remote.stderr.decode(), remote_elapsed_seconds=time.monotonic()-started, local_revision=revision, matches_current_upstream_head=remote.returncode == 0 and remote.stdout.decode() == revision + '\tHEAD\n', actual_tree=str(tree.resolve()), fresh_provider_inputs={relative:digest(tree / relative) for relative in reference['inputs']}, historical_provider_check_sha256=digest(old_path), fresh_provider_metadata_status='not yet queried')
    (out / 'fresh-provider-check.json').write_text(json.dumps(current, indent=2) + '\n')
    if not current['matches_current_upstream_head'] or revision != historical['local_revision'] or digest(tree / 'jimsh') != historical['interpreter_sha256'] or digest(tree / 'jim.c') != historical['original_jim_c_sha256']:
        raise ValueError('Execution-time original Jim provider/head/source/binary pairing differs')
    probe = subprocess.run([str(tree / 'jimsh')], input=b'puts [list [info patchlevel]]\n', capture_output=True, timeout=30, check=False)
    (out / 'fresh-provider-metadata.stdout').write_bytes(probe.stdout)
    (out / 'fresh-provider-metadata.stderr').write_bytes(probe.stderr)
    current.update(fresh_provider_metadata_status='queried', metadata_query_exit=probe.returncode, metadata_query_stdout_sha256=hashlib.sha256(probe.stdout).hexdigest(), metadata_query_stderr_sha256=hashlib.sha256(probe.stderr).hexdigest())
    (out / 'fresh-provider-check.json').write_text(json.dumps(current, indent=2) + '\n')
    if probe.returncode or probe.stdout != (historical['reported_version'] + '\n').encode() or probe.stderr:
        raise ValueError('Original Jim metadata pairing differs')
    return current
