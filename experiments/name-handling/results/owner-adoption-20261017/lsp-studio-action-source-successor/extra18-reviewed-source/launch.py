"""Root-only full18 replay with complete standard5+extra4 source/provider binding."""
from pathlib import Path
import argparse
import datetime
import hashlib
import importlib.util
import json
import subprocess
import sys
sys.dont_write_bytecode = True


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--package-manifest', type=Path, required=True)
    ap.add_argument('--package-sha256', required=True)
    ap.add_argument('--binding-manifest', type=Path, required=True)
    ap.add_argument('--binding-sha256', required=True)
    ap.add_argument('--list-only', action='store_true')
    args = ap.parse_args()
    if sys.flags.optimize or hashlib.sha256(args.package_manifest.read_bytes()).hexdigest() != args.package_sha256:
        raise RuntimeError('Optimised Python or changed package')
    package = json.loads(args.package_manifest.read_text())
    record = package['files']['support']
    if hashlib.sha256(Path(record['path']).read_bytes()).hexdigest() != record['sha256']:
        raise RuntimeError('Support changed')
    spec = importlib.util.spec_from_file_location('_extra18_launch_support', record['path'])
    helper = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = helper
    spec.loader.exec_module(helper)
    helper.checked_files(package)
    helper.require(Path(package['files']['launcher']['path']).resolve() == Path(__file__).resolve(), 'Another launcher pinned')
    ctx, plans = helper.context(package)
    helper.require(helper.sha(args.binding_manifest) == args.binding_sha256, 'Binding manifest changed')
    manifest = helper.read(args.binding_manifest)
    destination = Path(ctx['admission']['destination']).resolve()
    helper.require(args.binding_manifest.resolve().parent.parent == destination, 'Foreign binding directory')
    helper.require(manifest['status'] == 'ACTUAL_EXTRA18_BOUND_ALL_FOUR_TARGETS' and manifest['selected'] == 18, 'Wrong actual full18 scope')
    helper.require(manifest['prepared_package_sha256'] == args.package_sha256 and manifest['binder_source_sha256'] == package['files']['binder']['sha256'], 'Binding uses another preparation/binder')
    for name, record in manifest['files'].items():
        helper.require(Path(record['path']).resolve() == args.binding_manifest.resolve().parent / name and helper.sha(record['path']) == record['sha256'], 'Bound metadata changed')
    bound = helper.read(manifest['files']['binding.json']['path'])
    helper.require(bound['package_manifest'] == {'path': str(args.package_manifest.resolve()), 'sha256': args.package_sha256}, 'Foreign prepared package')
    helper.require(bound['status'] == manifest['status'] and bound['selected'] == 18 and bound['counts'] == helper.COUNTS and bound['standard_admission'] == package['files']['standard_admission'], 'Bound scope/standard source changed')
    helper.require(helper.read(manifest['files']['bound-plans.json']['path']) == plans, 'Bound vectors/source/guards changed')
    extras = bound['extra_builds']
    artifacts = dict(extras['libs']['artifacts'], **extras['xtask']['artifacts'])
    helper.require(helper.read(manifest['files']['artifacts.json']['path']) == artifacts, 'Combined LIB/BIN artifact manifest changed')

    def check_current():
        helper.require(helper.sha(args.package_manifest) == args.package_sha256 and helper.sha(args.binding_manifest) == args.binding_sha256, 'Package/binding changed')
        for record in manifest['files'].values():
            helper.require(helper.sha(record['path']) == record['sha256'], 'Bound metadata changed during capture')
        return helper.check_current(package, ctx, extras)

    preflight = check_current()
    helper.require(preflight == bound['pre_post_binding'], 'Actual nine-ELF/source/provider binding differs')
    stem = 'extra18-listing-results' if args.list_only else 'extra18-results'
    out = destination / stem
    receipt_path = destination / (stem + '-invocation.json')
    helper.require(not out.exists() and not receipt_path.exists(), 'Refuse to overwrite evidence')
    receipt = {'status': 'PREFLIGHT_PASS_EXTRA18', 'selected': 18, 'list_only': args.list_only, 'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'package_sha256': args.package_sha256, 'binding_sha256': args.binding_sha256, 'preflight': preflight, 'all12_original_rows_and_guards_unchanged': True, 'ordinary_acceptance': False, 'invocations': []}
    with receipt_path.open('x') as stream:
        stream.write(json.dumps(receipt, indent=2) + '\n')
    code = 1
    try:
        out.mkdir()
        code = 0
        for target in helper.GROUP_ORDER:
            attempt = {'target': target, 'selected': helper.COUNTS[target], 'invoked': False, 'wrapper_exit_code': 1}
            try:
                plan = plans[target]
                subset = {key: plan[key] for key in helper.GUARDS}
                subset.update(group=target, execution_order=plan['execution_order'], groups=plan['groups'], expected_group_counts=plan['expected_group_counts'], total_exact_selectors=helper.COUNTS[target])
                selected = out / (target + '-selection.json')
                selected.write_text(json.dumps(subset, indent=2) + '\n')
                argv = [sys.executable, ctx['pins']['canonical_runner']['path'], '--manifest', str(selected), '--manifest-sha256', helper.sha(selected), '--artifacts', manifest['files']['artifacts.json']['path'], '--root', str(helper.ROOT), '--out', str(out / target), '--group', target]
                if args.list_only:
                    argv.append('--list-only')
                attempt.update(argv=argv, invoked=True, selected_manifest_sha256=helper.sha(selected), target_artifact=artifacts[target])
                attempt['wrapper_exit_code'] = subprocess.run(argv, cwd=helper.ROOT, check=False).returncode
            except Exception as error:
                attempt['error'] = repr(error)
            receipt['invocations'].append(attempt)
            try:
                (out / (target + '-wrapper.json')).write_text(json.dumps(attempt, indent=2) + '\n')
            except Exception as error:
                attempt['wrapper_receipt_write_error'] = repr(error)
                code = 1
            if attempt['wrapper_exit_code'] != 0:
                code = 1
    finally:
        try:
            receipt['postflight'] = check_current()
            helper.require(receipt['postflight'] == preflight, 'Nine-ELF/source/provider postflight changed')
        except Exception as error:
            receipt['postflight_error'] = repr(error)
            code = 1
        receipt.update(completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), attempted_groups=len(receipt['invocations']), wrapper_exit_code=code, status='EXTRA18_CAPTURE_COMPLETED_POSTFLIGHT_PASS' if code == 0 else 'EXTRA18_CAPTURE_OR_POSTFLIGHT_FAILED')
        receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, indent=2), flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
