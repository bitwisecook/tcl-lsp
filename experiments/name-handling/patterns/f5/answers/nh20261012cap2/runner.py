# Sanitised retained source: transport/device literals changed; these are not original captured bytes.
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Retain isolated BIG-IP naming inputs, applied configuration and raw evidence."""

import argparse
import datetime
import hashlib
import io
import json
import re
import shlex
import subprocess
import tarfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SSH = ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", "-o", "ForwardAgent=no", "-o", "IdentityAgent=none", "-i", "/redacted/ssh-identity", "redacted-lab-target"]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def literal(text):
    if not text:
        return '{}'
    if not text.isascii():
        return '{' + text + '}'
    return '[binary format H* ' + text.encode('ascii').hex() + ']' 


def rpc(command, data=None, timeout=180):
    return subprocess.run(SSH + [command], input=data, capture_output=True, timeout=timeout, check=False)


def emit(context, tag, case, stage, code, value, unit):
    logger = "log local0." if context.startswith("tmm-") else "puts"
    return f"""
set nh_ec {{}}
set nh_ei {{}}
set nh_ec_present 0
set nh_ei_present 0
if {{{code} == 1}} {{
    if {{[info exists ::errorCode]}} {{set nh_ec $::errorCode; set nh_ec_present 1}}
    if {{[info exists ::errorInfo]}} {{set nh_ei $::errorInfo; set nh_ei_present 1}}
}}
foreach {{nh_field nh_value}} [list result {value} metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {{
    if {{$nh_field != "result" && {code} != 1}} {{continue}}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {{[catch {{$nh_encoder convertto utf-8 $nh_value}} nh_encoded]}} {{
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }}
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {{([string length $nh_hex] + 399) / 400}}]
    if {{$nh_chunks == 0}} {{set nh_chunks 1}}
    for {{set nh_i 0}} {{$nh_i < $nh_chunks}} {{incr nh_i}} {{
        set nh_part [string range $nh_hex [expr {{$nh_i * 400}}] [expr {{$nh_i * 400 + 399}}]]
        {logger} "NHO|{tag}|{case}|{context}|{stage}|{code}|{unit}|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }}
}}
"""


def body(cases, context, tag):
    unit = '[TMM::cmp_group]:[TMM::cmp_unit]' if context.startswith('tmm-') else 'none'
    source = f'set nh_unit "{unit}"\nset nh_invocation once\n'
    if context == 'tmm-traffic-event':
        source += 'set nh_invocation [HTTP::header value X-NH-Invocation]\n'
    for case in cases:
        source += f"set nh_setup {literal(case.get('setup', ''))}\nset nh_operation {literal(case['operation'])}\n"
        source += "set nh_setup_code [catch {eval $nh_setup} nh_setup_result]\n"
        source += emit(context, tag, case['id'], 'setup', '$nh_setup_code', '$nh_setup_result', '$nh_unit')
        source += "if {$nh_setup_code == 0} {\nset nh_code [catch {eval $nh_operation} nh_result]\n"
        source += emit(context, tag, case['id'], 'lookup', '$nh_code', '$nh_result', '$nh_unit')
        source += "}\n"
        if case.get('cleanup'):
            source += f"set nh_cleanup {literal(case['cleanup'])}\nset nh_code [catch {{eval $nh_cleanup}} nh_result]\n"
            source += emit(context, tag, case['id'], 'runtime-cleanup', '$nh_code', '$nh_result', '$nh_unit')
    return source


def capabilities(tag):
    n = '::__nh_' + tag
    specs = [
        ('version', '', 'list [info patchlevel] [info tclversion] [namespace current]'),
        ('encoding', '', 'binary scan [encoding convertto utf-8 [format %c 945]] H* x; set x'),
        ('namespace.eval', '', f'namespace eval {n} {{namespace current}}'),
        ('namespace.code', '', f'namespace eval {n} {{namespace code {{list captured}}}}'),
        ('namespace.path', '', 'namespace path'),
        ('namespace.export', f'namespace eval {n} {{}}', f'namespace eval {n} {{namespace export nh_none}}'),
        ('namespace.import', f'namespace eval {n} {{}}', f'namespace eval {n} {{namespace import ::__absent_nh_*}}'),
        ('namespace.ensemble', f'namespace eval {n} {{}}', f'namespace eval {n} {{namespace ensemble create -command facade -map {{}}}}'),
        ('namespace.upvar', f'namespace eval {n} {{variable x VALUE}}', f'namespace upvar {n} x nh_link; set nh_link'),
        ('interp.create', '', 'set nh_child [interp create]; set nh_result [interp eval $nh_child {info patchlevel}]; interp delete $nh_child; set nh_result'),
        ('interp.alias', f'proc {n}_target {{}} {{return TARGET}}', f'interp alias {{}} {n}_alias {{}} {n}_target; set r [{n}_alias]; interp alias {{}} {n}_alias {{}}; set r'),
        ('tcltest', '', 'package require tcltest'),
        ('tcloo', '', f'oo::class create {n}_class {{}}; {n}_class destroy'),
        ('catch.options', '', 'catch {list value} nh_r nh_o; list $nh_r $nh_o'),
        ('trace.execution', f'proc {n}_target {{}} {{return TARGET}}; proc {n}_trace args {{}}', f'trace add execution {n}_target leave {n}_trace; {n}_target'),
        ('proc.dynamic', '', f'proc {n}_target {{}} {{return TARGET}}; {n}_target'),
        ('call', '', f'call {n}_absent'),
    ]
    cases=[]
    for key, setup, op in specs:
        unique=n+'_'+key.replace('.', '_')
        cleanup=f'if {{[namespace exists {unique}]}} {{namespace delete {unique}}}; foreach nh_cmd [info commands {unique}_*] {{rename $nh_cmd {{}}}}; list [namespace exists {unique}] [info commands {unique}_*]'
        cases.append(dict(id=key, setup=setup.replace(n,unique), operation=op.replace(n,unique), cleanup=cleanup))
    return cases


def make_config(context, name, source, port=19912):
    base = '/Common/' + name
    if context == 'tmsh-cli-script':
        return f'cli script {base} {{\nproc script::run {{}} {{\n{source}\n}}\n}}\n', [('cli script', base)], f'tmsh run cli script {base}'
    if context == 'iapp-implementation':
        config = f'sys application template {base} {{\n actions {{ definition {{ implementation {{\n{source}\n}} presentation {{}} }} }}\n}}\nsys application service {base}_service {{ template {base} }}\n'
        return config, [('sys application template', base), ('sys application service', base + '_service')], f'tmsh modify sys application service {base}_service execute-action definition'
    if context == 'icall':
        event = name + '_event'
        config = f'sys icall script {base} {{ definition {{\n{source}\n}} }}\nsys icall handler triggered {base}_handler {{ script {base} subscriptions {{ run {{ event-name {event} }} }} }}\n'
        return config, [('sys icall script', base), ('sys icall handler triggered', base + '_handler')], f'tmsh generate sys icall event name {event}'
    if context == 'iapp-apl':
        config = f'sys application template {base} {{ actions {{ definition {{ implementation {{}} presentation {{ section probe {{ choice result default "UNREACHED" tcl {{\n{source}\nreturn [list [list "DONE" "DONE"]]\n}} }} }} }} }} }}\nsys application service {base}_service {{ template {base} }}\n'
        return config, [('sys application template', base), ('sys application service', base + '_service')], f'tmsh modify sys application service {base}_service execute-action definition'
    if context == 'tmm-procedure':
        config = f'ltm rule {base} {{\nproc measure {{}} {{\n{source}\n}}\nwhen RULE_INIT {{ call measure }}\n}}\n'
    elif context == 'tmm-traffic-event':
        config = f'ltm rule {base} {{\nwhen HTTP_REQUEST {{\n{source}\nHTTP::respond 200 content "{name} [TMM::cmp_group]:[TMM::cmp_unit]" X-NH-Invocation $nh_invocation Connection close\n}}\n}}\nltm virtual {base}_vs {{ destination 192.0.2.10:{port} mask 255.255.255.255 ip-protocol tcp profiles {{ tcp {{}} http {{}} }} rules {{ {base} }} }}\n'
        return config, [('ltm rule', base), ('ltm virtual', base + '_vs')], None
    else:
        config = f'ltm rule {base} {{\nwhen RULE_INIT {{\n{source}\n}}\n}}\n'
    return config, [('ltm rule', base)], None


def run_job(dest, remote_root, context, name, source, wait=2):
    dest.mkdir(parents=True, exist_ok=False)
    (dest / 'input.tcl').write_text(source)
    if context.startswith('stock-'):
        version = context.split('-')[1]
        command = f'/usr/bin/tclsh{version}'
        p = rpc(command, source.encode(), 60)
        (dest/'runtime.stdout').write_bytes(p.stdout)
        (dest/'runtime.stderr').write_bytes(p.stderr)
        (dest/'invocation.json').write_text(json.dumps({'argv': SSH+[command], 'exit_code':p.returncode, 'stdin_sha256':digest(source.encode())}, indent=2)+'\n')
        return
    config, objects, invoke = make_config(context, name, source)
    (dest/'applied.conf').write_text(config)
    evidence = remote_root + '/evidence/' + name
    remote_config = remote_root + '/' + name + '.conf'
    p = rpc('cat > ' + shlex.quote(remote_config), config.encode())
    if p.returncode:
        raise RuntimeError(p.stderr.decode(errors='replace'))
    script = f'''set -u
mkdir -p {shlex.quote(evidence)}
E={shlex.quote(evidence)}
owned=()
run() {{ label=$1; shift; "$@" > "$E/$label.stdout" 2> "$E/$label.stderr"; echo $? > "$E/$label.exit"; }}
cleanup() {{
  for (( i=${{#owned[@]}}-1; i>=0; i-- )); do
    read -r -a words <<< "${{owned[$i]}}"
    run "delete-$i" tmsh delete "${{words[@]}}"
    run "after-$i" tmsh list "${{words[@]}}"
  done
}}
trap cleanup EXIT
'''
    for index, (kind, obj) in enumerate(objects):
        script += f'run before-{index} tmsh list {kind} {shlex.quote(obj)}\n'
        script += f'if ! grep -q "was not found" "$E/before-{index}.stderr" "$E/before-{index}.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi\n'
    if context == 'tmm-traffic-event':
        script += 'run destination-inventory tmsh list ltm virtual one-line\nif grep -q "destination 192.0.2.10:19912 " "$E/destination-inventory.stdout"; then echo "Destination collision" >&2; exit 3; fi\n'
    script += f'run input-hash sha256sum {shlex.quote(remote_config)}\n'
    script += f'if ! grep -q "^{digest(config.encode())} " "$E/input-hash.stdout"; then exit 4; fi\n'
    # Ownership begins only after every absence check, before a possibly partial merge.
    for kind, obj in objects:
        script += f'owned+=({shlex.quote(kind + " " + obj)})\n'
    script += f'run load tmsh load sys config merge file {shlex.quote(remote_config)}\n'
    for index, (kind, obj) in enumerate(objects):
        script += f'run applied-{index} tmsh list {kind} {shlex.quote(obj)}\n'
    if invoke:
        script += 'run invoke ' + invoke + '\n'
    elif context == 'tmm-traffic-event':
        script += 'if [ "$(cat "$E/load.exit")" = 0 ] && [ "$(cat "$E/applied-1.exit")" = 0 ]; then touch "$E/traffic-ready"; for n in {1..90}; do [ -e "$E/traffic-done" ] && break; sleep 1; done; fi\n'
    log_path='/var/tmp/scriptd.out' if context in {'icall','iapp-apl','iapp-implementation'} else '/var/log/ltm'
    script += f'sleep {wait}\nrun runtime-logs grep -F {shlex.quote("NHO|" + TAG + "|")} {log_path}\n'
    script += 'echo "configuration_saved=false" > "$E/save-status.txt"\n'
    (dest/'invocation.sh').write_text(script)
    if context == 'tmm-traffic-event':
        process=subprocess.Popen(SSH+['bash -s'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        process.stdin.write(script.encode())
        process.stdin.close()
        for _ in range(30):
            ready=rpc('test -e '+shlex.quote(evidence+'/traffic-ready'))
            if ready.returncode == 0:
                break
            if process.poll() is not None:
                break
            time.sleep(0.5)
        if ready.returncode != 0:
            raise RuntimeError('Traffic configuration did not become ready')
        for index in range(32):
            command=['curl','-sS','--max-time','2','-D',str(dest/f'traffic-{index}.headers'),'-H',f'X-NH-Invocation: {index}','http://192.0.2.10:19912/']
            result=subprocess.run(command,capture_output=True,check=False)
            (dest/f'traffic-{index}.stdout').write_bytes(result.stdout)
            (dest/f'traffic-{index}.stderr').write_bytes(result.stderr)
            (dest/f'traffic-{index}.json').write_text(json.dumps({'argv':command,'exit_code':result.returncode},indent=2)+'\n')
            if index % 32 == 31:
                print('traffic',name,index+1,flush=True)
            if result.returncode and index > 4:
                break
        rpc('touch '+shlex.quote(evidence+'/traffic-done'))
        stdout=process.stdout.read()
        stderr=process.stderr.read()
        p=subprocess.CompletedProcess(SSH+['bash -s'],process.wait(timeout=90),stdout,stderr)
    else:
        p = rpc('bash -s', script.encode(), 600)
    (dest/'driver.stdout').write_bytes(p.stdout)
    (dest/'driver.stderr').write_bytes(p.stderr)
    (dest/'driver.json').write_text(json.dumps({'argv':SSH+['bash -s'], 'exit_code':p.returncode, 'stdin_sha256':digest(script.encode())},indent=2)+'\n')
    p = rpc('tar -C ' + shlex.quote(evidence) + ' -cf - .', timeout=60)
    (dest/'transport.stderr').write_bytes(p.stderr)
    if p.returncode:
        raise RuntimeError('Evidence retrieval failed')
    with tarfile.open(fileobj=io.BytesIO(p.stdout)) as tar:
        for member in tar.getmembers():
            if member.isfile():
                path = Path(member.name)
                if path.is_absolute() or '..' in path.parts:
                    raise RuntimeError('Invalid evidence path')
                (dest/path.name).write_bytes(tar.extractfile(member).read())
    for index in range(len(objects)):
        after = dest/f'after-{index}.stderr'
        if not after.exists() or b'was not found' not in after.read_bytes() + (dest/f'after-{index}.stdout').read_bytes():
            raise RuntimeError(f'Cleanup not proved: {context}/{name}/{index}')


def main():
    global TAG
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run',required=True)
    parser.add_argument('--phase',choices=['capabilities', 'questions', 'broader'], default='capabilities')
    args=parser.parse_args()
    if not re.fullmatch(r'[a-z0-9]{6,24}',args.run):
        raise ValueError('Run tag must be lowercase ASCII alphanumeric')
    TAG=args.run
    dest=ROOT/'experiments/name-handling/patterns/f5/answers'/TAG
    dest.mkdir(parents=True,exist_ok=False)
    (dest/'runner.py').write_bytes(Path(__file__).read_bytes())
    remote='/var/tmp/tcl_lsp_'+TAG
    p=rpc('test ! -e '+shlex.quote(remote)+' && mkdir -m 700 '+shlex.quote(remote))
    if p.returncode:
        raise RuntimeError('Remote staging directory is not absent')
    (dest/'staging-create.stdout').write_bytes(p.stdout)
    (dest/'staging-create.stderr').write_bytes(p.stderr)
    for label,command in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info'),('failover','tmsh show sys failover'),('virtuals','tmsh list ltm virtual one-line')]:
        result=rpc(command)
        (dest/(label+'.stdout')).write_bytes(result.stdout)
        (dest/(label+'.stderr')).write_bytes(result.stderr)
        (dest/(label+'.json')).write_text(json.dumps({'argv':SSH+[command],'exit_code':result.returncode},indent=2)+'\n')
    cases=capabilities(TAG)
    (dest/'capabilities.json').write_text(json.dumps(cases,indent=2)+'\n')
    (dest/'provenance.json').write_text(json.dumps({'run_id':TAG,'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'input_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'remote_staging':remote,'configuration_saved':False},indent=2)+'\n')
    for context in ['stock-8.4','stock-8.5','tmsh-cli-script','iapp-implementation','icall','tmm-RULE_INIT','tmm-procedure','tmm-traffic-event','iapp-apl']:
        name='__tcl_lsp_probe_'+TAG+'_'+context.replace('-','_').replace('.','_')
        groups=[cases[i:i+3] for i in range(0,len(cases),3)] if context in {'tmm-RULE_INIT','tmm-procedure'} else [cases]
        for group_id, group in enumerate(groups):
            group_name=name+'_'+str(group_id)
            source=body(group,context,TAG)
            run_job(dest/(context+'-'+str(group_id)),remote,context,group_name,source,wait=4 if context=='icall' else 2)
            print('captured',context,group_id,flush=True)
        continue
        source=body(cases,context,TAG)
        if context.startswith('stock-'):
            source=source.replace('tmsh::log err','puts')
        run_job(dest/context,remote,context,name,source,wait=4 if context=='icall' else 2)
        print('captured',context,flush=True)


if __name__=='__main__':
    main()

