from pathlib import Path
import argparse
import collections
import gzip
import hashlib
import json
import re

SELECTOR = 'analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion'
PREFIXES = (
    'PROVIDER_FIXTURE_PHASE', 'ANALYSER_PHASE', 'ANALYSER_COMMAND_PHASE',
    'DIAGNOSTIC_PHASE', 'DIAGNOSTIC_CFG_EMISSION_PHASE', 'COMPILER_PIPELINE_PHASE',
    'COMPILER_FUNCTION_PHASE_TOTALS', 'COMPILER_FUNCTION_PHASE',
    'COMPILER_BODY_LATTICE_PHASE',
)
FIELD = re.compile(r'([a-z_]+)=([A-Za-z0-9_.:/<>-]+)\Z')
HARNESS = 'test ' + SELECTOR + ' ... '
FUNCTION_INDEX = dict(enumerate(('prepare', 'ssa', 'def_use', 'dynamic_names', 'sccp',
                                'existence', 'types', 'rendered', 'instance_classes', 'taint')))
REQUIRED = {
    'PROVIDER_FIXTURE_PHASE': ('profile', 'bytes', 'stage', 'elapsed_us', 'thread_us'),
    'ANALYSER_PHASE': ('bytes', 'stage', 'ms'),
    'ANALYSER_COMMAND_PHASE': ('bytes', 'site', 'stage', 'ms'),
    'DIAGNOSTIC_PHASE': ('bytes', 'stage', 'ms'),
    'DIAGNOSTIC_CFG_EMISSION_PHASE': ('bytes', 'procedures', 'methods', 'body_units', 'stage', 'us'),
    'COMPILER_PIPELINE_PHASE': ('pipeline', 'bytes', 'procedures', 'stage', 'us'),
    'COMPILER_FUNCTION_PHASE_TOTALS': ('pipeline', 'final', 'functions', 'blocks', 'statements',
        'prepare_us', 'ssa_us', 'def_use_us', 'dynamic_names_us', 'sccp_us', 'existence_us',
        'types_us', 'rendered_us', 'instance_classes_us', 'taint_us'),
    'COMPILER_FUNCTION_PHASE': ('index', 'elapsed_us'),
    'COMPILER_BODY_LATTICE_PHASE': ('ordinal', 'stage', 'elapsed_us'),
}
NUMERIC = {'bytes', 'site', 'methods', 'body_units', 'procedures', 'functions', 'blocks',
           'statements', 'index', 'ordinal', 'ms', 'us'}

def valid_fields(prefix, fields):
    if set(fields) != set(REQUIRED[prefix]):
        return False
    if any(not value.isdigit() for key, value in fields.items()
           if key in NUMERIC or key.endswith('_us')):
        return False
    if prefix == 'COMPILER_FUNCTION_PHASE' and int(fields['index']) not in FUNCTION_INDEX:
        return False
    return fields.get('final', 'false') in ('true', 'false')


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()

def scan(path):
    rows = []
    profile = None
    skipped = collections.Counter()
    decoded = hashlib.sha256()
    with gzip.open(path, 'rb') as source:
        while True:
            line = source.readline(4097)
            if not line:
                break
            decoded.update(line)
            if not line.endswith(b'\n') and len(line) > 4096:
                skipped['overlong_lines'] += 1
                while line and not line.endswith(b'\n'):
                    line = source.readline(4097)
                    decoded.update(line)
                continue
            try:
                text = line.decode('ascii').strip()
            except UnicodeDecodeError:
                skipped['non_ascii_lines'] += 1
                continue
            if text.startswith(HARNESS):
                text = text[len(HARNESS):]
            parts = text.split()
            if not parts or parts[0] not in PREFIXES:
                skipped['unselected_lines'] += 1
                continue
            fields = {}
            for part in parts[1:]:
                match = FIELD.fullmatch(part)
                if not match or match[1] in fields:
                    break
                fields[match[1]] = match[2]
            else:
                if valid_fields(parts[0], fields):
                    if parts[0] == 'PROVIDER_FIXTURE_PHASE':
                        profile = fields.get('profile')
                    row = {'profile': profile, 'prefix': parts[0], 'fields': fields, 'line': text}
                    if parts[0] == 'COMPILER_FUNCTION_PHASE':
                        row['completed_function_stage'] = FUNCTION_INDEX.get(int(fields['index']))
                    rows.append(row)
                    continue
            skipped['malformed_primitive_lines'] += 1
    profiles = []
    for selected in dict.fromkeys(row['profile'] for row in rows):
        scoped = [row for row in rows if row['profile'] == selected]
        by_prefix = collections.defaultdict(list)
        for row in scoped:
            by_prefix[row['prefix']].append(row)
        latest = {prefix: values[-1] for prefix, values in by_prefix.items()}
        profiles.append({
            'profile': selected,
            'primitive_count': len(scoped),
            'provider_clock_rows': by_prefix['PROVIDER_FIXTURE_PHASE'],
            'latest_by_prefix': latest,
            'last_32_primitives': scoped[-32:],
            'completed_body_ordinals': [row['fields']['ordinal'] for row in
                by_prefix['COMPILER_BODY_LATTICE_PHASE'] if row['fields'].get('stage') == 'complete'],
        })
    return {'log': str(path), 'log_sha256': sha(path), 'decoded_sha256': decoded.hexdigest(),
            'skipped': dict(skipped), 'profiles': profiles, 'rows': rows,
            'scope': 'Bounded primitive TEMP diagnostic only. Timers are cumulative or nested; '
                     'a completed boundary does not identify exclusive cost, original function identity, '
                     'unfinished work duration or semantic acceptance. Missing body-complete means unfinished, '
                     'not failure of a particular remaining statement.'}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--directory', type=Path, default=Path('/tmp/name-tcltest-temp199-20261014'))
    parser.add_argument('--output', type=Path, default=Path('/tmp/name-temp199-primitive-reader-20261014/summary.json'))
    args = parser.parse_args()
    receipt = json.loads((args.directory / 'result.json').read_text())
    if not receipt.get('receipt_valid') or receipt.get('source_changes'):
        raise SystemExit('TEMP receipt must be sealed and valid without source changes')
    log = args.directory / 'run.log.gz'
    result = scan(log)
    if result['log_sha256'] != receipt['log_sha256']:
        raise SystemExit('Log hash differs from sealed TEMP receipt')
    result['receipt_sha256'] = sha(args.directory / 'result.json')
    result['timed_out'] = receipt.get('timed_out')
    result['host_elapsed_seconds'] = receipt.get('elapsed_seconds')
    cpu = args.directory / 'timeout-process-cpu.json'
    if cpu.exists():
        result['timeout_process_cpu'] = json.loads(cpu.read_text())
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'output': str(args.output), 'rows': len(result['rows']),
                      'profiles': [{'profile': row['profile'], 'primitives': row['primitive_count'],
                                    'latest': {key: value['fields'] for key, value in row['latest_by_prefix'].items()}}
                                   for row in result['profiles']],
                      'skipped': result['skipped']}, indent=2))

if __name__ == '__main__':
    main()
