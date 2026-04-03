#!/usr/bin/env python3
"""Generate a pixel-art food spritesheet inspired by Castlevania wall meat.

Layout (48×48 per frame, 8 columns):
  Row 0: Food variants (turkey leg, ham, roast, steak, drumstick, fruit, cheese, bread)

Each food item sits on a small plate/surface and has that classic NES/SNES
pixel-art look — bold outlines, warm cooked-meat tones, highlights.
Big food types (turkey leg, ham, roast, steak) are drawn larger within their frames.
"""

from PIL import Image, ImageDraw
import math

FRAME = 48
COLS = 8
ROWS = 1
WIDTH = FRAME * COLS
HEIGHT = FRAME * ROWS

# Palette – RGBA
TRANSPARENT = (0, 0, 0, 0)

# Meat tones
MEAT_DARK = (120, 45, 20, 255)
MEAT_MID = (170, 75, 35, 255)
MEAT_LIGHT = (210, 130, 60, 255)
MEAT_HIGHLIGHT = (240, 185, 100, 255)

# Crispy / roasted skin
SKIN_DARK = (100, 55, 25, 255)
SKIN_MID = (160, 95, 40, 255)
SKIN_LIGHT = (200, 145, 65, 255)

# Bone
BONE_DARK = (180, 170, 150, 255)
BONE_MID = (210, 200, 180, 255)
BONE_LIGHT = (235, 228, 215, 255)

# Plate / platter
PLATE_DARK = (140, 135, 125, 255)
PLATE_MID = (185, 180, 170, 255)
PLATE_LIGHT = (215, 210, 200, 255)
PLATE_RIM = (160, 155, 145, 255)

# Fruit colors
APPLE_RED = (190, 35, 30, 255)
APPLE_HIGHLIGHT = (230, 80, 60, 255)
APPLE_DARK = (140, 25, 20, 255)
LEAF_GREEN = (60, 140, 40, 255)
LEAF_DARK = (40, 100, 25, 255)

# Cheese
CHEESE_DARK = (200, 160, 40, 255)
CHEESE_MID = (230, 195, 60, 255)
CHEESE_LIGHT = (245, 220, 100, 255)
CHEESE_HOLE = (180, 140, 30, 255)

# Bread
BREAD_DARK = (140, 95, 45, 255)
BREAD_MID = (185, 140, 70, 255)
BREAD_LIGHT = (215, 180, 110, 255)
BREAD_INNER = (230, 210, 160, 255)

# Outline
OUTLINE = (30, 20, 15, 255)

# Glow / sparkle (Castlevania items often have a subtle shine)
SPARKLE = (255, 255, 220, 200)


def filled_ellipse(draw, cx, cy, rx, ry, fill):
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)


def outline_ellipse(draw, cx, cy, rx, ry, outline, fill):
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill, outline=outline)


def draw_plate(draw, cx, cy):
    """Draw a small oval plate at the bottom of the food."""
    filled_ellipse(draw, cx, cy + 2, 10, 3, PLATE_DARK)
    filled_ellipse(draw, cx, cy + 2, 9, 2, PLATE_MID)
    filled_ellipse(draw, cx, cy + 1, 7, 1, PLATE_LIGHT)


def draw_sparkle(draw, x, y):
    """Draw a tiny + shaped sparkle."""
    draw.point((x, y), fill=SPARKLE)
    draw.point((x - 1, y), fill=SPARKLE)
    draw.point((x + 1, y), fill=SPARKLE)
    draw.point((x, y - 1), fill=SPARKLE)
    draw.point((x, y + 1), fill=SPARKLE)


