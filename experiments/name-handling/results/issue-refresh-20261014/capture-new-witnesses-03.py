import datetime
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path('/workspace/tcl-lsp')
OUT = Path('/tmp/name-issues-refresh-20261014/oracles-03')
OUT.mkdir(exist_ok=True)
SCRIPTS = {
    '2419-sibling-variable-writers': 'package require struct::list\nset v {a b c}\nstruct::list swap v 0 2\nputs $v\nstruct::list delete v b\nputs $v\nproc odd {x} {expr {$x % 2}}\nputs [struct::list split {1 2 3} odd pass fail]\nputs [list $pass $fail]\nset rows [struct::list dbJoin -keys keys 0 {{k A}} 0 {{k B}}]\nputs [list $keys $rows]\nset rows [struct::list dbJoinKeyed -keys keyed {{k A}} {{k B}}]\nputs [list $keyed $rows]\n',
    'provider-identity': 'puts [info patchlevel]\n',
    '2417-root-body-collision': 'set g 1\nproc top {} {}\nputs [list $g [info procs top]]\n',
    '2296-computed-array-key-read': 'array set a {x 1}\nset k x\nputs $a($k)\n',
    '2268-unknown-and-distinct-array-key': 'proc p {k} {set a(x) 1; if {[info exists a($k)]} {puts yes} else {puts no}}\np x\np y\n',
    '2209-catch-array-result': 'catch {set x ok} a(key)\nputs [list [info exists a] [info exists a(key)] [set a(key)]]\nproc p {} {catch {set x local} a(key); return [set a(key)]}\nputs [p]\nnamespace eval ns {}\ncatch {set x qualified} ::ns::a(key)\nputs [set ::ns::a(key)]\n',
    '2209-catch-array-options': 'catch {set x ok} result options(key)\nputs [list $result [info exists options(key)] [dict get $options(key) -code]]\n',
    '2235-braced-incr-amount': 'proc p {} {set n 3; set x 1; incr x {$n}; return $x}\nputs [catch {p} m]\nputs $m\n',
    '2255-array-index-outer-mask': 'proc b {} {return k}\nset i k\nset a(k) V\nputs [subst -nocommands {$a([b])}]\nputs [subst -nobackslashes {$a(\\x6b)}]\nputs [subst -nocommands {$a($i)}]\n',
    '2218-dynamic-proc-head-rebinding': 'proc double_expr {x} {return [expr {[expr {$x * 2}]}]}\nproc g {cmd} {$cmd}\nproc change {} {rename ::expr ::realexpr; proc ::expr {args} {return [lindex $args 0]}}\ng change\nputs [double_expr 2]\n',
    '2324-original-tk-computed-head': 'package require Tk\nset e [ttk::entry .e]\nputs [llength [$e configure]]\n',
    '2289-child-created-in-procedure': 'proc New {} {catch {interp delete vectorCoreChild}; interp create vectorCoreChild; interp eval vectorCoreChild {set child VALUE}}\nNew\nputs [interp eval vectorCoreChild {set child}]\ninterp delete vectorCoreChild\nputs [catch {interp eval vectorCoreChild {set child}}]\n',
    '2332-external-global-reads': 'proc bysubst {} {return $::seen}\nproc bycommand {} {return [set ::seen]}\nproc bydecl {} {global seen; return $seen}\nset seen FIRST\nputs [list [bysubst] [bycommand] [bydecl]]\nset seen SECOND\nputs [list [bysubst] [bycommand] [bydecl]]\n',
    '2339-assignment-name-evaluation': 'proc p {} {set i 0; set a([incr i]) 5; if {$i == 0} {return zero} else {return nonzero}}\nputs [p]\n',
    '2399-global-read-store-before-unset': 'proc b6 {} {global g; return $g}\nset g 6\nset r [b6]; puts "<$r>"\nset g 123\nset r [b6]; puts "<$r>"\nunset g\n',
    '2419-tcllib-variable-writers': 'package require struct::set\npackage require struct::list\nproc inc {} {set s {}; struct::set include s a; return $s}\nproc exc {} {set s {a b}; struct::set exclude s a; return $s}\nproc add {} {set s {a}; struct::set add s {b}; return $s}\nproc sub {} {set s {a b}; struct::set subtract s {a}; return $s}\nproc shf {} {set l {a b c}; struct::list shift l; return $l}\nproc asg {} {set x 0; struct::list assign {1 2} x y; return $x}\nforeach p {inc exc add sub shf asg} {puts "$p: [$p]"}\n',
    '2143-materialised-factory': 'proc Configure {name default description} {proc $name {x} [subst -nocommands {return $default}]}\nConfigure port 8080 {the port}\nputs [port ignored]\n',
}

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def tree(path):
    return {str(p.relative_to(path)): sha(p) for p in sorted(path.rglob('*')) if p.is_file() and '.git' not in p.parts}

