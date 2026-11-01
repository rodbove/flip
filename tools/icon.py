import math
import os
import struct
import subprocess
import sys
import tempfile
import zlib

SIZE = 1024


def rounded(px, py, x0, y0, x1, y1, r):
    cx = min(max(px, x0 + r), x1 - r)
    cy = min(max(py, y0 + r), y1 - r)
    inside = x0 + r <= px <= x1 - r or y0 + r <= py <= y1 - r
    d = math.hypot(px - cx, py - cy) - r
    if x0 <= px <= x1 and y0 <= py <= y1 and inside:
        d = -min(px - x0, x1 - px, py - y0, y1 - py)
    return d


def circle(px, py, cx, cy, r):
    return math.hypot(px - cx, py - cy) - r


def cover(d):
    return min(max(0.5 - d, 0.0), 1.0)


BODY = (157, 158, 153)
SIDE = (108, 109, 105)
SCREEN = (13, 14, 15)
ORANGE = (255, 91, 20)
PAD = (221, 221, 215)
KEY = (42, 42, 42)


def pixel(x, y):
    color = (0, 0, 0)
    layers = [
        (rounded(x, y, 104, 140, 920, 936, 150), SIDE),
        (rounded(x, y, 104, 104, 920, 900, 150), BODY),
        (rounded(x, y, 200, 200, 824, 420, 40), SCREEN),
    ]
    for i in range(3):
        for j in range(3):
            x0 = 200 + j * 216
            y0 = 480 + i * 128
            layers.append((rounded(x, y, x0, y0 + 10, x0 + 192, y0 + 118, 22), SIDE))
            fill = ORANGE if (i, j) == (0, 0) else PAD
            layers.append((rounded(x, y, x0, y0, x0 + 192, y0 + 108, 22), fill))
    layers.append((rounded(x, y, 260, 268, 300, 360, 6), ORANGE))
    layers.append((rounded(x, y, 260, 268, 360, 300, 6), ORANGE))
    layers.append((rounded(x, y, 260, 300, 340, 328, 6), ORANGE))
    for k in range(4):
        layers.append((circle(x, y, 560 + k * 60, 314, 14), ORANGE if k == 1 else (60, 30, 18)))
    alpha_total = 0.0
    for d, c in layers:
        a = cover(d)
        if a > 0:
            color = tuple(color[i] * (1 - a) + c[i] * a for i in range(3))
            alpha_total = alpha_total + a * (1 - alpha_total)
    if alpha_total > 0:
        shade = 1.0 - 0.08 * (y - 104) / 800.0 if y > 104 else 1.0
        color = tuple(min(255, v * shade) for v in color)
    return color, alpha_total


def png(path, size):
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            c, a = pixel(x * SIZE / size + 0.5, y * SIZE / size + 0.5)
            row += bytes([int(c[0]), int(c[1]), int(c[2]), int(round(a * 255))])
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    head = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", head) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def main():
    out = sys.argv[1]
    with tempfile.TemporaryDirectory() as tmp:
        iconset = os.path.join(tmp, "Flip.iconset")
        os.makedirs(iconset)
        base = os.path.join(tmp, "base.png")
        png(base, 1024)
        for s in [16, 32, 128, 256, 512]:
            for scale in [1, 2]:
                name = f"icon_{s}x{s}{'@2x' if scale == 2 else ''}.png"
                subprocess.run(["sips", "-z", str(s * scale), str(s * scale), base, "--out", os.path.join(iconset, name)], check=True, capture_output=True)
        subprocess.run(["iconutil", "-c", "icns", iconset, "-o", out], check=True)


if __name__ == "__main__":
    main()
