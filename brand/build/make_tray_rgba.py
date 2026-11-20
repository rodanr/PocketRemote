#!/usr/bin/env python3
"""Turns the 2x tray PNGs from the logo kit into raw RGBA bytes that the host embeds (no image crate needed).
Usage: python3 brand/build/make_tray_rgba.py"""
import struct, zlib, pathlib

root = pathlib.Path(__file__).resolve().parents[2]
src = root / "brand/kit/dist/tray"
out = root / "desktop/assets/tray"
out.mkdir(parents=True, exist_ok=True)

def decode(path):
    d = path.read_bytes()
    assert d[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, w = 8, b"", 0
    while pos < len(d):
        n, t = struct.unpack(">I4s", d[pos:pos + 8])
        body = d[pos + 8:pos + 8 + n]
        if t == b"IHDR":
            w, h, depth, ctype, _, _, inter = struct.unpack(">IIBBBBB", body)
            assert (depth, ctype, inter) == (8, 6, 0), "need 8-bit RGBA, not interlaced"
        elif t == b"IDAT":
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    bpp, stride = 4, w * 4
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1: line[i] = (line[i] + a) & 255
            elif f == 2: line[i] = (line[i] + b) & 255
            elif f == 3: line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        rows.append(bytes(line)); prev = line
    return w, h, b"".join(rows)

for name in ("tray-waiting-template", "tray-paused-template", "tray-controlled"):
    w, h, px = decode(src / f"{name}@2x.png")
    (out / f"{name.replace('-template', '')}.rgba").write_bytes(px)
    print(name, w, h, len(px))
