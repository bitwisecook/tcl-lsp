import datetime, hashlib, importlib.util, json, os, pathlib, re, signal, subprocess, time
repo = pathlib.Path('/workspace/tcl-lsp')
out = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('capture_helpers', str(out.parent / 'repair-01/capture-helpers.py'))
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
source = repo / 'rust/tcl-vm/src/cmd_oo.rs'
assert helper.digest(source) == '663fdd0b359e09e947944cc734736f678e31e5f4b17364e77d3b63301e475caf'
assert not (out / 'vm-execution-record.json').exists()
assert os.environ['CARGO_HOME'] == str(repo / 'tmp/name-tools/cargo')
environment = os.environ.copy()
environment['CARGO_TARGET_DIR'] = str(repo / 'tmp/name-target-issues')
before = helper.snapshot(repo)
helper.save(out / 'vm-source-before.json', before)
changed = subprocess.check_output(['git', 'diff', '--name-only', '--', '*.rs', '*.toml'], cwd=repo, text=True).splitlines()
original = 'cmd_oo::native_context::definition_exit::tests::original_definition_exit_restores_real_row_usage_and_frame_after_public_callback'
new = 'cmd_oo::original_bootstrap_namespace_tests::original_c91_bootstrap_objects_publish_their_actual_private_namespaces'
manifest = json.loads((repo / 'experiments/name-handling/results/vm-test-api-repairs/controls-results.json').read_text())
subset = [x['name'] for x in manifest if 2 <= x['ordinal'] <= 16]
record = {'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'scope': 'broad Rust source/manifests/toolchain guard including unlinked runtime files; not compiled closure or published HEAD claim', 'source_scope_count': len(before), 'current_working_tree_changed_rust_paths': changed, 'current_working_tree_changed_hashes': {name: before[name] for name in changed if name in before}, 'oracle_receipt_sha256': helper.digest(out.parent / 'repair-01/c91-receipt.json'), 'oracle_nonnull_compile_and_process_zero': True, 'temporary_owners': [], 'environment_recipe': '/tmp/name-env.sh', 'cache': environment['CARGO_TARGET_DIR'], 'controls': [], 'outcome': 'not_completed'}
start = time.monotonic()
signal.signal(signal.SIGTERM, helper.interrupted)
signal.signal(signal.SIGINT, helper.interrupted)
def check_currency():
    changed = helper.differences(before, helper.snapshot(repo))
    assert not changed, 'source changed during acceptance: '+repr(changed)
def control(binary, name, ordinal):
    check_currency()
    listed_path = out / ('vm-control-%02d-list.log' % ordinal)
    with listed_path.open('w') as log:
        code = helper.run([str(binary),name,'--exact','--list'],repo,environment,log,subprocess.STDOUT,30)
    listed = listed_path.read_text()
    assert code == 0 and listed.count(name+': test') == 1
    command = [str(binary),name,'--exact','--nocapture','--test-threads=1']
    log_path = out / ('vm-control-%02d.log' % ordinal)
    with log_path.open('w') as log:
        code = helper.run(command,repo,environment,log,subprocess.STDOUT,60)
    text = log_path.read_text()
    row = {'ordinal':ordinal,'name':name,'command':command,'exit_code':code,'log':log_path.name,'exactly_one_test_executed':bool(re.search(r'^running 1 test$',text,re.MULTILINE)),'summary':[line for line in text.splitlines() if line.startswith('test result:')]}
    record['controls'].append(row)
    print(json.dumps(row),flush=True)
    assert row['exactly_one_test_executed']
    check_currency()
    assert helper.digest(binary) == record['binary_sha256_before']
    return code
try:
    command = ['cargo','test','--offline','-p','tcl-vm','--lib','--no-run','-j','2','--message-format=json-render-diagnostics']
    record['build_command'] = command
    print('Current source frozen; offline VM lib-test rebuild started.',flush=True)
    with (out / 'vm-build.jsonl').open('w') as stdout, (out / 'vm-build.stderr.txt').open('w') as stderr:
        record['build_exit'] = helper.run(command,repo,environment,stdout,stderr,600)
    check_currency()
    if record['build_exit']:
        record['outcome'] = 'build_failed'
    else:
        artifacts = []
        for line in (out / 'vm-build.jsonl').read_text().splitlines():
            try: item = json.loads(line)
            except ValueError: continue
            if item.get('reason') == 'compiler-artifact' and item.get('target',{}).get('name') == 'tcl_vm' and item.get('executable'):
                artifacts.append(item['executable'])
        assert len(artifacts) == 1
        binary = pathlib.Path(artifacts[0])
        record.update(binary=str(binary),binary_sha256_before=helper.digest(binary))
        first_exit = control(binary,original,1)
        control(binary,new,2)
        if first_exit == 0:
            for ordinal,name in enumerate(subset,3): control(binary,name,ordinal)
        record['original_control_passed'] = first_exit == 0
        record['remaining_oo_subset_executed'] = first_exit == 0
        record['outcome'] = 'measured_original_and_public_namespace' if first_exit else 'measured_original_and_public_namespace_and_original_oo_subset'
        record['binary_sha256_after'] = helper.digest(binary)
except BaseException as error:
    record['exception'] = repr(error)
    record['outcome'] = 'interrupted_or_harness_failed'
    raise
finally:
    after = helper.snapshot(repo)
    helper.save(out / 'vm-source-after.json',after)
    record['changed_sources_after'] = helper.differences(before,after)
    record['elapsed_seconds'] = round(time.monotonic()-start,3)
    record['finished_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    helper.save(out / 'vm-execution-record.json',record)
    print('Receipt sealed: '+record['outcome']+'; source differences='+repr(record['changed_sources_after']),flush=True)