TCLLIB = Path('/tmp/name-issues-refresh-20261014/tcllib-tcllib-2-0')

def inputs(name, binary, library):
    return {'binary': sha(binary), 'library': tree(library), 'shared_libraries': {str(p): sha(p) for p in sorted(binary.parent.glob('libtcl*.so*')) if p.is_file()}, 'tcllib': tree(TCLLIB)}

providers = [(f'tcl{v}', ROOT / f'tmp/tcl{v}/unix/tclsh', ROOT / f'tmp/tcl{v}/library') for v in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']]
providers.append(('jim', ROOT / 'tmp/jimtcl-oracle/jimsh', ROOT / 'tmp/jimtcl-oracle'))
records = []
before = {}
for name, binary, library in providers:
    before[name] = inputs(name, binary, library)
    env = os.environ.copy()
    if name != 'jim':
        env['TCL_LIBRARY'] = str(library)
        env['LD_LIBRARY_PATH'] = str(binary.parent)
        env['TCLLIBPATH'] = ' '.join(str(TCLLIB / 'modules' / component) for component in ['struct', 'cmdline'])
    provider = OUT / name
    provider.mkdir(exist_ok=True)
    for case, source in SCRIPTS.items():
        case_dir = provider / case
        case_dir.mkdir(exist_ok=True)
        script = case_dir / 'source.tcl'
        script.write_text(source)
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        try:
            answer = subprocess.run([str(binary), str(script)], env=env, capture_output=True, timeout=20)
            code, stdout, stderr = answer.returncode, answer.stdout, answer.stderr
        except subprocess.TimeoutExpired as e:
            code, stdout, stderr = None, e.stdout or b'', e.stderr or b''
        (case_dir / 'stdout').write_bytes(stdout)
        (case_dir / 'stderr').write_bytes(stderr)
        row = {'provider': name, 'case': case, 'argv': [str(binary), str(script)], 'started_at': started, 'exit_code': code, 'source_sha256': sha(script), 'binary_sha256': before[name]['binary'], 'stdout_sha256': sha(case_dir / 'stdout'), 'stderr_sha256': sha(case_dir / 'stderr')}
        (case_dir / 'receipt.json').write_text(json.dumps(row, indent=2) + '\n')
        records.append(row)
        print(name, case, code, repr(stdout.decode(errors='replace')[:150]), repr(stderr.decode(errors='replace')[:100]), flush=True)
after = {name: inputs(name, binary, library) for name, binary, library in providers}
seal = {'recorded_at': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'runner_sha256': sha(Path(__file__)), 'jim_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT / 'tmp/jimtcl-oracle', text=True).strip(), 'changed_provider_inputs': [name for name in before if before[name] != after[name]], 'records': records, 'authority': 'Exact direct original interpreter executions only. No tcltest, Rust acceptance, native receipt admission, F5 or successful unavailable package execution is claimed.'}
seal['tcllib_tag'] = 'tcllib-2-0'
seal['tcllib_revision'] = '2a63bf212cc6423efb7a39958596ae2a2333c266'
seal['tcllib_archive_sha256'] = sha(TCLLIB.parent / 'tcllib-2-0.tar.gz')
seal['provider_environment'] = {'C Tcl': {'TCL_LIBRARY': 'matching provider library', 'LD_LIBRARY_PATH': 'matching provider unix directory', 'TCLLIBPATH': [str(TCLLIB / 'modules' / p) for p in ['struct', 'cmdline']]}, 'Jim': 'inherited environment, no C Tcl library or package donation'}
for name, snapshot in [('provider-inputs-before.json.gz', before), ('provider-inputs-after.json.gz', after)]:
    with (OUT / name).open('wb') as stream:
        with gzip.GzipFile(fileobj=stream, mode='wb', mtime=0) as compressed:
            compressed.write((json.dumps(snapshot, indent=2) + '\n').encode())
(OUT / 'outcomes.json').write_text(json.dumps(seal, indent=2) + '\n')
