# make_logo.py - generates assets/logo.svg, assets/icon.png and assets/icon.ico from one geometry.
# Usage: python tools/make_logo.py   (needs Pillow)
from pathlib import Path
from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parent.parent / "assets"
S = 512  # design canvas

BG_TOP, BG_BOT = (38, 42, 54), (16, 17, 22)
ACCENT = (94, 156, 255)
BASE_Y = 300

# EQ curve: bass lift, mid dip, treble lift (cubic bezier segments)
SEGS = [
    ((92, 300), (140, 300), (150, 182), (196, 182)),
    ((196, 182), (244, 182), (236, 330), (280, 330)),
    ((280, 330), (322, 330), (334, 228), (420, 216)),
]
NODES = [(196, 182), (280, 330), (420, 216)]


def bez(p0, p1, p2, p3, t):
    u = 1 - t
    return tuple(u**3 * a + 3 * u * u * t * b + 3 * u * t * t * c + t**3 * d for a, b, c, d in zip(p0, p1, p2, p3))


def curve_points(n=80):
    pts = []
    for s in SEGS:
        pts += [bez(*s, i / n) for i in range(n + (s is SEGS[-1]))]
    return pts


def svg():
    d = f"M{SEGS[0][0][0]} {SEGS[0][0][1]} " + " ".join(f"C{a[0]} {a[1]} {b[0]} {b[1]} {c[0]} {c[1]}" for _, a, b, c in SEGS)
    x0, x1 = SEGS[0][0][0], SEGS[-1][3][0]
    nodes = "\n  ".join(
        f'<circle cx="{x}" cy="{y}" r="17" fill="#121318" stroke="#ffffff" stroke-width="9"/>' for x, y in NODES
    )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {S} {S}" width="{S}" height="{S}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="rgb{BG_TOP}"/><stop offset="1" stop-color="rgb{BG_BOT}"/>
    </linearGradient>
    <linearGradient id="fill" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="rgb{ACCENT}" stop-opacity="0.35"/><stop offset="1" stop-color="rgb{ACCENT}" stop-opacity="0"/>
    </linearGradient>
  </defs>
  <rect width="{S}" height="{S}" rx="116" fill="url(#bg)"/>
  <line x1="{x0}" y1="{BASE_Y}" x2="{x1}" y2="{BASE_Y}" stroke="#ffffff" stroke-opacity="0.14" stroke-width="6" stroke-linecap="round"/>
  <path d="{d} L{x1} 420 L{x0} 420 Z" fill="url(#fill)"/>
  <path d="{d}" fill="none" stroke="rgb{ACCENT}" stroke-width="22" stroke-linecap="round" stroke-linejoin="round"/>
  {nodes}
</svg>
"""


def raster(size):
    k = 4 * size / S  # supersample
    W = int(S * k)
    img = Image.new("RGBA", (W, W), (0, 0, 0, 0))

    bg = Image.new("RGBA", (W, W))
    gd = ImageDraw.Draw(bg)
    for y in range(W):
        t = y / (W - 1)
        gd.line([(0, y), (W, y)], fill=tuple(int(a + (b - a) * t) for a, b in zip(BG_TOP, BG_BOT)) + (255,))
    mask = Image.new("L", (W, W), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, W - 1, W - 1], radius=int(116 * k), fill=255)
    img.paste(bg, (0, 0), mask)

    pts = [(x * k, y * k) for x, y in curve_points()]
    x0, x1 = SEGS[0][0][0] * k, SEGS[-1][3][0] * k

    # area fill fading downward
    fill = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    ImageDraw.Draw(fill).polygon(pts + [(x1, 420 * k), (x0, 420 * k)], fill=ACCENT + (255,))
    fade = Image.new("L", (W, W), 0)
    fd = ImageDraw.Draw(fade)
    top, bot = int(182 * k), int(400 * k)
    for y in range(top, bot):
        fd.line([(0, y), (W, y)], fill=int(70 * (1 - (y - top) / (bot - top)) ** 1.6))
    fill.putalpha(Image.composite(fade, Image.new("L", (W, W), 0), fill.getchannel("A")))
    img.alpha_composite(fill)

    base = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    ImageDraw.Draw(base).line([(x0, BASE_Y * k), (x1, BASE_Y * k)], fill=(255, 255, 255, 36), width=int(6 * k))
    img.alpha_composite(base)

    # stroke by stamping discs along a dense sampling: smooth edges, round caps
    d = ImageDraw.Draw(img)
    r = 11 * k
    for x, y in [(x * k, y * k) for x, y in curve_points(600)]:
        d.ellipse([x - r, y - r, x + r, y + r], fill=ACCENT + (255,))
    for x, y in NODES:
        r, sw = 17 * k, 9 * k
        d.ellipse([x * k - r - sw / 2, y * k - r - sw / 2, x * k + r + sw / 2, y * k + r + sw / 2], fill=(255, 255, 255, 255))
        d.ellipse([x * k - r + sw / 2, y * k - r + sw / 2, x * k + r - sw / 2, y * k + r - sw / 2], fill=(18, 19, 24, 255))
    return img.resize((size, size), Image.LANCZOS)


if __name__ == "__main__":
    OUT.mkdir(exist_ok=True)
    (OUT / "logo.svg").write_text(svg(), encoding="utf-8")
    big = raster(512)
    big.save(OUT / "logo.png")
    raster(256).save(OUT / "icon.png")
    big.save(OUT / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("wrote", ", ".join(p.name for p in sorted(OUT.iterdir())))
