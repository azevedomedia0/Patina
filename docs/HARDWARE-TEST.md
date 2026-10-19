# Testing on real hardware

The build VM has software rendering only and cannot wake from suspend, so these need a real
Linux machine (Debian 12/13, Ubuntu 22.04/24.04 or Linux Mint 21/22).

Install the package for your release from a build (`docs/PACKAGING.md`), log out, pick
**Patina** at the login screen. If anything goes wrong, the log is
`~/.local/state/patina/session.log` (the previous one is `session.log.old`).

## Checklist

| Check | How | Expect |
| --- | --- | --- |
| GPU | `grep "using GPU" ~/.local/state/patina/session.log` | your GPU's `/dev/dri/cardN`, no `cannot use display` |
| Smoothness | drag a window around quickly | smooth on a real GPU (the VM manages about 30 fps) |
| Blur | drag a window over the bright wallpaper glow | a soft haze through the titlebar and panel |
| Idle | `top` while nothing happens | patina-compositor near 0% CPU |
| Suspend | close the lid or `systemctl suspend`, then wake | the desktop comes back as it was; typing works |
| VT switch | Ctrl+Alt+F3, then Ctrl+Alt+F1 (or F2/F7, wherever the session is) | the desktop comes back; no stuck keys |
| External monitor | plug in, unplug, plug in again | a panel appears on it; windows on it move to the built-in screen and back |
| HiDPI | a laptop or 4K screen | sharp text and titlebars; scale in the log line `display connected ... scale=` |
| Keyboard layout | any non-US layout set up in the system | the keys type that layout |
| Caps Lock light | press Caps Lock | the light follows |
| Touchscreen | tap and drag | apps respond; dragging a titlebar moves the window |
| Hybrid graphics | a laptop with Intel/AMD plus NVIDIA | both work; the log names each GPU with displays |

Things that are known not to work yet: the launcher, dock, quick settings and notifications
(milestone 3), lock screen and power menu (milestone 4).

When reporting a problem, include the session log and the output of
`ls /sys/class/drm` and `cat /sys/class/drm/card*-*/status`.
