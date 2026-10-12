import datetime, hashlib, importlib.util, json, os, pathlib, re, signal, subprocess, time
repo = pathlib.Path('/workspace/tcl-lsp')
out = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('capture_helpers', str(repo / 'experiments/name-handling/results/vm-oo-root-namespace-creation/repair-01/capture-helpers.py'))
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
source = repo / 'rust/tcl-vm/src/cmd_oo.rs'
assert helper.digest(source) == 'a93fa3aa3f9d0b583bdef67a5b1edb61ab5de29e6d88c013d5ce118982a9cd83'
assert helper.digest(repo / 'rust/tcl-vm/src/cmd_oo/native_context/definition_exit.rs') == '2bd96778da4a50142af56b5cd968d7d058c45f2d69c051114a8b8d6dcf2ab56e'
assert helper.digest(repo / 'rust/tcl-vm/src/cmd_oo/native_context/definition_exit/tests.rs') == '2b2f3fe4b984e9f023a0c8655a39e804630e9ae2bd491e39af36baf7a944cf4a'
assert not (out / 'vm-execution-record.json').exists()
assert os.environ['CARGO_HOME'] == str(repo / 'tmp/name-tools/cargo')
environment = os.environ.copy()
environment['CARGO_TARGET_DIR'] = os.environ.get('CARGO_TARGET_DIR', str(repo / 'tmp/name-target-issues'))
before = helper.snapshot(repo)
helper.save(out / 'vm-source-before.json', before)
changed = subprocess.check_output(['git', 'diff', '--name-only', '--', '*.rs', '*.toml'], cwd=repo, text=True).splitlines()
original = 'cmd_oo::native_context::definition_exit::tests::original_definition_worker_and_self_rewrite_restore_the_same_selected_row'
new = 'cmd_oo::original_bootstrap_namespace_tests::original_c91_bootstrap_objects_publish_their_actual_private_namespaces'
constructor = 'cmd_oo::native_context::definition_exit::tests::original_definition_exit_restores_real_row_usage_and_frame_after_public_callback'
software = 'cmd_oo::native_context::definition_exit::tests::original_definition_error_frame_retains_retired_result_storage_refusal_through_cleanup'
record = {'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'scope': 'broad Rust source/manifests/toolchain guard including unlinked runtime files; not compiled closure or published HEAD claim', 'source_scope_count': len(before), 'current_working_tree_changed_rust_paths': changed, 'current_working_tree_changed_hashes': {name: before[name] for name in changed if name in before}, 'oracle_receipt_sha256': helper.digest(repo / 'experiments/name-handling/results/vm-oo-root-namespace-creation/repair-01/c91-receipt.json'), 'oracle_nonnull_compile_and_process_zero': True, 'temporary_owners': [], 'environment_recipe': '/tmp/name-env.sh', 'cache': environment['CARGO_TARGET_DIR'], 'controls': [], 'outcome': 'not_completed'}
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
        control(binary,constructor,2)
        control(binary,new,3)
        control(binary,software,4)
        record['original_control_passed'] = first_exit == 0
        record['remaining_oo_subset_executed'] = False
        record['outcome'] = 'measured_four_exact_checked_reporting_acceptance_controls'
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
