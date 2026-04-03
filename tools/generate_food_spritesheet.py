#!/usr/bin/env python3
"""Generate a pixel-art food spritesheet inspired by Castlevania wall meat.

Layout (32×32 per frame, 8 columns):
  Row 0: Food variants (turkey leg, ham, roast, steak, drumstick, fruit, cheese, bread)

Each food item sits on a small plate/surface and has that classic NES/SNES
pixel-art look — bold outlines, warm cooked-meat tones, highlights.
"""

from PIL import Image, ImageDraw
import math

FRAME = 32
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
    cx, cy = 16, 16

    draw_plate(draw, cx, cy + 5)

    # Bone handle (lower-left, angled)
    bone_pts = [(cx + 6, cy + 4), (cx + 10, cy + 8)]
    draw.line(bone_pts, fill=BONE_MID, width=3)
    draw.line(bone_pts, fill=BONE_LIGHT, width=1)
    # Bone knob
    filled_ellipse(draw, cx + 11, cy + 9, 2, 2, BONE_MID)
    filled_ellipse(draw, cx + 11, cy + 9, 1, 1, BONE_LIGHT)

    # Main meat body (big drumstick shape)
    outline_ellipse(draw, cx - 1, cy, 9, 7, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx - 1, cy, 8, 6, SKIN_MID)
    filled_ellipse(draw, cx - 2, cy - 1, 6, 4, MEAT_MID)
    filled_ellipse(draw, cx - 3, cy - 2, 4, 3, MEAT_LIGHT)

    # Crispy highlight
    filled_ellipse(draw, cx - 4, cy - 3, 2, 1, MEAT_HIGHLIGHT)

    # Bite mark (dark crescent on right side)
    filled_ellipse(draw, cx + 4, cy + 1, 3, 3, MEAT_DARK)
    filled_ellipse(draw, cx + 5, cy + 1, 2, 2, MEAT_MID)

    draw_sparkle(draw, cx - 6, cy - 5)


def draw_ham(frame):
    """A big ham / pork roast on a plate."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 16

    draw_plate(draw, cx, cy + 5)

    # Ham body — wider than tall
    outline_ellipse(draw, cx, cy - 1, 10, 6, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx, cy - 1, 9, 5, SKIN_MID)
    filled_ellipse(draw, cx - 1, cy - 2, 7, 4, MEAT_MID)
    filled_ellipse(draw, cx - 1, cy - 3, 5, 2, MEAT_LIGHT)
    filled_ellipse(draw, cx - 2, cy - 4, 3, 1, MEAT_HIGHLIGHT)

    # Cross-hatch score marks on the skin
    for i in range(-2, 3):
        x = cx + i * 3
        draw.line([(x, cy + 1), (x + 2, cy + 3)], fill=SKIN_DARK, width=1)

    # Bone end sticking out left
    draw.line([(cx - 10, cy - 1), (cx - 13, cy - 1)], fill=BONE_MID, width=2)
    filled_ellipse(draw, cx - 13, cy - 1, 1, 1, BONE_LIGHT)

    draw_sparkle(draw, cx + 5, cy - 6)


def draw_whole_roast(frame):
    """A whole roast chicken / turkey on a plate — the iconic wall meat."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 15

    draw_plate(draw, cx, cy + 6)

    # Body — big oval
    outline_ellipse(draw, cx, cy, 10, 7, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx, cy, 9, 6, SKIN_MID)
    filled_ellipse(draw, cx, cy - 1, 7, 4, SKIN_LIGHT)
    filled_ellipse(draw, cx, cy - 2, 5, 3, MEAT_LIGHT)
    filled_ellipse(draw, cx - 1, cy - 3, 3, 1, MEAT_HIGHLIGHT)

    # Two drumsticks poking out the sides
    # Left drumstick
    draw.line([(cx - 7, cy + 3), (cx - 11, cy + 6)], fill=SKIN_DARK, width=2)
    filled_ellipse(draw, cx - 11, cy + 6, 1, 1, BONE_LIGHT)
    # Right drumstick
    draw.line([(cx + 7, cy + 3), (cx + 11, cy + 6)], fill=SKIN_DARK, width=2)
    filled_ellipse(draw, cx + 11, cy + 6, 1, 1, BONE_LIGHT)

    draw_sparkle(draw, cx - 3, cy - 7)
    draw_sparkle(draw, cx + 6, cy - 4)


