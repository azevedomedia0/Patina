# Build VM (QEMU)

Linux code (compositor, DRM/KMS, session) builds and runs in a Debian 12 arm64 VM on QEMU with Apple HVF, scripted in `ci/vm/`.

```sh
ci/vm/setup.sh /path/to/debian-12-genericcloud-arm64.qcow2   # once: disk, ssh key, cloud-init seed
ci/vm/start.sh                                                # boots (ssh on 127.0.0.1:2222, QMP socket in ci/vm/state)
ci/vm/sync.sh                                                 # copy the working tree into the VM
ci/vm/ssh.sh '. ~/.cargo/env; cd PatineDE && cargo build --release --locked'
ci/vm/ssh.sh 'sh PatineDE/ci/vm/guest/nested-up.sh'           # sway + nested compositor + panel + test windows
ci/vm/qmp.py move 0.09 0.025 && ci/vm/qmp.py click            # inject pointer/keys from the host
```

Notes
- First boot installs packages through cloud-init (a few minutes). The cloud kernel has no graphics or input drivers, so the VM installs `linux-image-arm64` and GRUB is set to boot it.
- The VM has virtio-gpu without 3D, so OpenGL is llvmpipe. Use `WGPU_BACKEND=gl` for iced apps.
- `ci/vm/state/` (disk, ssh key, firmware vars) is not committed.
- x86_64 builds run in Docker/CI, not here.

## Running the compositor on the VM's display (DRM)

```sh
ci/vm/ssh.sh 'sh PatineDE/ci/vm/guest/drm-up.sh'      # compositor on /dev/dri/card0 via seatd, panel and a terminal
ci/vm/qmp.py screendump out.png                       # what the virtual monitor shows
```
QEMU's `screendump` omits the hardware cursor plane. Start with `PATINA_NO_PLANES=1` to composite the cursor into the frame when you need it in a capture.
