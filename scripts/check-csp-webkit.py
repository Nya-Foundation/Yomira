#!/usr/bin/env python3
"""Check that WebKitGTK enforces the app's Content Security Policy.

`check:csp` drives Chromium, which is WebView2's engine. Linux ships
WebKitGTK, whose CSP support has to be verified against the engine itself.

No devtools and no visible window: the page reports its own findings back over
HTTP, so an offscreen webview is enough (a screenshot would prove nothing,
since the window does not paint without a GPU).

Needs python3-gi and the WebKit2 4.1 typelib (libwebkit2gtk-4.1-dev).
Pass --policy-file to test a policy captured from a packaged build instead of
the configured one.
"""
import base64, hashlib, json, re, sys, threading, http.server, socketserver
from pathlib import Path

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import Gtk, WebKit2, GLib  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
HASHED = "window.hashedInline = true"
DIGEST = base64.b64encode(hashlib.sha256(HASHED.encode()).digest()).decode()

argv = sys.argv[1:]
if "--policy-file" in argv:
    policy = Path(argv[argv.index("--policy-file") + 1]).read_text().strip()
else:
    policy = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())["app"]["security"]["csp"]

# Tauri serves the configured policy plus a generated script-src; mirror that
# shape so the hash and 'self' sources are actually exercised.
if "script-src" in policy:
    policy = re.sub(r"script-src ", f"script-src 'sha256-{DIGEST}' ", policy, count=1)
else:
    policy += f"; script-src 'self' 'sha256-{DIGEST}'"

HTML = f"""<!doctype html><html><head><meta charset="utf-8"><title>csp</title></head><body>
<script>window.plainInline = true</script>
<script>{HASHED}</script>
<script src="/plain.js"></script>
<script src="/probe.js"></script>
</body></html>"""

PROBE = """
var violations = [];
document.addEventListener('securitypolicyviolation', function (e) {
  violations.push(e.effectiveDirective);
});
var evalOk; try { eval('1+1'); evalOk = true; } catch (e) { evalOk = false; }
try { fetch('https://example.invalid/x').catch(function () {}); } catch (e) {}
setTimeout(function () {
  var x = new XMLHttpRequest();
  x.open('POST', '/report', true);
  x.send(JSON.stringify({
    plainInline: window.plainInline === true,
    hashedInline: window.hashedInline === true,
    externalSelf: window.plainExternal === true,
    evalAllowed: evalOk,
    violations: violations,
    engine: navigator.userAgent.replace(/.*AppleWebKit\\/([0-9.]+).*/, '$1'),
  }));
}, 700);
"""

result, done = {}, threading.Event()


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def _send(self, body, ctype):
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Security-Policy", policy)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/":
            self._send(HTML.encode(), "text/html")
        elif self.path == "/plain.js":
            self._send(b"window.plainExternal = true", "text/javascript")
        elif self.path == "/probe.js":
            self._send(PROBE.encode(), "text/javascript")
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        result.update(json.loads(self.rfile.read(n) or b"{}"))
        self.send_response(200)
        self.end_headers()
        done.set()


srv = socketserver.TCPServer(("127.0.0.1", 0), Handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()
window = Gtk.OffscreenWindow()
window.add(WebKit2.WebView())
window.show_all()
window.get_child().load_uri(f"http://127.0.0.1:{srv.server_address[1]}/")
loop = GLib.MainLoop()
GLib.timeout_add(250, lambda: False if done.is_set() and (loop.quit() or True) else True)
GLib.timeout_add_seconds(20, lambda: loop.quit() or False)
loop.run()

if not result:
    sys.exit("no report: the probe script never ran, so nothing was verified")

expected = {"plainInline": False, "hashedInline": True, "externalSelf": True, "evalAllowed": False}
failures = [f"{k}: expected {v}, got {result.get(k)}" for k, v in expected.items() if result.get(k) is not v]
if "connect-src" not in result.get("violations", []):
    failures.append("connect-src: a cross-origin fetch was not blocked")

print(f"WebKitGTK (AppleWebKit/{result.get('engine')}), policy {len(policy)} chars")
for key, value in expected.items():
    print(f"  {key}: {result.get(key)}")
print(f"  violations: {sorted(set(result.get('violations', [])))}")
if failures:
    sys.exit("CSP NOT enforced as expected:\n  " + "\n  ".join(failures))
print("csp enforced: inline blocked, hashes honored, self allowed, eval and cross-origin blocked")
