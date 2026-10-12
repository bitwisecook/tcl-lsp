#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Derive row conclusions without changing original answer or evidence authorities."""
import argparse
import collections
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
DEST = Path(__file__).resolve().parent
BASE = DEST.parent
MATRIX = DEST / 'original-questions.json'
CACHE = {}
DIGESTS = {}


def read(path):
    path = Path(path)
    if path not in CACHE:
        CACHE[path] = json.loads(path.read_text())
    return CACHE[path]


def digest(path):
    path = Path(path)
    if path not in DIGESTS:
        DIGESTS[path] = hashlib.sha256(path.read_bytes()).hexdigest()
    return DIGESTS[path]


def descriptor(path, pointer=None):
    path = Path(path)
    value = {'path': path.relative_to(ROOT).as_posix(), 'sha256': digest(path)}
    if pointer is not None:
        value['json_pointer'] = pointer
    return value


def resolve(value):
    path = ROOT / value['path']
    assert path.is_relative_to(ROOT) and '..' not in Path(value['path']).parts
    assert digest(path) == value['sha256'], value['path']
    result = read(path)
    for part in value.get('json_pointer', '').split('/')[1:]:
        key = part.replace('~1', '/').replace('~0', '~')
        result = result[int(key)] if isinstance(result, list) else result[key]
    return result


def native_records(question_id, context, run_name):
    path = BASE / run_name / 'records.json'
    if not path.exists():
        return []
    result = []
    source_descriptor = descriptor(path)
    for index, record in enumerate(read(path)):
        if record['question_id'] != question_id or record['provider']['context'] != context:
            continue
        if record['status'] != 'observed' or record['setup']['completion_code'] != 0 or record['lookup']['reached'] is not True:
            continue
        if not all(field['complete'] for stage in (record['setup'], record['lookup']) for field in stage['capture_fields']):
            continue
        result.append({'measurement': dict(source_descriptor, json_pointer='/' + str(index)), 'case_id': record['case_id'], 'event': record['provider']['event'], 'tmm_unit': record['provider']['tmm_unit'], 'invocation_id': record['invocation_id'], 'completion_code': record['lookup']['completion_code']})
    return result