def draw_steak(frame):
    """A thick T-bone steak."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 16

    draw_plate(draw, cx, cy + 5)

    # Steak body — slightly irregular
    outline_ellipse(draw, cx, cy - 1, 9, 6, OUTLINE, MEAT_DARK)
    filled_ellipse(draw, cx, cy - 1, 8, 5, MEAT_MID)
    filled_ellipse(draw, cx - 1, cy - 2, 6, 3, MEAT_LIGHT)
    filled_ellipse(draw, cx - 2, cy - 3, 3, 1, MEAT_HIGHLIGHT)

    # Grill marks
    for i in range(-2, 3):
        x = cx + i * 3
        draw.line([(x - 1, cy + 2), (x + 2, cy - 1)], fill=SKIN_DARK, width=1)

    # T-bone
    draw.line([(cx, cy - 5), (cx, cy + 3)], fill=BONE_MID, width=1)
    draw.line([(cx - 3, cy - 2), (cx + 3, cy - 2)], fill=BONE_MID, width=1)

    # Fat edge on top
    filled_ellipse(draw, cx, cy - 5, 5, 1, MEAT_HIGHLIGHT)

    draw_sparkle(draw, cx + 6, cy - 5)


def draw_drumstick(frame):
    """A single small drumstick — simpler, for small food drops."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 16

    draw_plate(draw, cx, cy + 4)

    # Bone
    draw.line([(cx + 3, cy + 2), (cx + 8, cy + 5)], fill=BONE_MID, width=2)
    filled_ellipse(draw, cx + 9, cy + 5, 2, 2, BONE_MID)
    filled_ellipse(draw, cx + 9, cy + 5, 1, 1, BONE_LIGHT)

    # Meat
    outline_ellipse(draw, cx - 1, cy - 1, 7, 5, OUTLINE, SKIN_DARK)
    filled_ellipse(draw, cx - 1, cy - 1, 6, 4, SKIN_MID)
    filled_ellipse(draw, cx - 2, cy - 2, 4, 3, MEAT_MID)
    filled_ellipse(draw, cx - 3, cy - 3, 2, 1, MEAT_LIGHT)

    draw_sparkle(draw, cx - 5, cy - 5)


def draw_apple(frame):
    """A shiny red apple — classic Castlevania health pickup."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 17

    # Apple body
    outline_ellipse(draw, cx, cy, 7, 7, OUTLINE, APPLE_DARK)
    filled_ellipse(draw, cx, cy, 6, 6, APPLE_RED)
    filled_ellipse(draw, cx - 1, cy - 1, 4, 4, APPLE_HIGHLIGHT)
    filled_ellipse(draw, cx - 2, cy - 2, 2, 2, (240, 120, 90, 255))

    # Stem
    draw.line([(cx, cy - 7), (cx + 1, cy - 10)], fill=SKIN_DARK, width=1)

    # Leaf
    filled_ellipse(draw, cx + 2, cy - 9, 2, 1, LEAF_GREEN)
    draw.point((cx + 3, cy - 10), fill=LEAF_DARK)

    # Highlight dot
    draw.point((cx - 3, cy - 4), fill=SPARKLE)
    draw.point((cx - 2, cy - 3), fill=SPARKLE)

    draw_sparkle(draw, cx + 5, cy - 6)


def draw_cheese(frame):
    """A wedge of cheese with holes."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 16

    draw_plate(draw, cx, cy + 5)

    # Cheese wedge — triangle-ish shape
    pts = [
        (cx - 8, cy + 3),   # bottom-left
        (cx + 8, cy + 3),   # bottom-right
        (cx + 4, cy - 7),   # top-right (wedge point)
    ]
    draw.polygon(pts, fill=CHEESE_MID, outline=OUTLINE)

    # Inner face (cut side)
    inner_pts = [
        (cx - 7, cy + 2),
        (cx + 7, cy + 2),
        (cx + 3, cy - 6),
    ]
    draw.polygon(inner_pts, fill=CHEESE_LIGHT)

    # Cheese holes
    filled_ellipse(draw, cx - 2, cy, 2, 2, CHEESE_HOLE)
    filled_ellipse(draw, cx + 3, cy - 1, 1, 1, CHEESE_HOLE)
    filled_ellipse(draw, cx, cy - 3, 1, 1, CHEESE_HOLE)

    # Top edge highlight
    draw.line([(cx - 6, cy + 2), (cx + 2, cy - 6)], fill=CHEESE_LIGHT, width=1)

    draw_sparkle(draw, cx - 4, cy - 6)


def draw_bread(frame):
    """A loaf of bread, torn open."""
    draw = ImageDraw.Draw(frame)
    cx, cy = 16, 16

    draw_plate(draw, cx, cy + 5)

    # Bread body — oval loaf
    outline_ellipse(draw, cx, cy - 1, 10, 5, OUTLINE, BREAD_DARK)
    filled_ellipse(draw, cx, cy - 1, 9, 4, BREAD_MID)
    filled_ellipse(draw, cx, cy - 2, 7, 3, BREAD_LIGHT)

    # Score line on top (bakery cut)
    draw.line([(cx - 5, cy - 3), (cx + 5, cy - 3)], fill=BREAD_DARK, width=1)

    # Torn/exposed bread interior on right
    filled_ellipse(draw, cx + 6, cy - 1, 3, 3, BREAD_INNER)
    filled_ellipse(draw, cx + 6, cy - 1, 2, 2, (240, 225, 180, 255))

    # Crust highlight on top
    filled_ellipse(draw, cx - 2, cy - 4, 4, 1, BREAD_LIGHT)

    draw_sparkle(draw, cx - 6, cy - 5)


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
