#!/usr/bin/env python3
"""Strict three-target library9 capture. All three xtask controls remain blocked/owed."""
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
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package-manifest', type=Path, required=True)
    parser.add_argument('--package-sha256', required=True)
    parser.add_argument('--binding-manifest', type=Path, required=True)
    parser.add_argument('--binding-sha256', required=True)
    parser.add_argument('--list-only', action='store_true')
    args = parser.parse_args()
    if sys.flags.optimize or hashlib.sha256(args.package_manifest.read_bytes()).hexdigest() != args.package_sha256:
        raise RuntimeError('Optimised Python or changed prepared package')
    package = json.loads(args.package_manifest.read_text())
    for name, record in package['files'].items():
        with Path(record['path']).open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != record['sha256']:
                raise RuntimeError('Package pin changed: ' + name)
    if Path(package['files']['launcher']['path']).resolve() != Path(__file__).resolve():
        raise RuntimeError('Another library9 launcher is pinned')
    path = Path(package['files']['support']['path'])
    spec = importlib.util.spec_from_file_location('_library9_launch_support', path)
    helper = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = helper
    spec.loader.exec_module(helper)
    ctx, plans, owed = helper.context(package)
    helper.require(helper.sha(args.binding_manifest) == args.binding_sha256, 'Actual binding manifest changed')
    manifest = helper.read(args.binding_manifest)
    helper.require(manifest['status'] == 'ACTUAL_LIBRARY9_BOUND_XTASK3_BLOCKED' and manifest['selected'] == 9 and manifest['xtask_controls_still_owed'] == 3, 'Wrong actual available scope')
    helper.require(manifest['prepared_package_sha256'] == args.package_sha256 and manifest['binder_source_sha256'] == package['files']['binder']['sha256'], 'Binding uses another preparation/binder')
    helper.require(args.binding_manifest.resolve().parent.parent == helper.DEST, 'Foreign actual binding directory')
    for name, record in manifest['files'].items():
        helper.require(Path(record['path']).resolve() == args.binding_manifest.resolve().parent / name and helper.sha(record['path']) == record['sha256'], 'Actual bound metadata changed')
    bound = helper.read(manifest['files']['binding.json']['path'])
    bound_plans = helper.read(manifest['files']['bound-plans.json']['path'])
    helper.require(bound['package_manifest']['sha256'] == args.package_sha256 and Path(bound['package_manifest']['path']).resolve() == args.package_manifest.resolve(), 'Foreign prepared package')
    helper.require(bound['status'] == manifest['status'] and bound['exact_target_counts'] == helper.COUNTS, 'Wrong actual bound scope')
    helper.require(bound['standard930_admission'] == package['files']['standard930_admission'] and bound['xtask_controls_still_owed'] == owed and bound['failed_xtask_build'] == package['files']['failed_xtask_build'], 'Original current/blocked obligations changed')
    library = helper.bind_library(ctx, bound['library_build']['build_dir'])
    helper.require(library == bound['library_build'], 'Actual library metadata differs from bound receipt')
    helper.require(set(bound_plans) == set(plans), 'Bound library target set changed')
    for target in helper.GROUP_ORDER:
        expected = json.loads(json.dumps(plans[target]))
        expected['freeze_binding'] = {'library_build': library, 'selector_count': helper.COUNTS[target], 'selectors': [row['selector'] for row in plans[target]['execution_order']]}
        helper.require(bound_plans[target] == expected, 'Binding changed selected rows/source/guards: ' + target)

    def check_current():
        helper.require(helper.sha(args.package_manifest) == args.package_sha256 and helper.sha(args.binding_manifest) == args.binding_sha256, 'Package/binding changed')
        for record in manifest['files'].values():
            helper.require(helper.sha(record['path']) == record['sha256'], 'Actual binding file changed')
        return helper.check_current(package, ctx, library)

    preflight = check_current()
    helper.require(preflight == bound['pre_post_binding'], 'Current eight-artifact/source binding differs')
    stem = 'extra-library9-listing-results' if args.list_only else 'extra-library9-results'
    out = helper.DEST / stem
    receipt_path = helper.DEST / (stem + '-invocation.json')
    helper.require(not out.exists() and not receipt_path.exists(), 'Refuse to overwrite prior capture')
    receipt = {'status': 'PREFLIGHT_PASS_LIBRARY9_XTASK3_BLOCKED', 'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'list_only': args.list_only, 'selected': 9, 'all_original9_library_rows_and_guards_unchanged': True, 'all_three_xtask_controls_still_owed': owed, 'package_manifest': str(args.package_manifest), 'package_sha256': args.package_sha256, 'binding_manifest': str(args.binding_manifest), 'binding_sha256': args.binding_sha256, 'preflight': preflight, 'scope': 'Exactly9 selectors in three canonical target groups; actual combined3Lib manifest, all five standard+three extra artifact/source/provider pre/post. All3 xtask controls remain blocked/owed. Not standard930 replay and not ordinary acceptance.', 'ordinary_acceptance': False, 'invocations': []}
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
                artifact_record = library['files']['artifacts']
                helper.require(helper.sha(artifact_record['path']) == artifact_record['sha256'], 'Actual combined artifact manifest changed')
                argv = [sys.executable, ctx['pins']['canonical_runner']['path'], '--manifest', str(selected), '--manifest-sha256', helper.sha(selected), '--artifacts', artifact_record['path'], '--root', str(helper.ROOT), '--out', str(out / target), '--group', target]
                if args.list_only:
                    argv.append('--list-only')
                attempt.update(argv=argv, invoked=True, selected_manifest_sha256=helper.sha(selected), actual_combined_artifact_manifest=artifact_record, target_artifact=library['artifacts'][target])
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
            # Earlier target failure never drops a later required target.
    finally:
        try:
            receipt['postflight'] = check_current()
            helper.require(receipt['postflight'] == preflight, 'Source/provider/eight-artifact postflight changed')
        except Exception as error:
            receipt['postflight_error'] = repr(error)
            code = 1
        receipt.update(completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), attempted_groups=len(receipt['invocations']), wrapper_exit_code=code, status='LIBRARY9_CAPTURE_COMPLETED_POSTFLIGHT_PASS_XTASK3_OWED' if code == 0 else 'LIBRARY9_CAPTURE_OR_POSTFLIGHT_FAILED_XTASK3_OWED')
        receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, indent=2), flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
