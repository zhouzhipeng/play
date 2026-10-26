#!/usr/bin/env python3
"""TCP reachability probe with an in-memory consecutive failure flag."""
import json
import logging
import socket
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class ProbeState:
    def __init__(self):
        self.failures = 0
        self.flag = False
        self.lock = threading.Lock()

    def record(self, reachable):
        with self.lock:
            self.failures = 0 if reachable else min(3, self.failures + 1)
            self.flag = self.failures >= 3

    def read(self):
        with self.lock:
            return self.flag


STATE = ProbeState()


def monitor():
    deadline = time.monotonic()
    while True:
        try:
            with socket.create_connection(('ip.zhouzhipeng.com', 80), timeout=10):
                reachable = True
        except OSError as exc:
            logging.warning('TCP probe failed: %s', exc)
            reachable = False
        STATE.record(reachable)
        logging.info('reachable=%s failures=%s flag=%s', reachable, STATE.failures, STATE.read())
        deadline += 60
        time.sleep(max(0, deadline - time.monotonic()))


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != '/should-change-ip':
            self.send_error(404)
            return
        body = json.dumps(STATE.read()).encode('ascii')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Cache-Control', 'no-store')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)


if __name__ == '__main__':
    logging.basicConfig(level=logging.INFO, format='%(asctime)s %(levelname)s %(message)s')
    threading.Thread(target=monitor, daemon=True).start()
    ThreadingHTTPServer(('0.0.0.0', 8081), Handler).serve_forever()
