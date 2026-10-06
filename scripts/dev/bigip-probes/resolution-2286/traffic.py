#!/usr/bin/env python3
"""Bounded independent TCP/HTTP requests with evidence of actual responding TMMs."""
import argparse
import base64
import collections
import hashlib
import json
import socket
import time
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--vip',required=True)
p.add_argument('--port',type=int,default=80)
p.add_argument('--host',default='resolution-2286.invalid')
p.add_argument('--run',required=True)
p.add_argument('--path',default='/?action=read')
p.add_argument('--requests',type=int,default=128)
p.add_argument('--source-ip',help='a real routable client address, never a BIG-IP self IP')
p.add_argument('--source-port-start',type=int,default=20000)
p.add_argument('--expect-units',default='',help='observed active roster, comma-separated, e.g. 0,1 or 0:0,0:1')
p.add_argument('--out',type=Path,required=True)
a=p.parse_args()
if a.requests<1 or a.requests>4096: p.error('requests must be 1..4096')
if a.source_port_start<1024 or a.source_port_start+a.requests>65536:p.error('source-port range invalid')
if any(c in a.host+a.path+a.run for c in '\r\n'):p.error('HTTP fields cannot contain newlines')
try:(a.host+a.path+a.run).encode('ascii')
except UnicodeEncodeError:p.error('HTTP fields must be ASCII; percent-encode non-ASCII URI bytes explicitly')
counts=collections.Counter()
with a.out.open('x',encoding='utf-8') as out:
    for i in range(a.requests):
        request_id=a.run+'-'+str(i)
        sep='&' if '?' in a.path else '?'
        target=a.path+sep+'probe_id='+request_id
        req=('GET '+target+' HTTP/1.1\r\nHost: '+a.host+'\r\nConnection: close\r\nX-R2286-Request: '+request_id+'\r\n\r\n').encode('ascii')
        record={'request':request_id,'target':target,'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
        try:
            family=socket.AF_INET6 if ':' in a.vip else socket.AF_INET
            with socket.socket(family,socket.SOCK_STREAM) as conn:
                conn.settimeout(8)
                if a.source_ip:
                    conn.bind((a.source_ip,a.source_port_start+i))
                conn.connect((a.vip,a.port))
                record['client']=conn.getsockname()
                record['peer']=conn.getpeername()
                conn.sendall(req)
                response=bytearray()
                while True:
                    data=conn.recv(65536)
                    if not data:break
                    response.extend(data)
                    if len(response)>1048576:raise RuntimeError('response exceeds 1 MiB evidence cap')
            raw=bytes(response)
            record['response_sha256']=hashlib.sha256(raw).hexdigest()
            record['response_base64']=base64.b64encode(raw).decode('ascii')
            headers=raw.partition(b'\r\n\r\n')[0].split(b'\r\n')
            record['status']=headers[0].decode('latin1') if headers else ''
            units=[h.partition(b':')[2].strip().decode('ascii') for h in headers if h.lower().startswith(b'x-r2286-tmm:')]
            record['reported_units']=units
            if len(units)==1:counts[units[0]]+=1
        except Exception as error:
            record['error']=str(error)
        out.write(json.dumps(record,sort_keys=True)+'\n');out.flush()
expected=set(filter(None,a.expect_units.split(',')))
summary={'actual_response_unit_counts':dict(counts),'expected_active_roster':sorted(expected),
         'missing_units':sorted(expected-set(counts)),
         'coverage':'COMPLETE_FOR_SUPPLIED_ROSTER' if expected and expected<=set(counts) else 'INCOMPLETE_OR_ROSTER_UNSPECIFIED'}
print(json.dumps(summary,indent=2))
