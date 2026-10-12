import hashlib
import json
from pathlib import Path

root = Path('/workspace/.proofs/native-child-command-lifetime078-v2')
cases = [('namespace-publication-intermediate', 'set created [namespace eval ::N {interp create kid}]; set before [list [interp slaves] [info commands ::N::kid] [info commands ::kid]]; set code [catch {::N::kid issafe} message]; list $created $before $code $message [::kid issafe]'), ('namespace-relative-root-lookup', 'namespace eval ::N {interp create kid; list [info commands kid] [info commands ::kid] [info commands ::N::kid] [kid issafe] [::kid issafe] [catch {::N::kid issafe} message] $message}'), ('namespace-root-child-survives-delete', 'namespace eval ::N {interp create kid}; namespace delete ::N; list [interp slaves] [info commands ::kid] [kid issafe]'), ('namespace-qualified-publication', 'namespace eval ::N {interp create ::N::kid}; list [interp slaves] [info commands ::N::kid] [info commands ::kid] [::N::kid issafe]'), ('namespace-relative-qualified-publication', 'namespace eval ::N {interp create N::kid}; list [interp slaves] [info commands ::N::kid] [info commands ::N::N::kid] [::N::kid issafe]'), ('namespace-renamed-child-deleted-with-namespace', 'namespace eval ::N {interp create kid; rename kid moved; list [info commands moved] [info commands ::kid]}; namespace delete ::N; list [interp slaves] [info commands ::N::moved] [info commands ::kid]'), ('renamed-child-delete-recreate', 'interp create kid; namespace eval ::N {rename ::kid moved}; interp delete kid; interp create kid; list [interp slaves] [info commands ::N::moved] [kid issafe]'), ('active-child-continuation-errors', 'interp create kid; interp alias kid swap {} swap; proc swap {} {interp delete kid; interp create kid; return REPLACED}; kid eval {set before OLD; set replacement [swap]; list $before $replacement}'), ('active-child-replacement-survives-error', 'interp create kid; interp alias kid swap {} swap; proc swap {} {interp delete kid; interp create kid; return REPLACED}; set code [catch {kid eval {set before OLD; set replacement [swap]; list $before $replacement}} message]; list $code $message [interp exists kid] [kid eval {info exists before}]')]
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