def draw_turkey_leg(frame):
    """Classic Castlevania wall meat — a big roasted turkey leg."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 24, 22

    draw_plate(draw, cx, cy + 8)

    # Bone handle (lower-left, angled)
    bone_pts = [(cx + 9, cy + 5), (cx + 15, cy + 12)]
    draw.line(bone_pts, fill=BONE_MID, width=4)
    draw.line(bone_pts, fill=BONE_LIGHT, width=2)
    # Bone knob
    filled_ellipse(draw, cx + 16, cy + 13, 3, 3, BONE_MID)
    filled_ellipse(draw, cx + 16, cy + 13, 2, 2, BONE_LIGHT)

    # Main meat body (big drumstick shape)
    outline_ellipse(draw, cx - 2, cy, 13, 10, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx - 2, cy, 12, 9, SKIN_MID)
    filled_ellipse(draw, cx - 3, cy - 1, 9, 6, MEAT_MID)
    filled_ellipse(draw, cx - 4, cy - 2, 6, 4, MEAT_LIGHT)

    # Crispy highlight
    filled_ellipse(draw, cx - 6, cy - 4, 3, 2, MEAT_HIGHLIGHT)

    # Bite mark (dark crescent on right side)
    filled_ellipse(draw, cx + 6, cy + 1, 4, 4, MEAT_DARK)
    filled_ellipse(draw, cx + 7, cy + 1, 3, 3, MEAT_MID)

    draw_sparkle(draw, cx - 8, cy - 7)


def draw_ham(frame):
    """A big ham / pork roast on a plate."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 24, 22

    draw_plate(draw, cx, cy + 8)

    # Ham body — wider than tall
    outline_ellipse(draw, cx, cy - 1, 15, 9, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx, cy - 1, 14, 8, SKIN_MID)
    filled_ellipse(draw, cx - 1, cy - 2, 11, 6, MEAT_MID)
    filled_ellipse(draw, cx - 1, cy - 3, 8, 3, MEAT_LIGHT)
    filled_ellipse(draw, cx - 2, cy - 4, 5, 2, MEAT_HIGHLIGHT)

    # Cross-hatch score marks on the skin
    for i in range(-2, 3):
        x = cx + i * 4
        draw.line([(x, cy + 1), (x + 3, cy + 4)], fill=SKIN_DARK, width=1)

    # Bone end sticking out left
    draw.line([(cx - 15, cy - 1), (cx - 20, cy - 1)], fill=BONE_MID, width=3)
    filled_ellipse(draw, cx - 20, cy - 1, 2, 2, BONE_LIGHT)

    draw_sparkle(draw, cx + 8, cy - 8)


def draw_whole_roast(frame):
    """A whole roast chicken / turkey on a plate — the iconic wall meat."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 24, 20

    draw_plate(draw, cx, cy + 10)

    # Body — big oval
    outline_ellipse(draw, cx, cy, 15, 10, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx, cy, 14, 9, SKIN_MID)
    filled_ellipse(draw, cx, cy - 1, 11, 6, SKIN_LIGHT)
    filled_ellipse(draw, cx, cy - 2, 8, 4, MEAT_LIGHT)
    filled_ellipse(draw, cx - 1, cy - 3, 5, 2, MEAT_HIGHLIGHT)

    # Two drumsticks poking out the sides
    # Left drumstick
    draw.line([(cx - 10, cy + 4), (cx - 16, cy + 9)], fill=SKIN_DARK, width=3)
    filled_ellipse(draw, cx - 16, cy + 9, 2, 2, BONE_LIGHT)
    # Right drumstick
    draw.line([(cx + 10, cy + 4), (cx + 16, cy + 9)], fill=SKIN_DARK, width=3)
    filled_ellipse(draw, cx + 16, cy + 9, 2, 2, BONE_LIGHT)

    draw_sparkle(draw, cx - 5, cy - 10)
    draw_sparkle(draw, cx + 9, cy - 6)


def draw_steak(frame):
    """A thick T-bone steak."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 24, 22

    draw_plate(draw, cx, cy + 8)

    # Steak body — slightly irregular
    outline_ellipse(draw, cx, cy - 1, 14, 9, OUTLINE, MEAT_DARK)
    filled_ellipse(draw, cx, cy - 1, 13, 8, MEAT_MID)
    filled_ellipse(draw, cx - 1, cy - 2, 10, 5, MEAT_LIGHT)
    filled_ellipse(draw, cx - 2, cy - 3, 5, 2, MEAT_HIGHLIGHT)

    # Grill marks
    for i in range(-2, 3):
        x = cx + i * 4
        draw.line([(x - 1, cy + 2), (x + 3, cy - 1)], fill=SKIN_DARK, width=1)

    # T-bone
    draw.line([(cx, cy - 7), (cx, cy + 4)], fill=BONE_MID, width=2)
    draw.line([(cx - 5, cy - 2), (cx + 5, cy - 2)], fill=BONE_MID, width=2)

    # Fat edge on top
    filled_ellipse(draw, cx, cy - 7, 8, 2, MEAT_HIGHLIGHT)

    draw_sparkle(draw, cx + 9, cy - 7)


