#!/usr/bin/env python3
"""Inject keyboard and pointer input into the build VM through QEMU's QMP socket.

  qmp.py move X Y        absolute pointer, X and Y as 0.0..1.0 fractions of the screen
  qmp.py click [left]    press and release a button
  qmp.py down|up [left]  press or release a button (for drags)
  qmp.py drag X1 Y1 X2 Y2  press at (X1,Y1), move to (X2,Y2) in steps, release
  qmp.py combo alt m     hold modifiers, tap the last key
  qmp.py screendump FILE.png  capture the VM's display (what a monitor would show)
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
    elif cmd in ("down", "up"):
        btn = argv[2] if len(argv) > 2 else "left"
        events(f, [{"type": "btn", "data": {"button": btn, "down": cmd == "down"}}])
    elif cmd == "drag":
        x1, y1, x2, y2 = (float(v) for v in argv[2:6])
        def at(x, y):
            events(f, [{"type": "abs", "data": {"axis": "x", "value": int(x * 32767)}},
                       {"type": "abs", "data": {"axis": "y", "value": int(y * 32767)}}])
        at(x1, y1); time.sleep(0.15)
        events(f, [{"type": "btn", "data": {"button": "left", "down": True}}]); time.sleep(0.15)
        for i in range(1, 11):
            at(x1 + (x2 - x1) * i / 10, y1 + (y2 - y1) * i / 10); time.sleep(0.05)
        time.sleep(0.15)
        events(f, [{"type": "btn", "data": {"button": "left", "down": False}}])
    elif cmd == "screendump":
        r = send(f, {"execute": "screendump", "arguments": {"filename": os.path.abspath(argv[2]), "format": "png"}})
        if "error" in r:
            sys.exit(r["error"]["desc"])
    elif cmd == "combo":
        keys = [{"type": "qcode", "data": k} for k in argv[2:]]
        for k in keys:
            events(f, [{"type": "key", "data": {"key": k, "down": True}}]); time.sleep(0.05)
        for k in reversed(keys):
            events(f, [{"type": "key", "data": {"key": k, "down": False}}]); time.sleep(0.05)
    elif cmd == "key":
        key = {"type": "qcode", "data": argv[2]}
        events(f, [{"type": "key", "data": {"key": key, "down": True}}])
        time.sleep(0.05)
        events(f, [{"type": "key", "data": {"key": key, "down": False}}])
    else:
        sys.exit(__doc__)


main(sys.argv)
