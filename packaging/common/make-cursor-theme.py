#!/usr/bin/env python3
"""Builds the Patina XCursor theme from the design's cursor images.

Run from the repository root; needs Pillow (`pip install pillow`, or python3-pil). Reads
design/assets/cursors/*.png (48 x 48, drawn to be shown at 2.5 device pixels per logical
pixel, so 19 logical pixels) and writes assets/cursors/Patina/, which the packages install to
/usr/share/icons/Patina. The output is committed, so building a package needs no Pillow.

A cursor file holds the image at several nominal sizes. A client asking for size N (24 by
default) gets an image N * 0.8 pixels wide, the same 19 pixels the design shows at 24. The 48
pixel original is the nominal size 60 image and is never scaled up.
"""
import os
import shutil
import struct
import sys

from PIL import Image

SOURCE = "design/assets/cursors"
OUT = "assets/cursors/Patina"
SIZES = (24, 32, 48, 60)  # nominal; 60 is the 48 pixel original
RATIO = 0.8  # image width / nominal size

# file, hotspot in the 48 pixel image, other names the cursor answers to
CURSORS = {
    "default": ("default.png", (5, 3), [
        "left_ptr", "arrow", "top_left_arrow", "context-menu",
    ]),
    "pointer": ("pointer.png", (10, 4), [
        "hand1", "hand2", "pointing_hand", "alias",
        "9d800788f1b08800ae810202380a0822", "e29285e634086352946a0e7090d73106",
    ]),
    "text": ("text.png", (13, 24), ["xterm", "ibeam"]),
    "move": ("move.png", (24, 24), [
        "fleur", "all-scroll", "size_all", "grab", "grabbing", "dnd-move",
    ]),
    "nwse-resize": ("nwse-resize.png", (24, 24), [
        "nw-resize", "se-resize", "size_fdiag", "bd_double_arrow", "top_left_corner",
        "bottom_right_corner", "c7088f0f3e6c8088236ef8e1e3e70000",
    ]),
    "nesw-resize": ("nesw-resize.png", (24, 24), [
        "ne-resize", "sw-resize", "size_bdiag", "fd_double_arrow", "top_right_corner",
        "bottom_left_corner", "fcf1c3c7cd4491d801f1e1c78f100000",
    ]),
    "wait": ("wait.png", (19, 16), [
        "watch", "progress", "left_ptr_watch", "half-busy",
        "08e8e1c95fe2fc01f976f1e063a24ccd", "3ecb610c1bf2410f44200f48c40d3599",
    ]),
    "help": ("help.png", (5, 3), [
        "question_arrow", "left_ptr_help", "whats_this",
        "5c6cd98b3f3ebcb1f9c7f1c204630408", "d9ce0ab605698f320427677b458ad60b",
    ]),
    "not-allowed": ("not-allowed.png", (5, 3), [
        "crossed_circle", "forbidden", "no-drop", "circle", "dnd-none",
        "03b6e0fcb3499374a867c041f52298f0",
    ]),
}

INDEX = """[Icon Theme]
Name=Patina
Comment=The Patina Desktop pointer
Inherits=Adwaita
"""


def image_at(src, nominal):
    """(width, premultiplied RGBA bytes) of `src` for a nominal size."""
    width = round(nominal * RATIO)
    if width != src.width:
        # Premultiplied alpha, so the soft shadow does not get a light fringe when scaled.
        src = src.convert("RGBa").resize((width, width), Image.LANCZOS)
    else:
        src = src.convert("RGBa")
    return width, src.tobytes()


def xcursor(src, hotspot):
    """An XCursor file (the format of libXcursor) holding every nominal size."""
    chunks = []
    for nominal in SIZES:
        width, rgba = image_at(src, nominal)
        xhot = min(width - 1, round(hotspot[0] * width / src.width))
        yhot = min(width - 1, round(hotspot[1] * width / src.height))
        # ARGB, premultiplied, one little-endian 32 bit word per pixel.
        argb = b"".join(
            struct.pack("<I", a << 24 | r << 16 | g << 8 | b)
            for r, g, b, a in zip(rgba[0::4], rgba[1::4], rgba[2::4], rgba[3::4])
        )
        header = struct.pack("<9I", 36, 0xFFFD0002, nominal, 1, width, width, xhot, yhot, 0)
        chunks.append((nominal, header + argb))
    toc_start = 16
    pos = toc_start + 12 * len(chunks)
    out = struct.pack("<4s3I", b"Xcur", 16, 0x10000, len(chunks))
    for nominal, data in chunks:
        out += struct.pack("<3I", 0xFFFD0002, nominal, pos)
        pos += len(data)
    return out + b"".join(data for _, data in chunks)


def main() -> int:
    if not os.path.isdir(SOURCE):
        print(f"run from the repository root ({SOURCE} not found)", file=sys.stderr)
        return 1
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(f"{OUT}/cursors")
    for name, (png, hotspot, aliases) in CURSORS.items():
        src = Image.open(f"{SOURCE}/{png}").convert("RGBA")
        assert src.size == (48, 48), f"{png} is {src.size}, expected 48 x 48"
        with open(f"{OUT}/cursors/{name}", "wb") as f:
            f.write(xcursor(src, hotspot))
        for alias in aliases:
            os.symlink(name, f"{OUT}/cursors/{alias}")
    with open(f"{OUT}/index.theme", "w") as f:
        f.write(INDEX)
    print(f"wrote {OUT}: {len(CURSORS)} cursors, {sum(len(a) for _, _, a in CURSORS.values())} aliases")
    return 0


if __name__ == "__main__":
    sys.exit(main())