def draw_drumstick(frame):
    """A single small drumstick — simpler, for small food drops."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 20, 20

    draw_plate(draw, cx, cy + 6)

    # Bone
    draw.line([(cx + 4, cy + 3), (cx + 12, cy + 8)], fill=BONE_MID, width=3)
    filled_ellipse(draw, cx + 13, cy + 8, 3, 3, BONE_MID)
    filled_ellipse(draw, cx + 13, cy + 8, 2, 2, BONE_LIGHT)

    # Meat
    outline_ellipse(draw, cx - 1, cy - 1, 10, 7, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx - 1, cy - 1, 9, 6, SKIN_MID)
    filled_ellipse(draw, cx - 2, cy - 2, 6, 4, MEAT_MID)
    filled_ellipse(draw, cx - 3, cy - 3, 3, 2, MEAT_LIGHT)

    draw_sparkle(draw, cx - 7, cy - 7)


def draw_apple(frame):
    """A shiny red apple — classic Castlevania health pickup."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 24, 23

    # Apple body
    outline_ellipse(draw, cx, cy, 10, 10, OUTLINE, APPLE_DARK)
    filled_ellipse(draw, cx, cy, 9, 9, APPLE_RED)
    filled_ellipse(draw, cx - 1, cy - 1, 6, 6, APPLE_HIGHLIGHT)
    filled_ellipse(draw, cx - 2, cy - 2, 3, 3, (240, 120, 90, 255))

    # Stem
    draw.line([(cx, cy - 10), (cx + 1, cy - 14)], fill=SKIN_DARK, width=2)

    # Leaf
    filled_ellipse(draw, cx + 3, cy - 12, 3, 2, LEAF_GREEN)
    draw.point((cx + 5, cy - 13), fill=LEAF_DARK)

    # Highlight dot
    draw.point((cx - 4, cy - 5), fill=SPARKLE)
    draw.point((cx - 3, cy - 4), fill=SPARKLE)

    draw_sparkle(draw, cx + 7, cy - 8)


def draw_cheese(frame):
    """A wedge of cheese with holes."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 22, 22

    draw_plate(draw, cx, cy + 8)

    # Cheese wedge — triangle-ish shape
    pts = [
        (cx - 12, cy + 4),   # bottom-left
        (cx + 12, cy + 4),   # bottom-right
        (cx + 6, cy - 10),   # top-right (wedge point)
    ]
    draw.polygon(pts, fill=CHEESE_MID, outline=OUTLINE)

    # Inner face (cut side)
    inner_pts = [
        (cx - 11, cy + 3),
        (cx + 11, cy + 3),
        (cx + 5, cy - 9),
    ]
    draw.polygon(inner_pts, fill=CHEESE_LIGHT)

    # Cheese holes
    filled_ellipse(draw, cx - 3, cy, 3, 3, CHEESE_HOLE)
    filled_ellipse(draw, cx + 4, cy - 1, 2, 2, CHEESE_HOLE)
    filled_ellipse(draw, cx, cy - 4, 2, 2, CHEESE_HOLE)

    # Top edge highlight
    draw.line([(cx - 9, cy + 2), (cx + 3, cy - 9)], fill=CHEESE_LIGHT, width=2)

    draw_sparkle(draw, cx - 6, cy - 8)


def draw_bread(frame):
    """A loaf of bread, torn open."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 22, 22

    draw_plate(draw, cx, cy + 8)

    # Bread body — oval loaf
    outline_ellipse(draw, cx, cy - 1, 15, 7, OUTLINE, BREAD_DARK)
    filled_ellipse(draw, cx, cy - 1, 14, 6, BREAD_MID)
    filled_ellipse(draw, cx, cy - 2, 11, 4, BREAD_LIGHT)

    # Score line on top (bakery cut)
    draw.line([(cx - 8, cy - 4), (cx + 8, cy - 4)], fill=BREAD_DARK, width=2)

    # Torn/exposed bread interior on right
    filled_ellipse(draw, cx + 9, cy - 1, 4, 4, BREAD_INNER)
    filled_ellipse(draw, cx + 9, cy - 1, 3, 3, (240, 225, 180, 255))

    # Crust highlight on top
    filled_ellipse(draw, cx - 3, cy - 5, 6, 2, BREAD_LIGHT)

    draw_sparkle(draw, cx - 8, cy - 7)


# Ordered list of food drawing functions
FOOD_RENDERERS = [
    draw_turkey_leg,     # 0 — the iconic wall meat
    draw_ham,            # 1
    draw_whole_roast,    # 2
    draw_steak,          # 3
    draw_drumstick,      # 4
    draw_apple,          # 5
    draw_cheese,         # 6
    draw_bread,          # 7
]


def main():
    img = Image.new("RGBA", (WIDTH, HEIGHT), TRANSPARENT)

    for col, renderer in enumerate(FOOD_RENDERERS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        renderer(frame)
        img.paste(frame, (col * FRAME, 0), frame)

    out_path = "assets/food_spritesheet.png"
    import os
    os.makedirs("assets", exist_ok=True)
    img.save(out_path)
    print(f"Food spritesheet saved to {out_path}  ({WIDTH}×{HEIGHT}, {COLS}×{ROWS} frames of {FRAME}×{FRAME})")


if __name__ == "__main__":
    main()
