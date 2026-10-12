#!/usr/bin/env python3
"""Measure real-harness and original-name ownership boundaries; omit authentication metadata."""
import datetime
import hashlib
import json
from pathlib import Path
import sys
import subprocess

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'scripts/dev/bigip-probes/name-patterns'))
import capture

RUN = 'nh20261012repair2'
DEST = Path(__file__).resolve().parent
capture.TAG = RUN
original_rpc = capture.rpc

def rpc(command, data=None, timeout=180):
    result = original_rpc(command, data, timeout)
    # Transport security notices are outside the measured Tcl/configuration result.
    result.stderr = b'\n'.join(line for line in result.stderr.split(b'\n') if not (line.startswith(b'** ') or b'openssh.com/pq.html' in line))
    return result

capture.rpc = rpc
remote = '/var/tmp/tcl_lsp_' + RUN
r = rpc('test ! -e ' + remote + ' && mkdir -m 700 ' + remote)
if r.returncode: raise RuntimeError('staging absence not proved')
(DEST / 'provenance.json').write_text(json.dumps({'run_id': RUN, 'configuration_saved': False, 'timestamp_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'input_commit': subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(), 'transport': 'authenticated remote execution; authentication metadata omitted'}, indent=2)+'\n')
for label, command in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info'),('failover','tmsh show sys failover')]:
    r = rpc(command)
    output = b'\n'.join(line for line in r.stdout.split(b'\n') if b'FIPS Module' not in line)
    (DEST/(label+'.stdout')).write_bytes(output)
    (DEST/(label+'.stderr')).write_bytes(r.stderr)
    (DEST/(label+'.json')).write_text(json.dumps({'command':command,'exit_code':r.returncode},indent=2)+'\n')

ns = '::__tcl_lsp_probe_' + RUN
cases = [
 {'id':'version','setup':'','operation':'list [info patchlevel] [info tclversion] [namespace current]'},
 {'id':'parent-original-name-inventory','setup':'','operation':'''set nh_inventory {}; foreach nh_name {::forward ::change ::outer ::emit ::red ::blue ::DebugPuts ::tcltest::DebugPuts} {set nh_body {}; set nh_body_code [catch {info body $nh_name} nh_body]; lappend nh_inventory [list $nh_name commands [info commands $nh_name] procs [info procs $nh_name] body-code $nh_body_code body $nh_body]}; foreach nh_name {ns target description prefix} {lappend nh_inventory [list frame-variable $nh_name exists [info exists $nh_name] vars [info vars $nh_name] traces [trace vinfo $nh_name]]}; set nh_inventory'''},
 {'id':'parent-tcltest-real-package','setup':'','operation':'package require tcltest'},
 {'id':'child-provider-and-proc-owner','setup':'set nh_child [interp create]','operation':'''interp eval $nh_child {list [info patchlevel] [info tclversion] [namespace current] [namespace eval '''+ns+''' {proc tail {} {namespace current}; list [info commands '''+ns+'''::tail] [info commands ::tail] [catch {tail} nh_x] $nh_x}]}''','cleanup':'interp delete $nh_child'},
 {'id':'child-tcltest-real-package','setup':'set nh_child [interp create]','operation':'interp eval $nh_child {list [package require tcltest] [info commands ::tcltest::test] [info commands ::test]}','cleanup':'interp delete $nh_child'},
 {'id':'parent-proc-rename-owner','setup':'if {[namespace exists '+ns+'] || [llength [info commands '+ns+'_tail]]} {error ownership-collision}; namespace eval '+ns+' {}','operation':'namespace eval '+ns+' {proc '+ns+'_tail {} {variable debug; namespace current}}; set nh_before [list [info commands '+ns+'::'+ns.rsplit('::',1)[-1]+'_tail] [info commands '+ns+'_tail]]; rename '+ns+'_tail '+ns+'::tail; list $nh_before [info commands '+ns+'::tail] [catch {'+ns+'::tail} nh_x] $nh_x','cleanup':'if {[llength [info commands '+ns+'_tail]]} {rename '+ns+'_tail {}}; namespace delete '+ns},
]
for context in ['tmsh-cli-script','iapp-implementation','icall','tmm-RULE_INIT','tmm-procedure','tmm-traffic-event']:
    selected=[dict(c,context=context,variant_id='real-package-and-owner-diagnostic-v1',exact_source=False,scope='Independent capability/ownership diagnostic; no original lookup answer') for c in cases]
    (DEST/(context+'-cases.json')).write_text(json.dumps(selected,indent=2)+'\n')
    # Separate load groups keep each TMM configuration below its source limit.
    for index, group in enumerate([selected[:3],selected[3:]]):
        job=DEST/(context+'-'+str(index))
        name='__tcl_lsp_probe_'+RUN+'_'+context.replace('-','_')+'_'+str(index)
        capture.run_job(job,remote,context,name,capture.body(group,context,RUN),wait=5)
        for filename in ['driver.json','invocation.json']:
            path=job/filename
            if path.exists():
                meta=json.loads(path.read_text()); meta.pop('argv',None);meta['command']='bash -s' if filename=='driver.json' else 'retained input execution'; meta['authentication_metadata']='omitted';path.write_text(json.dumps(meta,indent=2)+'\n')
        print('captured',context,index,flush=True)
(DEST/'capture-complete.json').write_text(json.dumps({'completed':True})+'\n')
