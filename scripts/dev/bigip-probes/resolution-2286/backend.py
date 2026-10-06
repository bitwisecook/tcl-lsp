#!/usr/bin/env python3
"""Disposable external HTTP backend; no BIG-IP configuration access."""
import argparse
import json
import time
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--bind',required=True)
p.add_argument('--port',type=int,default=18080)
a=p.parse_args()
class Handler(BaseHTTPRequestHandler):
    protocol_version='HTTP/1.1'
    def do_GET(self):
        value={'backend':self.server.server_address,'client':self.client_address,
               'path':self.path,'probe_id':self.headers.get('X-R2286-Request'),
               'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
        body=(json.dumps(value,sort_keys=True)+'\n').encode('utf-8')
        print(json.dumps(value,sort_keys=True),flush=True)
        self.send_response(200)
        self.send_header('Content-Type','application/json')
        self.send_header('Content-Length',str(len(body)))
        self.send_header('Connection','close')
        self.end_headers();self.wfile.write(body);self.close_connection=True
ThreadingHTTPServer((a.bind,a.port),Handler).serve_forever()
