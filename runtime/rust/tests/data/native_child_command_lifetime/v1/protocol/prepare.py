import hashlib
import json
from pathlib import Path

root = Path('/workspace/.proofs/native-child-command-lifetime078')
cases = [
    ('availability', 'interp create kid; list [interp exists kid] [kid issafe]'),
    ('namespace-publication', 'namespace eval ::N {interp create kid}; list [interp slaves] [info commands ::N::kid] [info commands ::kid] [::N::kid issafe]'),
    ('namespace-rename-delete', 'namespace eval ::N {interp create kid; rename kid moved}; interp delete kid; list [interp slaves] [info commands ::N::moved] [info commands ::kid]'),
    ('replace-child-command', 'interp create kid; proc kid {} {return REPLACEMENT}; list [interp slaves] [kid]'),
    ('delete-child-command', 'interp create kid; rename kid {}; list [interp slaves] [info commands kid]'),
    ('rename-delete-recreate', 'interp create kid; rename kid moved; interp delete kid; interp create kid; list [interp slaves] [info commands moved] [kid issafe]'),
    ('namespace-delete-child-command', 'namespace eval ::N {interp create kid}; namespace delete ::N; list [interp slaves] [info commands ::N::kid]'),
    ('active-child-script-recreation', 'interp create kid; interp alias kid swap {} swap; proc swap {} {interp delete kid; interp create kid; return REPLACED}; list [catch {kid eval {set before OLD; set replacement [swap]; list $before $replacement}} message] $message [interp exists kid] [kid eval {info exists before}]'),
]
records = []
header = []
for n, (name, text) in enumerate(cases):
    value = text.encode('ascii')
    path = root / 'inputs' / (name + '.tcl')
    path.parent.mkdir(exist_ok=True)
    path.write_bytes(value)
    records.append(dict(id=name, channel='Counted ASCII source passed to Tcl_EvalEx or Jim_NewStringObj/Jim_EvalObj', source_hex=value.hex(), source_sha256=hashlib.sha256(value).hexdigest()))
    header.append('static const unsigned char source_%d[] = {%s};' % (n, ','.join(str(x) for x in value)))
header.append('static const Case cases[] = {')
for n, (name, text) in enumerate(cases):
    header.append('{"%s", source_%d, %d},' % (name, n, len(text.encode('ascii'))))
header.append('};')
(root / 'cases.h').write_text('\n'.join(header) + '\n')
(root / 'inputs.json').write_text(json.dumps(records, indent=2) + '\n')
