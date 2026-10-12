#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Measure the unchanged real appliance harness in newly created child interpreters."""
import datetime
import json
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[6] / 'scripts/dev/bigip-probes/name-patterns'))
import capture

ROOT=capture.ROOT
RUN='nh20261012harness1'
DEST=Path(__file__).resolve().parent
capture.TAG=RUN
orig_rpc=capture.rpc

def rpc(command,data=None,timeout=180):
    r=orig_rpc(command,data,timeout)
    r.stderr=b'\n'.join(x for x in r.stderr.split(b'\n') if not (x.startswith(b'** ') or b'openssh.com/pq.html' in x))
    return r
capture.rpc=rpc
remote='/var/tmp/tcl_lsp_'+RUN
assert rpc('test ! -e '+remote+' && mkdir -m 700 '+remote).returncode==0
for label,command in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info')]:
    r=rpc(command)
    (DEST/(label+'.stdout')).write_bytes(b'\n'.join(x for x in r.stdout.split(b'\n') if b'FIPS Module' not in x))
    (DEST/(label+'.stderr')).write_bytes(r.stderr)
    (DEST/(label+'.json')).write_text(json.dumps({'command':command,'exit_code':r.returncode})+'\n')
module=(ROOT/'experiments/name-handling/patterns/f5/answers/nh20261012repair2/real-tcltest-2.2.8.tcl').read_bytes()
(DEST/'real-tcltest-2.2.8.tcl').write_bytes(module)
(DEST/'provenance.json').write_text(json.dumps({'run_id':RUN,'configuration_saved':False,'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'transport':'authentication metadata omitted','real_module_sha256':capture.digest(module),'adaptation':'Fresh host-created child interpreter; module bytes evaluated explicitly rather than discovered through package search. Native parent package state remains independent.'},indent=2)+'\n')
chunks=[module[i:i+12000] for i in range(0,len(module),12000)]
original_make=capture.make_config
for ci,context in enumerate(['tmm-RULE_INIT','tmm-procedure','tmm-traffic-event']):
    providers=['/Common/__tcl_lsp_probe_'+RUN+'_'+str(ci)+'_module'+str(i) for i in range(len(chunks))]
    expression='append nh_module '+' '.join('[call '+p+'::bytes]' for p in providers)
    selected=[{'id':'real-harness-explicit-source-child','context':context,'variant_id':'host-child-unchanged-real-module-v1','original_input':{'path':str((DEST/'real-tcltest-2.2.8.tcl').relative_to(ROOT)),'sha256':capture.digest(module)},'setup':'set nh_child [interp create]; set nh_module {}; '+expression,'operation':'''set nh_child_version [interp eval $nh_child {list [info patchlevel] [info tclversion] [namespace current]}]; set nh_module_code [catch {interp eval $nh_child $nh_module} nh_module_result]; set nh_package_code [catch {interp eval $nh_child {package require tcltest}} nh_package_result]; list child-provider $nh_child_version module-code $nh_module_code module-result $nh_module_result require-code $nh_package_code require-result $nh_package_result''','cleanup':'interp delete $nh_child','scope':'Independent child-interpreter real-package capability. Does not substitute for native-parent package require or original suite.'}]
    (DEST/(context+'-cases.json')).write_text(json.dumps(selected,indent=2)+'\n')
    def config(ctx,name,source,port=19912):
        conf,objects,invoke=original_make(ctx,name,source,port)
        definitions=''.join('ltm rule '+p+' {\nproc bytes {} {return [binary format H* '+chunk.hex()+']}\n}\n' for p,chunk in zip(providers,chunks))
        return definitions+conf,[('ltm rule',p) for p in providers]+objects,invoke
    capture.make_config=config
    job=DEST/(context+'-0')
    capture.run_job(job,remote,context,'__tcl_lsp_probe_'+RUN+'_'+str(ci),capture.body(selected,context,RUN),wait=5)
    meta=json.loads((job/'driver.json').read_text());meta.pop('argv',None);meta['command']='bash -s';meta['authentication_metadata']='omitted';(job/'driver.json').write_text(json.dumps(meta,indent=2)+'\n')
    print('captured',context,flush=True)
capture.make_config=original_make
(DEST/'capture-complete.json').write_text(json.dumps({'completed':True})+'\n')
