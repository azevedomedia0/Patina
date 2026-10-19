#!/usr/bin/env python3
"""Inject keyboard and pointer input into the build VM through QEMU's QMP socket.

  qmp.py move X Y        absolute pointer, X and Y as 0.0..1.0 fractions of the screen
  qmp.py click [left]    press and release a button
  qmp.py down|up [left]  press or release a button (for drags)
  qmp.py drag X1 Y1 X2 Y2  press at (X1,Y1), move to (X2,Y2) in steps, release
  qmp.py combo alt m     hold modifiers, tap the last key
  qmp.py status          show whether the VM is running or paused
  qmp.py cont            resume a paused VM (QEMU pauses it when the host disk fills)
  qmp.py tap X Y          touch the screen briefly (needs TOUCH=1 ci/vm/start.sh)
  qmp.py swipe X1 Y1 X2 Y2  touch at (X1,Y1), slide to (X2,Y2), lift
  qmp.py wakeup          wake a guest that suspended itself (system_wakeup)
  qmp.py screendump FILE.png [DEVICE [HEAD]]  capture a display (default: the first GPU's
                         first connector)
  qmp.py plug ID [PORT] [DRIVER]  hot-plug a second GPU with a display into a PCIe root port
                         (default hp1; start.sh creates hp1 and hp2). DRIVER defaults to
                         virtio-gpu-pci; bochs-display is a display-only adapter
  qmp.py unplug ID       remove it again
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
    elif cmd == "status":
        print(json.dumps(send(f, {"execute": "query-status"})["return"]))
    elif cmd in ("tap", "swipe"):
        # A touch is a "begin" for its slot, then its position as "data" events, then "end".
        tid = int(time.time() * 10) % 60000 + 1  # a fresh id per touch, as on real hardware
        def mt(kind, axis="x", value=0):
            # QEMU lifts the finger when the tracking id is -1, as evdev does.
            return {"type": "mtt", "data": {"type": kind, "slot": 0,
                                            "tracking-id": -1 if kind == "end" else tid,
                                            "axis": axis, "value": value}}
        def at(x, y):
            return [mt("data", "x", int(float(x) * 32767)), mt("data", "y", int(float(y) * 32767))]
        pts = argv[2:]
        events(f, [mt("begin")] + at(pts[0], pts[1]))
        time.sleep(0.1)
        if cmd == "swipe":
            x1, y1, x2, y2 = (float(v) for v in pts[:4])
            for i in range(1, 11):
                events(f, [mt("update")] + at(x1 + (x2 - x1) * i / 10, y1 + (y2 - y1) * i / 10))
                time.sleep(0.05)
        events(f, [mt("end")])
    elif cmd == "wakeup":
        print(json.dumps(send(f, {"execute": "system_wakeup"})))
    elif cmd == "cont":
        print(json.dumps(send(f, {"execute": "cont"})))
    elif cmd == "screendump":
        args = {"filename": os.path.abspath(argv[2]), "format": "png"}
        if len(argv) > 3:
            args["device"] = argv[3]
        if len(argv) > 4:
            args["head"] = int(argv[4])
        r = send(f, {"execute": "screendump", "arguments": args})
        if "error" in r:
            sys.exit(r["error"]["desc"])
    elif cmd == "plug":
        port = argv[3] if len(argv) > 3 else "hp1"
        driver = argv[4] if len(argv) > 4 else "virtio-gpu-pci"
        r = send(f, {"execute": "device_add", "arguments": {"driver": driver, "id": argv[2], "bus": port}})
        if "error" in r:
            sys.exit(r["error"]["desc"])
    elif cmd == "unplug":
        r = send(f, {"execute": "device_del", "arguments": {"id": argv[2]}})
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
