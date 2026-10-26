#!/usr/bin/env python3
"""Check TCP reachability on every HTTP request."""
import json
import logging
import socket
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def should_change_ip():
    started = time.monotonic()
    try:
        with socket.create_connection(('ip.zhouzhipeng.com', 80), timeout=5):
            logging.info('TCP probe succeeded, should_change=False elapsed_ms=%.0f',
                         (time.monotonic() - started) * 1000)
            return False
    except OSError as exc:
        logging.warning('TCP probe failed, should_change=True elapsed_ms=%.0f error=%s',
                        (time.monotonic() - started) * 1000, exc)
        return True


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != '/should-change-ip':
            self.send_error(404)
            return
        logging.info('Live probe requested by %s', self.client_address[0])
        body = json.dumps(should_change_ip()).encode('ascii')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Cache-Control', 'no-store')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)


if __name__ == '__main__':
    logging.basicConfig(level=logging.INFO, format='%(asctime)s %(levelname)s %(message)s')
    ThreadingHTTPServer(('0.0.0.0', 8081), Handler).serve_forever()