def cell(value):
    return str(value).replace('|', '\\|').replace('\n', ' ')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--native-review', type=Path)
    args = parser.parse_args()
    assert not (DEST / 'MANIFEST.sha256').exists(), 'Sealed run is immutable'
    original = read(MATRIX)
    proposed = copy.deepcopy(original)
    prior1path = BASE / 'nh20261012unreached1/reviewed-unreached.json'
    prior2path = BASE / 'nh20261012unreached2/reviewed-boundaries.json'
    prior1 = {(r['question_id'], r['identity']['context']): (i, r) for i, r in enumerate(read(prior1path)['reviews'])}
    prior2 = {(r['question_id'], r['matrix_context']): (i, r) for i, r in enumerate(read(prior2path)['matrix_rows'])}
    native_review = {}
    if args.native_review:
        native_review = {(r['question_id'], r['context']): r for r in read(args.native_review)['rows']}
        assert len(native_review) == 15
    resolutions = []
    categories = collections.Counter()
    preserved = []
    for question, original_question in zip(proposed['questions'], original['questions'], strict=True):
        for context, entry in question['contexts'].items():
            key = question['id'], context
            old = original_question['contexts'][context]
            if old['status'] == 'observed':
                preserved.append({'question_id': question['id'], 'context': context, 'answer_record': copy.deepcopy(old['answer_record'])})
                assert entry == old
                continue
            conclusion = {'question_id': question['id'], 'canonical_question_id': question.get('canonical_question_id'), 'case_id': question['case_id'], 'context': context, 'previous_status': old['status'], 'original_attempt_record': copy.deepcopy(old.get('attempt_record')), 'investigation_status': 'complete', 'intended_original_lookup_reached': False, 'evidence': [], 'scope': 'No unchanged portable final lookup is inferred from setup, whole-program errors, adapted spellings, or a diagnostic child.'}
            if question['case_id'] is not None:
                if key in prior1:
                    index, reviewed = prior1[key]
                    category = 'earlier-original-program-failure'
                    conclusion['conclusion'] = reviewed['review']['reason']
                    conclusion['evidence'].append(descriptor(prior1path, '/reviews/' + str(index)))
                elif key in prior2:
                    index, reviewed = prior2[key]
                    category = 'real-harness-constraint-skip' if question['case_id'].startswith('wiki-') else 'earlier-original-program-failure'
                    conclusion['evidence'].append(descriptor(prior2path, '/matrix_rows/' + str(index)))
                    measurements = reviewed['measurements']
                    if category == 'real-harness-constraint-skip':
                        names = sorted({m['constraint']['name'] for m in measurements})
                        versions = sorted({m['provider']['interpreter_patchlevel'] for m in measurements})
                        conclusion['conclusion'] = 'The unchanged selected test is skipped by its actual real-harness constraint ' + ', '.join(names) + ' in both appliance stock interpreters (own versions ' + ', '.join(versions) + '). ' + str(measurements[0]['terminal_boundary']) + '; no selected body return or final lookup is measured.'
                    else:
                        conclusion['conclusion'] = 'The unchanged original program fails before its intended final operation: ' + str(measurements[0]['original_failure']['failing_command']) + '. The complete stack and sequential source review retain this earlier boundary; no final result is inferred.'
                elif question['case_id'].startswith('wiki-'):
                    attempt = resolve(old['attempt_record'])
                    setup = attempt['setup']
                    assert setup['completion_code'] == 1
                    raw = bytes.fromhex(setup['result_hex'])
                    if context.startswith('tmm-'):
                        category = 'real-harness-package-missing'
                        assert b"can't find package tcltest" in raw
                        conclusion['conclusion'] = 'Real tcltest package require fails in this actual parent context before the selected original test body. Explicit unchanged-module sourcing in a separate fresh-child control also fails at pwd; this supplies no body result and does not replace the parent harness.'
                        conclusion['evidence'].append(descriptor(BASE / 'nh20261012harness2/records.json', '/' + str(next(i for i, r in enumerate(read(BASE / 'nh20261012harness2/records.json')) if r['provider']['context'] == context))))
                    else:
                        category = 'real-harness-initialisation-debug'
                        assert b'can\'t read "debug"' in raw
                        conclusion['conclusion'] = 'The real tcltest package initialiser fails reading debug before the selected original test body; no native-parent or original body result is inferred.'
                        diagnostic_name = 'nh20261012aplrepair1' if context == 'iapp-apl' else 'nh20261012repair2'
                        diagnostic_path = BASE / diagnostic_name / 'records.json'
                        diagnostic_case = 'native-real-tcltest' if context == 'iapp-apl' else 'parent-tcltest-real-package'
                        diagnostic_index = next(i for i, r in enumerate(read(diagnostic_path)) if r['provider']['context'] == context and r['case_id'] == diagnostic_case)
                        conclusion['evidence'].append(descriptor(diagnostic_path, '/' + str(diagnostic_index)))
                        if context != 'iapp-apl':
                            conclusion['conclusion'] += ' The separate same-context fresh-child package success applies only to that child.'
                    conclusion['evidence'].append(copy.deepcopy(old['attempt_record']))
                else:
                    category = 'original-ownership-guard-collision'
                    attempt = resolve(old['attempt_record'])
                    assert attempt['setup']['completion_code'] == 1
                    message = bytes.fromhex(attempt['setup']['result_hex']).decode('ascii')
                    assert 'ownership-collision' in message
                    conclusion['conclusion'] = 'Original ownership guard rejects existing state: ' + message + '. Ownership was not acquired, so the unchanged original program was not entered and unowned state was left intact. Separately renamed controls do not answer this original lookup.'
                    conclusion['evidence'].append(copy.deepcopy(old['attempt_record']))
                conclusion['terminal_boundary'] = category
                conclusion['status'] = 'unreached'
                conclusion['original_answer_record'] = None
                entry.update(status='unreached', runtime='unreached', reached=False, answer_record=None)
            elif not context.startswith('tmm-'):
                category = 'native-call-absent'
                path = BASE / ('nh20261012aplrepair1' if context == 'iapp-apl' else 'nh20261012nativecap1') / 'records.json'
                records = read(path)
                desired = ['iapp-apl'] if context == 'iapp-apl' else ['stock-8.4', 'stock-8.5'] if context == 'appliance-stock-tclsh' else [context]
                cases = ['native-call-command-surface', 'native-call-runtime'] if context == 'iapp-apl' else ['native-call-command-surface', question['id']]
                selected = [(i, r) for i, r in enumerate(records) if r['provider']['context'] in desired and r['case_id'] in cases]
                assert len(selected) == 2 * len(desired)
                for index, record in selected:
                    assert record['setup']['completion_code'] == 0 and record['lookup']['reached']
                    if record['case_id'] != 'native-call-command-surface':
                        assert record['lookup']['completion_code'] == 1 and bytes.fromhex(record['lookup']['result_hex']) == b'invalid command name "call"'
                    conclusion['evidence'].append(descriptor(path, '/' + str(index)))
                versions = sorted({r['provider']['interpreter_patchlevel'] for _, r in selected})
                assert None not in versions
                conclusion.update(status='unsupported', terminal_boundary=category, conclusion='Native call is absent in this exact context (own Tcl patchlevels ' + ', '.join(versions) + '): actual command/procedure surface is empty and the attempted dispatch fails with missing call. The question requires native rule dispatch, so provider selection, mutation, lifetime and storage cannot execute through this surface. No hypothetical provider behaviour is assigned.', scope='Actual native-command prerequisite boundary for this question/context; APL uses shared actual call/surface controls, stock includes both measured interpreters.')
                entry.update(status='unsupported', runtime='unsupported', reached=False)
            else:
                category = 'native-composite-observation'
                selected = native_records(question['id'], context, 'nh20261012native1') + native_records(question['id'], context, 'nh20261012native2')
                assert selected
                conclusion['measurements'] = selected
                conclusion['evidence'] = [r['measurement'] for r in selected]
                conclusion['scope'] = 'Composite native question answer limited to listed actual materialisations, administrative phases, events, units and invocations. Nested caught completions remain result data; no portable original or suspended-frame, restart, reload or unmeasured worker claim.'
                if key in native_review:
                    review = native_review[key]
                    assert review['investigation_status'] == 'complete'
                    selected = []
                    for selector in review['measurements']:
                        measurement = dict(selector['records'], json_pointer=selector['json_pointer'])
                        record = resolve(measurement)
                        assert record['status'] == 'observed' and record['setup']['completion_code'] == 0 and record['lookup']['reached'] is True
                        assert all(f['complete'] for stage in (record['setup'], record['lookup']) for f in stage['capture_fields'])
                        assert record['provider']['context'] == context == selector['context']
                        assert record['provider']['event'] == selector['event'] and record['provider']['tmm_unit'] == selector['unit']
                        assert record['invocation_id'] == selector['invocation'] and record['case_id'] == selector['case_id']
                        assert record['actual_input'] == selector['source'] and record['case_metadata'] == selector['case_metadata']
                        assert record['question_id'] == question['id'] or (question['id'] == 'corpus-f5-computed-target' and record['case_id'] == 'literal-matched-arguments')
                        selected.append({'measurement': measurement, 'case_id': record['case_id'], 'source_question_id': record['question_id'], 'contribution': 'direct comparison control' if record['question_id'] != question['id'] else 'question operation', 'event': selector['event'], 'tmm_unit': selector['unit'], 'invocation_id': selector['invocation'], 'administrative_phase': selector.get('administrative_phase'), 'completion_code': record['lookup']['completion_code']})
                    assert selected
                    conclusion['measurements'] = selected
                    conclusion['evidence'] = [r['measurement'] for r in selected]
                    conclusion['conclusion'] = review['conclusion']
                    conclusion['evidence'].append(descriptor(args.native_review, '/rows/' + str(read(args.native_review)['rows'].index(review))))
                else:
                    conclusion['investigation_status'] = 'pending-native-phase-review'
                    conclusion['conclusion'] = 'Provisional measured native cases: ' + ', '.join(sorted({r['case_id'] for r in selected})) + '. Complete observations retained; remaining initialisation phase gaps require native2 and independent review before final admission.'
                conclusion.update(status='observed', terminal_boundary=category, intended_original_lookup_reached=None)
                entry.update(status='observed', runtime='observed', reached=True)
            categories[category] += 1
            resolutions.append(conclusion)
            pointer = '/rows/' + str(len(resolutions) - 1)
            entry['resolution_record'] = {'path': (DEST / 'resolutions.json').relative_to(ROOT).as_posix(), 'json_pointer': pointer, 'answer_document': (DEST / 'ANSWERS.md').relative_to(ROOT).as_posix()}
            entry['investigation_status'] = conclusion['investigation_status']
            entry['reason'] = conclusion['conclusion']
            if question['case_id'] is None:
                entry['answer_record'] = copy.deepcopy(entry['resolution_record'])
                entry['scope'] = conclusion['scope']
    assert len(resolutions) == 307 and len(preserved) == 93
    assert categories == {'real-harness-initialisation-debug': 120, 'real-harness-package-missing': 90, 'original-ownership-guard-collision': 12, 'earlier-original-program-failure': 37, 'real-harness-constraint-skip': 8, 'native-call-absent': 25, 'native-composite-observation': 15}
    output = {'schema_version': 1, 'derived_run_id': DEST.name, 'new_execution': False, 'scope': 'Per-row completed investigations preserve original attempts and distinguish actual observed, capability-unsupported and unreached final operations.', 'rows': resolutions}
    (DEST / 'resolutions.json').write_text(json.dumps(output, indent=2) + '\n')
    sha = descriptor(DEST / 'resolutions.json')['sha256']
    for q in proposed['questions']:
        for c in q['contexts'].values():
            if 'resolution_record' in c:
                c['resolution_record']['sha256'] = sha
                if c['answer_record'] is not None:
                    c['answer_record']['sha256'] = sha
    (DEST / 'proposed-questions.json').write_text(json.dumps(proposed, indent=2) + '\n')
    states = collections.Counter(c['status'] for q in proposed['questions'] for c in q['contexts'].values())
    investigation = collections.Counter(r['investigation_status'] for r in resolutions)
    assert states == {'observed': 108, 'unsupported': 25, 'unreached': 267}
    validation = {'source_matrix': descriptor(MATRIX), 'total_rows': 400, 'previously_accepted_rows_preserved': 93, 'previously_nonaccepted_rows': 307, 'result_status_counts': dict(states), 'investigation_status_counts': dict(investigation), 'terminal_categories': dict(categories), 'final_ready': investigation == {'complete': 307}, 'canonical_matrix_modified': False, 'canonical_proof_manifest_modified': False}
    checked_references = 0
    for row in resolutions:
        for evidence in row['evidence']:
            resolve(evidence)
            checked_references += 1
        if row['original_attempt_record']:
            resolve(row['original_attempt_record'])
            checked_references += 1
    validation['evidence_references_verified'] = checked_references
    validation['native_composite_selectors'] = sum(len(row.get('measurements', [])) for row in resolutions)
    validation['portable_null_final_answers'] = 267
    validation['repository_gate_status'] = 'Coverage checks passed; full repository gates are not asserted by this derivation.'
    (DEST / 'validation.json').write_text(json.dumps(validation, indent=2) + '\n')
    lines = ['# Per-row investigation conclusions', '', 'Draft coverage pending native phase review.' if not validation['final_ready'] else 'All 307 previously outstanding row investigations are complete. This means 108 measured answers, 25 native-command capability boundaries and 267 unreached original final operations across the 400-row matrix; it does not mean every final lookup executed.', '', 'The 93 existing accepted answers and their pointers are preserved. Portable setup/package/constraint failures keep answer_record null. Adapted collision controls remain separate.', '', '| Question | Context | Status | Investigation | Conclusion | Evidence |', '| --- | --- | --- | --- | --- | --- |']
    for i, r in enumerate(resolutions):
        links = ['[' + Path(e['path']).parent.name + e.get('json_pointer', '') + '](' + '/' + (ROOT / e['path']).as_posix().lstrip('/') + ')' for e in r['evidence'][:3]]
        lines.append('| ' + ' | '.join(cell(x) for x in [r['question_id'], r['context'], r['status'], r['investigation_status'], r['conclusion'], '; '.join(links) + '; complete selectors in resolutions.json /rows/' + str(i)]) + ' |')
    (DEST / 'ANSWERS.md').write_text('\n'.join(lines) + '\n')
    print(json.dumps(validation))


if __name__ == '__main__':
    main()
