#!/usr/bin/env python3
"""Bind the actual successful library9 scope; never build, list or execute tests."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import sys

sys.dont_write_bytecode = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package-manifest', type=Path, required=True)
    parser.add_argument('--package-sha256', required=True)
    parser.add_argument('--libs-build', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if sys.flags.optimize or hashlib.sha256(args.package_manifest.read_bytes()).hexdigest() != args.package_sha256:
        raise RuntimeError('Optimised Python or changed prepared package')
    package = json.loads(args.package_manifest.read_text())
    for name, record in package['files'].items():
        with Path(record['path']).open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != record['sha256']:
                raise RuntimeError('Package pin changed: ' + name)
    if Path(package['files']['binder']['path']).resolve() != Path(__file__).resolve():
        raise RuntimeError('Another binder is pinned')
    helper_path = Path(package['files']['support']['path'])
    spec = importlib.util.spec_from_file_location('_library9_bind_support', helper_path)
    helper = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = helper
    spec.loader.exec_module(helper)
    ctx, plans, owed = helper.context(package)
    pre = ctx['check_current']()
    library = helper.bind_library(ctx, args.libs_build)
    full = helper.check_current(package, ctx, library)
    helper.require(full['standard'] == pre, 'Source/standard artifacts changed while binding')
    output = args.output.resolve()
    helper.require(output.parent == helper.DEST and not output.exists(), 'Need a new owned current-parent binding directory')
    # Output is created only after the real successful build, all complete
    # before/after source rows and all eight actual artifacts have been joined.
    bound_plans = json.loads(json.dumps(plans))
    for target, plan in bound_plans.items():
        plan['freeze_binding'] = {'library_build': library, 'selector_count': helper.COUNTS[target], 'selectors': [row['selector'] for row in plan['execution_order']]}
    binding = {'status': 'ACTUAL_LIBRARY9_BOUND_XTASK3_BLOCKED', 'selected': 9, 'package_manifest': {'path': str(args.package_manifest.resolve()), 'sha256': args.package_sha256}, 'standard930_admission': package['files']['standard930_admission'], 'standard_actual_build': ctx['pins']['actual_build'], 'full39030_inputs': ctx['pins']['actual_inputs'], 'standard_five_artifacts': ctx['pins']['actual_artifacts'], 'prepared_plans': package['files']['prepared_plans'], 'library_build': library, 'exact_target_counts': helper.COUNTS, 'xtask_controls_still_owed': owed, 'failed_xtask_build': package['files']['failed_xtask_build'], 'pre_post_binding': full, 'listing_and_test_execution': False, 'ordinary_acceptance': False}
    output.mkdir()
    (output / 'bound-plans.json').write_text(json.dumps(bound_plans, indent=2) + '\n')
    (output / 'binding.json').write_text(json.dumps(binding, indent=2) + '\n')
    manifest = {'status': binding['status'], 'selected': 9, 'xtask_controls_still_owed': 3, 'binder_source_sha256': package['files']['binder']['sha256'], 'prepared_package_sha256': args.package_sha256, 'files': {name: {'path': str(output / name), 'sha256': helper.sha(output / name)} for name in ['bound-plans.json', 'binding.json']}, 'ordinary_acceptance': False}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'status': binding['status'], 'binding': str(output / 'binding.json'), 'binding_sha256': helper.sha(output / 'binding.json'), 'manifest': str(output / 'manifest.json'), 'manifest_sha256': helper.sha(output / 'manifest.json'), 'selected': 9, 'xtask_controls_still_owed': 3}, indent=2), flush=True)


if __name__ == '__main__':
    main()
