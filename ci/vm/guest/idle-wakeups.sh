#!/bin/sh
# Runs inside the VM. Reports how often the idle compositor wakes up, as context switches per
# second over a window, plus CPU time used. Start a session first (drm-up.sh) and let it settle.
WINDOW=${1:-10}
PID=$(pgrep -f '[t]arget/release/patina-compositor' | head -1)
[ -n "$PID" ] || { echo "no compositor running"; exit 1; }
switches() { awk '/^voluntary_ctxt_switches|^nonvoluntary_ctxt_switches/ {s+=$2} END {print s}' "/proc/$PID/status"; }
cpu() { awk '{print $14+$15}' "/proc/$PID/stat"; }
a=$(switches); c1=$(cpu)
sleep "$WINDOW"
b=$(switches); c2=$(cpu)
ticks=$(getconf CLK_TCK)
echo "compositor pid $PID: $(( (b - a) / WINDOW )) context switches/s, $(( (c2 - c1) * 1000 / ticks / WINDOW )) ms CPU per second, over ${WINDOW}s"
