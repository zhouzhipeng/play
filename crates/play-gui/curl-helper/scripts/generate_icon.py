#!/usr/bin/env python3
"""Generate a macOS app icon for Curl Helper."""
import sys
from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
PAD = int(SIZE * 0.10)  # ~10% margin per Apple HIG
INNER = SIZE - PAD * 2  # actual icon content size
RADIUS = int(INNER * 0.22)
out_path = sys.argv[1] if len(sys.argv) > 1 else "icon_1024.png"

# Draw on inner-sized canvas first
inner_img = Image.new("RGBA", (INNER, INNER), (0, 0, 0, 0))
draw = ImageDraw.Draw(inner_img)

# ── Background gradient ──
for y in range(INNER):
    t = y / INNER
    r = int(18 + (26 - 18) * t)
    g = int(20 + (36 - 20) * t)
    b = int(41 + (66 - 41) * t)
    draw.line([(0, y), (INNER, y)], fill=(r, g, b, 255))

# Rounded rect mask
mask_inner = Image.new("L", (INNER, INNER), 0)
ImageDraw.Draw(mask_inner).rounded_rectangle([(0, 0), (INNER, INNER)], radius=RADIUS, fill=255)
inner_img.putalpha(mask_inner)

# ── Subtle grid lines ──
for y in range(40, INNER, 40):
    draw.line([(0, y), (INNER, y)], fill=(255, 255, 255, 8), width=1)

# ── Accent glow ──
glow = Image.new("RGBA", (INNER, INNER), (0, 0, 0, 0))
glow_draw = ImageDraw.Draw(glow)
for rv in range(200, 0, -2):
    alpha = max(0, int(16 * (1 - rv / 200)))
    cx, cy = INNER // 2, INNER // 2
    glow_draw.ellipse([cx - rv, cy - rv, cx + rv, cy + rv], fill=(230, 113, 84, alpha))
inner_img = Image.alpha_composite(inner_img, glow)
draw = ImageDraw.Draw(inner_img)

# ── Fonts ──
def get_font(size, bold=False):
    names = [
        "/System/Library/Fonts/SFMono-Bold.otf" if bold else "/System/Library/Fonts/SFMono-Regular.otf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.dfont",
    ]
    for n in names:
        try:
            return ImageFont.truetype(n, size)
        except (OSError, IOError):
            continue
    return ImageFont.load_default()

def get_light_font(size):
    for n in ["/System/Library/Fonts/SFMono-Light.otf", "/System/Library/Fonts/SFMono-Regular.otf", "/System/Library/Fonts/Menlo.ttc"]:
        try:
            return ImageFont.truetype(n, size)
        except (OSError, IOError):
            continue
    return ImageFont.load_default()

S = INNER  # shorthand
brace_font = get_light_font(int(S * 0.44))
arrow_font = get_font(int(S * 0.23), bold=True)
curl_font = get_font(int(S * 0.10), bold=False)

# ── "{ }" braces ──
teal = (79, 201, 176, 255)
bb = draw.textbbox((0, 0), "{", font=brace_font)
tw = bb[2] - bb[0]
draw.text((S * 0.18 - tw // 2, S * 0.28), "{", fill=teal, font=brace_font)
draw.text((S * 0.78 - tw // 2, S * 0.28), "}", fill=teal, font=brace_font)

# ── ">_" arrow ──
orange = (230, 113, 84, 255)
bb = draw.textbbox((0, 0), ">_", font=arrow_font)
tw = bb[2] - bb[0]
draw.text((S * 0.50 - tw // 2, S * 0.35), ">_", fill=orange, font=arrow_font)

# ── "curl" text ──
light_text = (165, 182, 204, 160)
bb = draw.textbbox((0, 0), "curl", font=curl_font)
tw = bb[2] - bb[0]
draw.text((S * 0.50 - tw // 2, S * 0.78), "curl", fill=light_text, font=curl_font)

# Re-apply mask
masked = Image.new("RGBA", (INNER, INNER), (0, 0, 0, 0))
masked.paste(inner_img, mask=mask_inner)

# ── Place centered on full-size transparent canvas ──
final = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
final.paste(masked, (PAD, PAD))
final.save(out_path, "PNG")
print(f"Icon generated: {out_path}")
