# Toolkit spike: iced + iced_layershell

Question: can iced draw the Patina panel as a layer-shell client at design fidelity?
Answer: yes, with two caveats (blur is the compositor's job; software rendering needs care).

Tested on Debian 12 aarch64 (Docker, then a QEMU VM), against headless sway and against the nested Patina compositor.
Screenshots: `panel-wgpu-gl.png` (panel on sway), `nested-compositor-panel.png` (panel and windows in `patina-compositor`).

## Works
- 36 px top panel anchored top/left/right with a 36 px exclusive zone, built only from `patina-tokens`.
- Both bundled fonts load (Plus Jakarta Sans, JetBrains Mono); the layout matches `TopPanel` in the design system.
- Translucent surface composites correctly on the wgpu path: panel pixel `#1B2B47` against an expected `#1A2A45` over the wallpaper.
- 180 ms `cubic-bezier(.2,.8,.2,1)` easing implemented and unit-tested. The frame subscription only runs while animating.

## Findings
1. **Blur cannot be done by the panel.** A client cannot read pixels behind its own surface. Glass must be drawn by the compositor behind layer surfaces (keyed on the `patina-panel` namespace, or via `ext-background-effect-v1` if Smithay supports it; check in Milestone 1). The panel only supplies translucent ink.
2. **tiny-skia has no alpha here.** The software renderer produced an opaque surface (ink at 72% over black). No translucency, so no glass. Fine for a low-spec mode only if we accept a solid panel, or we fix the shm format.
3. **wgpu on software Vulkan fails.** lavapipe lacks `SHADER_FLOAT16_IN_FLOAT32`, which iced_wgpu's quad shader needs, and the panel panics at startup. wgpu's OpenGL backend on llvmpipe works (`WGPU_BACKEND=gl`). Low-spec and VM sessions need an explicit renderer fallback chain, not a crash.
4. **Variable fonts:** Semibold and Bold looked correct in the screenshot, but this needs a closer weight check on a GPU.
5. **No OpenType features in iced text** (no `tnum`). The clock uses JetBrains Mono, which is tabular anyway.
6. **Dependency drift.** `iced_layershell` 0.19.1 does not build against `winit-core 0.31.0-beta.3`. `Cargo.lock` pins `winit-core` and `winit-common` to `0.31.0-beta.2`. Build with `--locked` and do not run a blanket `cargo update`.

## Verified in the QEMU VM (Milestone 1 compositor)
- The panel, with real Lucide icons, runs inside `patina-compositor` as a layer-shell client and matches the design.
- Pointer input reaches the panel through sway and the nested compositor: clicking workspace 3 highlights it.
- Click-to-raise and focus work for xdg-shell windows.
- `iced_layershell` needs the compositor to offer `wp_viewporter` and to send the initial layer `configure` itself; without them the panel waits or panics.

## Notes from window management work
- Variable-font weights work in cosmic-text (ink coverage rises from weight 200 to 800), but weight 700 measured a much wider line than 600 or 800 (123 px against 99 px for the same text). Check before using bold text.
- Smithay does not send the initial `configure` for toplevels or layer surfaces; the commit handler must. Keyboard focus before the first configure crashes some clients (foot).

## Not verified
- GPU rendering: the VM's virtio-gpu has no 3D, so everything ran on llvmpipe. A real GPU is untested.
- Frame times, x86_64, and a real aarch64 board.
- The easing animation on screen (it is unit-tested).

## Recommendation
Keep iced + iced_layershell for the shell, provided the QEMU VM test (virtio-gpu) confirms input and GPU rendering. Plan for compositor-side blur and a renderer fallback chain. Fallback if it fails: custom wgpu layer.
