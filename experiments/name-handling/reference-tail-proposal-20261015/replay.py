from pathlib import Path
import argparse
import datetime
import hashlib
import json
import os
import subprocess

HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('--repository', type=Path, default=HERE.parents[2])
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--initial', action='store_true')
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=False)
kind = 'initial' if args.initial else 'controlled'
original = json.loads((HERE / kind / 'manifest.json').read_text())
probe = HERE / kind / 'probe.tcl'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(probe) == original['probe_sha256']
rows = []
for row in original['rows']:
    provider = row.get('provider_label', row.get('provider'))
    source_argv = row['argv']
    binary = args.repository / Path(source_argv[0]).relative_to('/workspace/tcl-lsp')
    original_files = row.get('original_files_before', row.get('files_before'))
    selected_files = {
        str(args.repository / Path(path).relative_to('/workspace/tcl-lsp')): expected
        for path, expected in original_files.items()
    }
    before = {path: sha(Path(path)) for path in selected_files}
    assert before == selected_files
    env = dict(os.environ)
    selected_library = row.get('selected_original_library')
    if selected_library:
        selected_library = str(args.repository / Path(selected_library).relative_to('/workspace/tcl-lsp'))
        env['TCL_LIBRARY'] = selected_library
    argv = [str(binary), str(probe)]
    case = row.get('case', 'original')
    if not args.initial:
        argv.append(case)
    run = subprocess.run(argv, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)
    stem = str(provider).replace('.', '-') + '-' + case
    stdout, stderr = args.out / (stem + '.stdout'), args.out / (stem + '.stderr')
    stdout.write_bytes(run.stdout)
    stderr.write_bytes(run.stderr)
    after = {path: sha(Path(path)) for path in before}
    assert before == after
    rows.append({'provider_label': provider, 'case': case, 'argv': argv,
                 'selected_original_library': selected_library,
                 'original_files_before': before, 'original_files_after': after,
                 'exit_code': run.returncode, 'stdout_file': stdout.name,
                 'stderr_file': stderr.name, 'stdout_sha256': sha(stdout),
                 'stderr_sha256': sha(stderr)})
manifest = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'probe_sha256': sha(probe), 'source_manifest_sha256': sha(HERE / kind / 'manifest.json'),
            'scope': 'Fresh original-provider grammar observations only; statuses/refusals/signals retained, no Rust/LSP or execution authority claim.',
            'rows': rows}
(args.out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
