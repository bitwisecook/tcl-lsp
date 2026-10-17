"""Root-only actual full18 binder; no build/list/test execution."""
from pathlib import Path
import argparse
import importlib.util
import json
import sys
sys.dont_write_bytecode = True


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--package-manifest', type=Path, required=True)
    ap.add_argument('--package-sha256', required=True)
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    if sys.flags.optimize:
        raise RuntimeError('Optimised Python disables canonical guards')
    import hashlib
    if hashlib.sha256(args.package_manifest.read_bytes()).hexdigest() != args.package_sha256:
        raise RuntimeError('Prepared package changed')
    package = json.loads(args.package_manifest.read_text())
    record = package['files']['support']
    if hashlib.sha256(Path(record['path']).read_bytes()).hexdigest() != record['sha256']:
        raise RuntimeError('Support changed')
    spec = importlib.util.spec_from_file_location('_extra18_bind_support', record['path'])
    helper = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = helper
    spec.loader.exec_module(helper)
    helper.checked_files(package)
    helper.require(Path(package['files']['binder']['path']).resolve() == Path(__file__).resolve(), 'Another binder pinned')
    ctx, plans = helper.context(package)
    pre = ctx['check_current']()
    destination = Path(ctx['admission']['destination']).resolve()
    extras = {kind: helper.bind_extra(ctx, destination / ('extra-' + kind), kind) for kind in ['libs', 'xtask']}
    full = helper.check_current(package, ctx, extras)
    helper.require(full['standard'] == pre, 'Standard/source changed while binding')
    output = args.output.resolve()
    helper.require(output.parent == destination and not output.exists(), 'New owned binding directory required')
    artifacts = dict(extras['libs']['artifacts'], **extras['xtask']['artifacts'])
    helper.require(set(artifacts) == set(helper.COUNTS), 'All four extra targets required')
    binding = {'status': 'ACTUAL_EXTRA18_BOUND_ALL_FOUR_TARGETS', 'selected': 18, 'package_manifest': {'path': str(args.package_manifest.resolve()), 'sha256': args.package_sha256}, 'standard_admission': package['files']['standard_admission'], 'extra_builds': extras, 'counts': helper.COUNTS, 'pre_post_binding': full, 'ordinary_acceptance': False, 'test_or_listing_execution': False}
    output.mkdir()
    (output / 'binding.json').write_text(json.dumps(binding, indent=2) + '\n')
    (output / 'bound-plans.json').write_text(json.dumps(plans, indent=2) + '\n')
    (output / 'artifacts.json').write_text(json.dumps(artifacts, indent=2) + '\n')
    manifest = {'status': binding['status'], 'selected': 18, 'prepared_package_sha256': args.package_sha256, 'binder_source_sha256': package['files']['binder']['sha256'], 'files': {name: {'path': str(output / name), 'sha256': helper.sha(output / name)} for name in ['binding.json', 'bound-plans.json', 'artifacts.json']}}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'manifest': str(output / 'manifest.json'), 'sha256': helper.sha(output / 'manifest.json')}, indent=2))


if __name__ == '__main__':
    main()
