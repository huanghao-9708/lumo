"""[MA0 Spike] 本地 WebDAV 测试桩。

在宿主机 8765 端口响应 PROPFIND(Depth 1) 的最小 207 multistatus，
供 Android 模拟器（10.0.2.2:8765）上的 debug_webdav_probe 做端到端连通性验证：
WebView 按钮 -> IPC -> reqwest(ring) -> 宿主 HTTP -> 207 -> XML 解析 -> JSON 回显。
"""
from http.server import BaseHTTPRequestHandler, HTTPServer

RESPONSE_207 = '''<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
    <D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
  <D:response>
    <D:href>/Music/</D:href>
    <D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
    <D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
  <D:response>
    <D:href>/Music/test.flac</D:href>
    <D:propstat><D:prop><D:resourcetype/><D:getcontentlength>1234567</D:getcontentlength></D:prop>
    <D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
</D:multistatus>'''


class StubHandler(BaseHTTPRequestHandler):
    def do_PROPFIND(self):
        body = RESPONSE_207.encode()
        self.send_response(207)
        self.send_header("Content-Type", "application/xml; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        print("[webdav-stub] PROPFIND %s <- 207 (%d bytes)" % (self.path, len(body)), flush=True)

    def log_message(self, fmt, *args):
        pass


if __name__ == "__main__":
    print("[webdav-stub] listening on 0.0.0.0:8765 (emulator: http://10.0.2.2:8765/)", flush=True)
    HTTPServer(("0.0.0.0", 8765), StubHandler).serve_forever()
