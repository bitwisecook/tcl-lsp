from pathlib import Path
import argparse, datetime, gzip, hashlib, json, os, re, shlex, signal, subprocess, time

parser = argparse.ArgumentParser()
parser.add_argument('--root', default='/workspace/tcl-lsp')
parser.add_argument('--out', required=True)
parser.add_argument('--target', required=True)
args = parser.parse_args()
root = Path(args.root).resolve()
out = Path(args.out).resolve()
target = Path(args.target).resolve()
out.mkdir(parents=True, exist_ok=False)
target.mkdir(parents=True, exist_ok=True)
packet = Path(__file__).parent
plan = json.loads((packet / 'selector-plan.json').read_text())
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS='1', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0')
env.pop('TCL_LIBRARY', None)
env.pop('TCLLIBPATH', None)
trace_environment = {key: value for key, value in env.items() if key.startswith('TCL_LSP_TRACE') and value}

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(command, log, timeout, input=None, selected_env=None):
    started = time.monotonic()
    with (out / log).open('wb') as output:
        process = subprocess.Popen(command, cwd=root, env=selected_env or env, stdin=subprocess.PIPE if input is not None else subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        timed_out = False
        try:
            process.communicate(input=input, timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
    return {'argv': command, 'exit_code': process.returncode, 'timed_out': timed_out, 'elapsed_seconds': time.monotonic() - started, 'log': log, 'log_sha256': sha(out / log)}

try:
    provider_paths = set()
    providers = []
    for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']:
        tree = root / 'tmp' / ('tcl' + version)
        executable = tree / 'unix/tclsh'
        required = [executable, tree / 'library/init.tcl', tree / 'library/tcltest/tcltest.tcl']
        for path in required:
            if not path.is_file():
                raise ValueError('Missing original provider input: ' + str(path))
            provider_paths.add(path.resolve())
        provider_paths.update(p.resolve() for p in tree.rglob('*') if p.is_file() and (p.suffix in {'.c', '.h', '.tcl', '.test', '.tm', '.a', '.so', '.sh'} or '.so.' in p.name))
        oracle_key = {'8.4.20': 'TCL_LSP_TCLSH84', '8.5.19': 'TCL_LSP_TCLSH85', '8.6.18': 'TCL_LSP_TCLSH86', '9.0.4': 'TCL_LSP_TCLSH90', '9.1.0': 'TCL_LSP_TCLSH91'}[version]
        if oracle_key in env and Path(env[oracle_key]).resolve() != executable.resolve():
            raise ValueError('Foreign selected interpreter: ' + oracle_key)
        env[oracle_key] = str(executable)
        providers.append({'kind': 'C Tcl', 'version': version, 'executable': str(executable), 'original_library': str(tree / 'library'), 'original_files': {str(p): sha(p) for p in required}, 'probe': 'puts [list [info patchlevel] [info library]]\n'})
    jim_tree = root / 'tmp/jimtcl-oracle'
    jim_executable = jim_tree / 'jimsh'
    if 'TCL_LSP_JIMSH' in env and Path(env['TCL_LSP_JIMSH']).resolve() != jim_executable.resolve():
        raise ValueError('Foreign selected Jim interpreter')
    env.update(TCL_LSP_JIMSH=str(jim_executable), TCL_LSP_REQUIRE_ALL_TCL_ORACLES='1', TCL_LSP_REQUIRE_JIM_ORACLE='1')
    provider_paths.add(jim_executable.resolve())
    provider_paths.update(p.resolve() for p in jim_tree.rglob('*') if p.is_file() and (p.suffix in {'.c', '.h', '.tcl', '.test', '.tm', '.a', '.so', '.sh'} or '.so.' in p.name))
    providers.append({'kind': 'Jim', 'executable': str(jim_executable), 'executable_sha256': sha(jim_executable), 'probe': 'puts [list [info patchlevel]]\n', 'latest_check': json.loads((packet / 'provider-check.json').read_text())})
    for provider in providers:
        linked = subprocess.check_output(['ldd', provider['executable']], text=True)
        provider['linked_libraries'] = {str(Path(word).resolve()): sha(Path(word).resolve()) for line in linked.splitlines() for word in line.split() if word.startswith('/') and Path(word).is_file()}
        provider_paths.update(Path(p) for p in provider['linked_libraries'])


    def snapshot():
        paths = set(provider_paths)
        for family in ['rust', 'runtime', 'data', 'scripts', 'specs', 'experiments/name-handling', '.cargo']:
            directory = root / family
            paths.update(p.resolve() for p in directory.rglob('*') if p.is_file() and 'target' not in p.parts and '.git' not in p.parts and not p.resolve().is_relative_to(target) and not p.resolve().is_relative_to(out))
        for name in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'runtime/rust/Cargo.lock']:
            path = root / name
            if path.is_file():
                paths.add(path.resolve())
        if 'TCL_TOMMATH_DIR' in env:
            math_source = Path(env['TCL_TOMMATH_DIR']).resolve()
            paths.update(p.resolve() for p in math_source.rglob('*') if p.is_file())
        registry = Path(env.get('CARGO_HOME', '/workspace/.cargo')) / 'registry/src'
        paths.update(p.resolve() for p in registry.rglob('*') if p.is_file())
        for path in tuple(paths):
            if path.suffix == '.rs':
                for relative in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)', path.read_text(errors='strict')):
                    included = (path.parent / relative).resolve()
                    if included.is_file():
                        paths.add(included)
        return {str(path): sha(path) for path in sorted(paths)}


    def compressed(name, value):
        with gzip.open(out / name, 'wt') as stream:
            json.dump(value, stream, sort_keys=True)

    result = {'started_utc': now(), 'providers': providers, 'builds': [], 'runs': [], 'errors': [], 'passed': False, 'scope': plan['scope']}
    before = snapshot()
    compressed('source-before.json.gz', before)
    (out / 'selected-plan.json').write_text(json.dumps(plan, indent=2) + '\n')
    (out / 'start.json').write_text(json.dumps({'started_utc': result['started_utc'], 'runner_sha256': sha(Path(__file__)), 'source_count': len(before), 'plan_sha256': sha(packet / 'selector-plan.json'), 'patch_sha256': plan['patch_sha256'], 'original_automatic_error_case_count': plan['original_automatic_error_case_count'], 'target': str(target), 'rustc': subprocess.check_output(['rustc', '-Vv'], text=True), 'cargo': subprocess.check_output(['cargo', '-V'], text=True), 'closure': 'Rust/runtime/data/scripts/specs/experiments/Cargo, literal include files, original C5/Jim source/interpreters/libraries, linked libraries and registry sources'}, indent=2) + '\n')
    inputs = {}
    artifacts = {}
    cargo_artifact_rows = []
    try:
        if trace_environment:
            raise ValueError('Actual replay requires diagnostic traces disabled: ' + ', '.join(sorted(trace_environment)))
        latest = json.loads((packet / 'provider-check.json').read_text())
        if not latest['matches_current_upstream_head'] or sha(jim_executable) != latest['interpreter_sha256']:
            raise ValueError('Supplied Jim executable differs from retained latest-provider check')
        actual_revision = subprocess.check_output(['git', '-C', str(jim_tree), 'rev-parse', 'HEAD'], text=True).strip()
        if actual_revision != latest['local_revision']:
            raise ValueError('Supplied Jim source differs from retained latest-provider check')
        for index, provider in enumerate(providers):
            selected = env.copy()
            if provider['kind'] == 'C Tcl':
                selected['TCL_LIBRARY'] = provider['original_library']
            provider['metadata'] = run([provider['executable']], f'provider-{index + 1:02}.log', 30, input=provider['probe'].encode(), selected_env=selected)
            provider['selected_environment'] = {'TCL_LIBRARY': selected.get('TCL_LIBRARY'), 'TCLLIBPATH': selected.get('TCLLIBPATH')}
            if provider['metadata']['exit_code'] or provider['metadata']['timed_out']:
                raise ValueError('Original provider metadata failed')
            if provider['kind'] == 'C Tcl' and (out / provider['metadata']['log']).read_text().strip() != provider['version'] + ' ' + provider['original_library']:
                raise ValueError('Incorrect original init/library selection: ' + provider['version'])
            if provider['kind'] == 'Jim' and (out / provider['metadata']['log']).read_text().strip() != latest['reported_version']:
                raise ValueError('Incorrect original Jim version')
        groups = {data['cargo_target_name']: data['selectors'] for data in plan['builds'].values()}
        originals = json.loads((packet / 'original-selectors.json').read_text())
        for label, selected in originals.items():
            actual = plan['builds'][label]['selectors']
            if any(name not in actual for name in selected):
                raise ValueError('Original selected control was removed: ' + label)
        if sum(map(len, groups.values())) != 133 or plan['total_exact_selectors'] != 133 or sum(map(len, originals.values())) != 35 or plan['original_automatic_error_case_count'] != 20:
            raise ValueError('Incorrect retained original/new selector or original program count')
        baseline = json.loads((packet / 'baseline45-selectors.json').read_text())
        if sum(map(len, baseline.values())) != 45 or any(name not in plan['builds'][label]['selectors'] for label, names in baseline.items() for name in names):
            raise ValueError('Original45 VM baseline selector was removed')
        baseline52 = json.loads((packet / 'baseline52-selectors.json').read_text())
        if sum(map(len, baseline52.values())) != 55-3 or any(name not in plan['builds'][label]['selectors'] for label, names in baseline52.items() for name in names):
            raise ValueError('Previous52 VM selector was removed')
        baseline58 = json.loads((packet / 'baseline58-plan.json').read_text())
        if any(name not in plan['builds'][label]['selectors'] for label, group in baseline58['builds'].items() for name in group['selectors']):
            raise ValueError('Previous58 VM selector was removed')
        baseline55 = json.loads((packet / 'baseline55-selectors.json').read_text())
        if sum(map(len, baseline55.values())) != 55 or any(name not in plan['builds'][label]['selectors'] for label, names in baseline55.items() for name in names):
            raise ValueError('Previous55 VM selector was removed')
        baseline185 = json.loads((packet / 'baseline185-plan.json').read_text())
        if sum(len(group['selectors']) for group in baseline185['builds'].values()) != 59 or any(name not in plan['builds'][label]['selectors'] for label, group in baseline185['builds'].items() for name in group['selectors']):
            raise ValueError('A complete VM185 selector was removed or baseline count changed')
        baseline83 = json.loads((packet / 'baseline83-plan.json').read_text())
        if sum(len(group['selectors']) for group in baseline83['builds'].values()) != 83 or any(name not in plan['builds'][label]['selectors'] for label, group in baseline83['builds'].items() for name in group['selectors']):
            raise ValueError('A complete post-TEMP83 selector was removed or baseline count changed')
        baseline84 = json.loads((packet / 'baseline84-plan.json').read_text())
        if sum(len(group['selectors']) for group in baseline84['builds'].values()) != 84 or any(name not in plan['builds'][label]['selectors'] for label, group in baseline84['builds'].items() for name in group['selectors']):
            raise ValueError('A complete preflight84 selector was removed or baseline count changed')
        baseline87 = json.loads((packet / 'baseline87-plan.json').read_text())
        if sum(len(group['selectors']) for group in baseline87['builds'].values()) != 87 or any(name not in plan['builds'][label]['selectors'] for label, group in baseline87['builds'].items() for name in group['selectors']):
            raise ValueError('A complete87 predecessor selector was removed or baseline count changed')
        source_audit = json.loads((packet / 'source-selector-audit.json').read_text())
        if source_audit['added_count'] != 35 or len(source_audit['added']) != 35:
            raise ValueError('Added BigJim selector audit count changed')
        for selected in source_audit['added']:
            if selected['selector'] not in plan['builds']['lib']['selectors']:
                raise ValueError('A BigJim selector was removed: ' + selected['selector'])
            source = root / selected['path']
            if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Actual BigJim source selector is missing or ambiguous: ' + selected['selector'])
        predecessor122 = json.loads((packet / 'predecessor122-selector-plan.json').read_text())
        if any(plan['builds'][label]['selectors'][:len(group['selectors'])] != group['selectors'] for label, group in predecessor122['builds'].items()):
            raise ValueError('Original full122 ordered prefix selectors changed')
        baseline122 = json.loads((packet / 'baseline122-selector-plan.json').read_text())
        if sum(len(group['selectors']) for group in baseline122['builds'].values()) != 122 or any(plan['builds'][label]['selectors'][:len(group['selectors'])] != group['selectors'] for label, group in baseline122['builds'].items()):
            raise ValueError('Actual251 full122 baseline changed')
        warning_affected = json.loads((packet / 'warning-affected-selection-audit.json').read_text())
        warning_selectors = [row['selector'] for row in warning_affected['audited']]
        expected_added = warning_selectors + ['interp::namespace_context::tests::original_jim_expression_refusal_keeps_unmaterialized_namespace_context']
        if len(warning_selectors) != 8 or len(set(expected_added)) != 9 or plan['added_after122'] != expected_added or plan['builds']['lib']['selectors'][len(baseline122['builds']['lib']['selectors']):len(baseline122['builds']['lib']['selectors']) + 9] != expected_added:
            raise ValueError('Eight warning affected controls or actual Expr control changed')
        old_failures = json.loads((packet / 'prior251-failure-priority.json').read_text())['selectors']
        if len(old_failures) != 14 or len(set(old_failures)) != 14 or json.loads((packet / 'predecessor188-selector-plan.json').read_text())['execution_priority'] != old_failures + expected_added:
            raise ValueError('Actual251 fourteen failure priority or nine additions changed')
        predecessor131 = json.loads((packet / 'predecessor188-selector-plan.json').read_text())
        predecessor132 = json.loads((packet / 'predecessor132-selector-plan.json').read_text())
        expected_new = 'command::native_apply_original_tests::jim_lambda_temporary_publication_keeps_the_original_execution_namespace_separate'
        if sum(len(group['selectors']) for group in predecessor131['builds'].values()) != 131 or any(predecessor132['builds'][label]['selectors'] != group['selectors'] + ([expected_new] if label == 'lib' else []) for label, group in predecessor131['builds'].items()) or plan['added_after131'] != [expected_new]:
            raise ValueError('Exact complete131 prefix or sole lambda addition changed')
        actual260_failures = json.loads((packet / 'actual260-failure-priority.json').read_text())['selectors']
        expected_priority = list(dict.fromkeys(actual260_failures + [expected_new] + predecessor131['execution_priority']))
        if len(actual260_failures) != 10 or len(set(actual260_failures)) != 10 or not set(actual260_failures).issubset(set(name for group in predecessor131['builds'].values() for name in group['selectors'])) or predecessor132['execution_priority'] != expected_priority:
            raise ValueError('Actual260 ten-failure then new control execution priority changed')
        expected_text = 'cmd_namespace::original_jim_namespace_text_result_tests::qualifiers_selects_the_actual_shared_empty_without_changing_root_or_tail_identity'
        if sum(len(group['selectors']) for group in predecessor132['builds'].values()) != 132 or any(plan['builds'][label]['selectors'] != group['selectors'] + ([expected_text] if label == 'lib' else []) for label, group in predecessor132['builds'].items()) or plan['added_after132'] != [expected_text]:
            raise ValueError('Exact complete132 ordered prefix or sole SharedEmpty addition changed')
        failures269 = json.loads((packet / 'actual269-failure-priority.json').read_text())['selectors']
        if len(failures269) != 6 or len(set(failures269)) != 6 or not set(failures269).issubset(set(name for group in predecessor132['builds'].values() for name in group['selectors'])) or plan['execution_priority'] != list(dict.fromkeys(failures269 + [expected_text] + predecessor132['execution_priority'])):
            raise ValueError('Actual269 six-failure then new control priority changed')
        current_audit = json.loads((packet / 'current-source-selector-audit.json').read_text())
        expected_pairs = {(label, selector) for label, group in plan['builds'].items() for selector in group['selectors']}
        if current_audit['count'] != 133 or len(current_audit['tests']) != 133 or {(row['build'], row['selector']) for row in current_audit['tests']} != expected_pairs:
            raise ValueError('Current full133 source audit membership changed')
        for selected in current_audit['tests']:
            source = root / selected['path']
            if not selected['test_attribute'] or sha(source) != selected['source_sha256'] or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Current full133 source binding changed: ' + selected['selector'])
        required_adopted = json.loads((packet / 'predecessor188-required-adopted-afterimages.json').read_text())
        prior_required = json.loads((packet / 'predecessor177-required-adopted-afterimages.json').read_text())
        previous = {row['path']: row for row in prior_required['files']}
        latest_required = {row['path']: row for row in required_adopted['files']}
        updates = required_adopted['sequential_updates']
        update_paths = {row['path'] for row in updates}
        expected_updates = {
            'rust/tcl-vm/src/interp/native_namespace_names.rs': '42cf65c1d40223b2f0d4e2b0c015e26e98f6138027b873f7ff41372b641e1393',
            'runtime/rust/src/interp/native_jim_lookup.rs': 'f9660e506da82d669d8581743d8c275c0604b292805d5e275f513714c08cbc79',
        }
        if len(previous) != 13 or len(required_adopted['files']) != 14 or len(latest_required) != 14 or len(updates) != 2 or update_paths != set(expected_updates) or not set(previous).issubset(latest_required):
            raise ValueError('Required188 sequential source inventory changed')
        for row in updates:
            prior = previous.get(row['path'])
            if row['source_patch_sha256'] != expected_updates[row['path']] or row['previously_required'] != (prior is not None) or (prior is not None and prior['after_sha256'] != row['before_sha256']) or latest_required[row['path']]['after_sha256'] != row['after_sha256']:
                raise ValueError('Required188 source successor is not exact: ' + row['path'])
        for name, row in previous.items():
            if name not in update_paths and latest_required[name] != row:
                raise ValueError('Earlier176/177 required source was waived: ' + name)
        current_required = json.loads((packet / 'required-adopted-afterimages.json').read_text())
        older = {row['path']: row for row in required_adopted['files']}
        current = {row['path']: row for row in current_required['files']}
        updates132 = current_required['sequential_updates_132']
        expected_paths = {'rust/tcl-vm/src/command.rs', 'rust/tcl-vm/src/command/native_apply_original_tests.rs'}
        if len(older) != 14 or len(current_required['files']) != 15 or len(current) != 15 or len(updates132) != 2 or {row['path'] for row in updates132} != expected_paths or not set(older).issubset(current):
            raise ValueError('Exact132 source successor inventory changed')
        for row in updates132:
            prior = older.get(row['path'])
            if row['source_patch_sha256'] != '0f0f7a18305fa2a96c0d0f88fa578c32a654ca623f8496345556b8451bae3184' or row['previously_required'] != (prior is not None) or (prior is not None and prior['after_sha256'] != row['before_sha256']) or current[row['path']]['after_sha256'] != row['after_sha256']:
                raise ValueError('Lambda source successor not exact: ' + row['path'])
        for name, row in older.items():
            if name not in expected_paths and current[name] != row:
                raise ValueError('Earlier required source waived: ' + name)
        post269 = json.loads((packet / 'post269-required-afterimages.json').read_text())
        post_by_path = {row['path']: row for row in post269['files']}
        if len(post269['files']) != 13 or set(post_by_path) != {'rust/tcl-vm/src/interp/oo_cleanup/active_frame/tests.rs', 'rust/tcl-syntax/src/naming/native.rs', 'rust/tcl-syntax/src/naming.rs', 'runtime/rust/src/cmd_namespace/native_jim_parent_tests.rs', 'runtime/rust/src/cmd_namespace.rs', 'rust/tcl-vm/src/interp/selected_frame/tests.rs', 'runtime/rust/tests/data/native_jim_namespace_text/getter-identity-20261015.tsv', 'rust/tcl-vm/src/cmd_namespace.rs', 'rust/tcl-vm/src/command/native_apply_original_tests.rs', 'runtime/rust/src/cmd_namespace/native_jim_text_result_tests.rs', 'runtime/rust/src/interp/native_jim_namespace.rs', 'runtime/rust/src/cmd_namespace/native_jim_delete_tests.rs', 'runtime/rust/tests/data/native_jim_namespace_parent/getter-identity-20261015.tsv'} or {row['patch_sha256'] for row in post269['source_packets']} != {'8a43b0cd228a86f009b15b54c7ed0b662d9df8dd51235f7a77b6ea142dd1cb9e', '9069063743862877780908bacc7fbf6292bb592ad8d6db0813955221e492705a', 'dae0d44b88a8f4d681a6aeec2751e6fecbc5697ec56aa4f879fce9c717ee1392', '6b6906699b381618668e235482c9cedef52d72ca4c1a1e6260c56c32afdc48c6', '4ba6185030876386254a6c1b01fd721b20b56a5b6b420612543216de876243d9', '6435bdae42eb2857fd04b407ab4a1adca185267ba16c247f9d673bc2be0efb7e'}:
            raise ValueError('Exact post269 adopted source inventory changed')
        predecessor132_audit = json.loads((packet / 'predecessor132-current-source-selector-audit.json').read_text())
        for row in post269['files']:
            chain = row['chain']
            if not chain or chain[0]['before_sha256'] != row['before_sha256'] or chain[-1]['after_sha256'] != row['after_sha256'] or any(a['after_sha256'] != b['before_sha256'] for a, b in zip(chain, chain[1:])) or any(leg['source_patch_sha256'] not in {p['patch_sha256'] for p in post269['source_packets']} for leg in chain):
                raise ValueError('Post269 source succession is not exact: ' + row['path'])
            prior = [r for r in predecessor132_audit['tests'] if r['path'] == row['path']]
            if any(r['source_sha256'] != row['before_sha256'] for r in prior) or sha(root / row['path']) != row['after_sha256']:
                raise ValueError('Post269 actual source before/after mismatch: ' + row['path'])
        def required_current_hash(path, historical_hash):
            successor = post_by_path.get(path)
            if successor is None:
                return historical_hash
            if not successor['previously_required'] or successor['before_sha256'] != historical_hash:
                raise ValueError('Required historical image is not exact predecessor: ' + path)
            return successor['after_sha256']
        for row in current_required['files']:
            source = root / row['path']
            if not source.is_file() or sha(source) != required_current_hash(row['path'], row['after_sha256']):
                raise ValueError('Required132 adopted source changed: ' + row['path'])
        successor211 = json.loads((packet / 'required-source211-afterimages.json').read_text())
        expected211 = {'rust/tcl-vm/src/interp.rs', 'rust/tcl-vm/src/exec.rs', 'rust/tcl-vm/src/cmd_namespace.rs'}
        if successor211['source_patch_sha256'] != '7ee108826b1fa6fd31218a61b8577f7422c1131a6a0897f87f9bcad439569c3e' or len(successor211['files']) != 3 or {row['path'] for row in successor211['files']} != expected211 or expected211.intersection(current):
            raise ValueError('Exact adopted211 source successor set changed')
        predecessor207_audit = json.loads((packet / 'predecessor207-current-source-selector-audit.json').read_text())
        for row in successor211['files']:
            prior = [selected for selected in predecessor207_audit['tests'] if selected['path'] == row['path']]
            if any(selected['source_sha256'] != row['before_sha256'] for selected in prior) or sha(root / row['path']) != required_current_hash(row['path'], row['after_sha256']):
                raise ValueError('Adopted211 exact before/after source changed: ' + row['path'])
        automatic_cases = root / 'rust/tcl-test-support/src/automatic_errors.rs'
        if sha(automatic_cases) != plan['original_automatic_error_case_file_sha256']:
            raise ValueError('Original automatic error program source changed')
        builds = [(label, data['argv'] + ['--offline', '--message-format=json', '-j1'])
                  for label, data in plan['builds'].items()]
        if any(not names or len(names) != len(set(names)) for names in groups.values()):
            raise ValueError('Empty or duplicate selection')
        for label, command in builds:
            built = run(command, label + '-build.jsonl', 1800)
            result['builds'].append(built)
            if built['exit_code'] or built['timed_out']:
                raise ValueError('Build failed: ' + label)
            for line in (out / built['log']).read_text().splitlines():
                try:
                    row = json.loads(line)
                except ValueError:
                    continue
                if row.get('reason') == 'compiler-artifact':
                    cargo_artifact_rows.append(row)
                    if row.get('profile', {}).get('test') and row.get('executable') and row['target']['name'] in groups:
                        artifacts[row['target']['name']] = Path(row['executable'])
        if set(artifacts) != set(groups):
            raise ValueError('Missing actual compiled binary: ' + str(set(groups) - set(artifacts)))
        actual_dep_info = set()
        for row in cargo_artifact_rows:
            candidates = set()
            for reported in row['filenames']:
                artifact = Path(reported)
                if row['target']['kind'] == ['custom-build']:
                    digest = artifact.parent.name.rsplit('-', 1)[-1]
                    candidates.add(artifact.parent / ('build_script_build-' + digest + '.d'))
                else:
                    stem = artifact.stem if artifact.suffix in {'.rlib', '.rmeta', '.so', '.a', '.dylib', '.dll', '.exe'} else artifact.name
                    if stem.startswith('lib') and row['target']['kind'] != ['bin']:
                        stem = stem[3:]
                    candidates.add(artifact.parent / (stem + '.d'))
            existing = {path.resolve() for path in candidates if path.is_file()}
            if not existing:
                raise ValueError('Missing dependency info for actual Cargo artifact: ' + row['target']['name'])
            actual_dep_info.update(existing)
        (out / 'actual-cargo-artifacts.json').write_text(json.dumps(cargo_artifact_rows, indent=2) + '\n')
        dep_info, unresolved, uncovered = {}, [], []
        for path in sorted(actual_dep_info):
            dep_info[str(path)] = sha(path)
            for line in path.read_text(errors='strict').replace('\\\n', '').splitlines():
                if line.startswith('#') or ': ' not in line:
                    continue
                for word in shlex.split(line.split(': ', 1)[1]):
                    original = Path(word)
                    if not original.is_absolute():
                        original = next((candidate for candidate in [root / 'runtime/rust' / original, root / original] if candidate.is_file()), original)
                    if not original.is_file():
                        unresolved.append({'dep_info': str(path), 'input': word})
                        continue
                    original = original.resolve()
                    inputs[str(original)] = sha(original)
                    if str(original) not in before and not original.is_relative_to(target):
                        uncovered.append(str(original))
        if not dep_info or not inputs:
            raise ValueError('Empty compiled dependency closure')
        result['artifacts'] = {str(path): sha(path) for path in artifacts.values()}
        (out / 'compiled-closure.json').write_text(json.dumps({'inputs': inputs, 'dep_info': dep_info, 'unresolved': unresolved, 'uncovered': sorted(set(uncovered)), 'artifacts': result['artifacts']}, indent=2) + '\n')
        if unresolved or uncovered:
            raise ValueError('Compiled source coverage incomplete')
        for label, binary in artifacts.items():
            listed = run([str(binary), '--list'], label + '.list', 30)
            if listed['exit_code'] or listed['timed_out']:
                raise ValueError('Actual binary list failed')
            names = [line.removesuffix(': test') for line in (out / listed['log']).read_text().splitlines() if line.endswith(': test')]
            missing = [name for name in groups[label] if name not in names]
            if not names or missing:
                raise ValueError('Missing exact selectors: ' + str(missing))
        result['selected_tests'] = groups
        original_order = [(label, name) for label, names in groups.items() for name in names]
        by_selector = {name: label for label, name in original_order}
        priority = plan['execution_priority']
        if len(by_selector) != 133 or any(name not in by_selector for name in priority):
            raise ValueError('Execution priority is not part of exact full133 union')
        execution_order = [(by_selector[name], name) for name in priority] + [(label, name) for label, name in original_order if name not in priority]
        if len(execution_order) != 133 or set(execution_order) != set(original_order):
            raise ValueError('Execution ordering removed or duplicated a selected control')
        (out / 'exact-plan.json').write_text(json.dumps([{'binary': str(artifacts[label]), 'selector': name} for label, name in execution_order], indent=2) + '\n')
        for label, name in execution_order:
            print('Starting ' + label + ' ' + name, flush=True)
            row = run([str(artifacts[label]), name, '--exact', '--nocapture', '--test-threads=1'], f'test-{len(result["runs"]) + 1:02}.log', 180)
            content = (out / row['log']).read_text()
            row.update(selector=name, binary_kind=label, running_counts=re.findall(r'running (\d+) tests?', content), summaries=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed;', content))
            row['passed'] = row['exit_code'] == 0 and not row['timed_out'] and row['running_counts'] == ['1'] and row['summaries'] == [('ok', '1', '0')]
            result['runs'].append(row)
            (out / 'runs-in-progress.json').write_text(json.dumps(result['runs'], indent=2) + '\n')
        result['passed'] = len(result['runs']) == sum(map(len, groups.values())) and all(row['passed'] for row in result['runs'])
    except Exception as error:
        result['errors'].append(str(error))
    finally:
        after = snapshot()
        compressed('source-after.json.gz', after)
        result['source_drift'] = {path: [before.get(path), after.get(path)] for path in set(before) | set(after) if before.get(path) != after.get(path)}
        result['compiled_input_drift'] = {path: [digest, sha(Path(path)) if Path(path).is_file() else None] for path, digest in inputs.items() if not Path(path).is_file() or sha(Path(path)) != digest}
        result['artifact_drift'] = {path: [digest, sha(Path(path)) if Path(path).is_file() else None] for path, digest in result.get('artifacts', {}).items() if not Path(path).is_file() or sha(Path(path)) != digest}
        result.update(attempted=len(result['runs']), passed_count=sum(row['passed'] for row in result['runs']), failed_count=sum(not row['passed'] for row in result['runs']), finished_utc=now())
        if result['source_drift'] or result['compiled_input_drift'] or result['artifact_drift']:
            result['passed'] = False
        (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps({key: result[key] for key in ['passed', 'attempted', 'passed_count', 'failed_count', 'errors']}), flush=True)
    raise SystemExit(0 if result['passed'] else 1)
except Exception as startup_error:
    import traceback
    (out / 'startup-failure.log').write_text(traceback.format_exc())
    if not (out / 'result.json').exists():
        (out / 'result.json').write_text(json.dumps({
            'passed': False, 'attempted': 0, 'passed_count': 0,
            'errors': [str(startup_error)], 'scope': plan['scope'],
            'finished_utc': now(), 'startup_failed_before_complete_capture': True,
        }, indent=2) + '\n')
    raise SystemExit(1)
