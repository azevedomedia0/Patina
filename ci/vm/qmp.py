#!/usr/bin/env python3
"""Inject keyboard and pointer input into the build VM through QEMU's QMP socket.

  qmp.py move X Y        absolute pointer, X and Y as 0.0..1.0 fractions of the screen
  qmp.py click [left]    press and release a button
  qmp.py key ret         press and release a key (QMP key names: a, ret, esc, ctrl, ...)
"""
import json, os, socket, sys, time

SOCK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "state", "qmp.sock")


def connect():
    s = socket.socket(socket.AF_UNIX)
    s.connect(SOCK)
    f = s.makefile("rw")
    f.readline()  # greeting
    send(f, {"execute": "qmp_capabilities"})
    return f


def send(f, cmd):
    f.write(json.dumps(cmd) + "\n")
    f.flush()
    while True:
        reply = json.loads(f.readline())
        if "return" in reply or "error" in reply:
            return reply


def events(f, evs):
    r = send(f, {"execute": "input-send-event", "arguments": {"events": evs}})
    if "error" in r:
        sys.exit(r["error"]["desc"])


def main(argv):
    f = connect()
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd == "move":
        x, y = (int(float(v) * 32767) for v in argv[2:4])
        events(f, [{"type": "abs", "data": {"axis": "x", "value": x}},
                   {"type": "abs", "data": {"axis": "y", "value": y}}])
    elif cmd == "click":
        btn = argv[2] if len(argv) > 2 else "left"
        events(f, [{"type": "btn", "data": {"button": btn, "down": True}}])
        time.sleep(0.05)
        events(f, [{"type": "btn", "data": {"button": btn, "down": False}}])
    elif cmd == "key":
        key = {"type": "qcode", "data": argv[2]}
        events(f, [{"type": "key", "data": {"key": key, "down": True}}])
        time.sleep(0.05)
        events(f, [{"type": "key", "data": {"key": key, "down": False}}])
    else:
        sys.exit(__doc__)


main(sys.argv)
