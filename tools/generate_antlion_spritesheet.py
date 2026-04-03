#!/usr/bin/env python3
"""Generate a pixel-art antlion spritesheet for the colony simulation.

An antlion larva sits at the bottom of a conical sand pit trap.
This spritesheet has two sections:

**Creature sprite** (32×32 per frame, 8 columns):
  Row 0: Idle / lurking — mandibles slightly moving, partially buried in sand
  Row 1: Attack / grabbing — mandibles wide open, sand flying, body visible
  Row 2: Death — curls up and sinks into sand

**Pit sprite** (96×96 per frame, 8 columns):
  Row 3: Sand pit funnel — concentric rings of sand with subtle grain animation

All sprites are top-down view.
"""

import math
import random
from PIL import Image, ImageDraw

# Creature frames
CREATURE_FRAME = 32
# Pit frames
PIT_FRAME = 96
COLS = 8

# The final image has creature rows (3 rows of 32px) stacked above pit row (1 row of 96px)
# Total height = 3 * 32 + 1 * 96 = 192
# Width = 8 * 96 = 768 (pit is widest)
# But to keep a uniform atlas grid, we'll use 96×96 for everything
# and draw the creature centered in the 96×96 frame.
FRAME = 96
ROWS = 4
WIDTH = FRAME * COLS
HEIGHT = FRAME * ROWS

TRANSPARENT = (0, 0, 0, 0)

# ── Palette ──
# Sand / pit colors
SAND_OUTER = (190, 170, 130, 180)
SAND_MID = (170, 148, 108, 200)
SAND_INNER = (145, 125, 88, 220)
SAND_DEEP = (110, 92, 62, 240)
SAND_DARKEST = (80, 65, 42, 250)
SAND_GRAIN = (200, 182, 142, 160)
SAND_GRAIN_DARK = (140, 120, 85, 180)

# Antlion larva body
BODY_LIGHT = (175, 150, 105, 255)
BODY_MID = (140, 118, 78, 255)
BODY_DARK = (105, 85, 55, 255)
BODY_SEGMENT = (120, 100, 68, 255)

# Mandibles
MANDIBLE_DARK = (90, 60, 35, 255)
MANDIBLE_TIP = (60, 40, 22, 255)

# Eyes (tiny, dark)
EYE_COLOR = (40, 30, 18, 255)

# Attack sand spray
SPRAY_COLOR = (180, 160, 120, 180)

# Death fade
DEATH_BODY = (130, 110, 75, 200)


def filled_ellipse(draw, cx, cy, rx, ry, fill):
    """Draw a filled ellipse centered at (cx, cy)."""
    if rx < 1 or ry < 1:
        return
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)


def draw_antlion_body(draw, cx, cy, scale=1.0, visible_amount=1.0, palette=None):
    """Draw antlion larva body — fat grub-like with segmented abdomen.

    visible_amount: 0.0=fully buried, 1.0=fully visible
    Returns (head_cx, head_cy) for mandible placement.
    """
    if palette is None:
        palette = (BODY_DARK, BODY_MID, BODY_LIGHT, BODY_SEGMENT)
    dark, mid, light, segment = palette
    s = scale

    # Abdomen (rear, widest part) — only visible if visible_amount > 0.3
    if visible_amount > 0.3:
        abd_alpha = min(1.0, (visible_amount - 0.3) / 0.4)
        abd_cy = cy + int(5 * s)
        rx = int(7 * s * abd_alpha)
        ry = int(5 * s * abd_alpha)
        filled_ellipse(draw, cx, abd_cy, rx, ry, dark)
        filled_ellipse(draw, cx, abd_cy, max(1, rx - 1), max(1, ry - 1), mid)

        # Segment lines
        for seg in range(3):
            seg_y = abd_cy - int((2 - seg) * 2.5 * s * abd_alpha)
            seg_rx = int((5 - seg * 0.5) * s * abd_alpha)
            if seg_rx > 0:
                draw.line([
                    (cx - seg_rx, seg_y),
                    (cx + seg_rx, seg_y)
                ], fill=segment, width=1)

    # Thorax (mid section)
    if visible_amount > 0.15:
        thor_alpha = min(1.0, (visible_amount - 0.15) / 0.3)
        thor_cy = cy
        rx = int(5 * s * thor_alpha)
        ry = int(4 * s * thor_alpha)
        filled_ellipse(draw, cx, thor_cy, rx, ry, mid)
        filled_ellipse(draw, cx, thor_cy, max(1, rx - 1), max(1, ry - 1), light)

    # Head (always at least partially visible)
    head_cy = cy - int(5 * s)
    head_rx = int(4 * s * max(0.5, visible_amount))
    head_ry = int(3 * s * max(0.5, visible_amount))
    filled_ellipse(draw, cx, head_cy, head_rx, head_ry, dark)
    filled_ellipse(draw, cx, head_cy, max(1, head_rx - 1), max(1, head_ry - 1), mid)

    # Eyes
    if visible_amount > 0.2:
        draw.point((cx - int(2 * s), head_cy - int(1 * s)), fill=EYE_COLOR)
        draw.point((cx + int(2 * s), head_cy - int(1 * s)), fill=EYE_COLOR)

    return cx, head_cy


def draw_mandibles(draw, head_cx, head_cy, scale=1.0, open_amount=0.3):
    """Draw curved mandibles. open_amount: 0.0=closed, 1.0=wide open."""
    s = scale
    spread = int(5 * s * open_amount)
    length = int(5 * s + 2 * s * open_amount)

    # Left mandible
    draw.line([
        (head_cx - int(2 * s), head_cy),
        (head_cx - spread - int(1 * s), head_cy - length)
    ], fill=MANDIBLE_DARK, width=1)
    draw.point((head_cx - spread - int(1 * s), head_cy - length), fill=MANDIBLE_TIP)

    # Right mandible
    draw.line([
        (head_cx + int(2 * s), head_cy),
        (head_cx + spread + int(1 * s), head_cy - length)
    ], fill=MANDIBLE_DARK, width=1)
    draw.point((head_cx + spread + int(1 * s), head_cy - length), fill=MANDIBLE_TIP)


def draw_sand_spray(draw, cx, cy, frame_idx, scale=1.0):
    """Draw sand particles being flung upward during attack."""
    rng = random.Random(frame_idx * 97 + 43)
    num = 12 + frame_idx * 2

    for _ in range(num):
        angle = rng.uniform(-math.pi * 0.8, -math.pi * 0.2)  # upward arc
        dist = rng.uniform(4, 14) * scale
        px = cx + int(dist * math.cos(angle))
        py = cy + int(dist * math.sin(angle))
        alpha = rng.randint(120, 200)
        color = (
            rng.randint(160, 200),
            rng.randint(140, 180),
            rng.randint(100, 140),
            alpha,
        )
        draw.point((px, py), fill=color)


def render_antlion_idle(img, row):
    """Row 0: Lurking — mostly buried, mandibles twitch."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2, FRAME // 2 + 4

        # Mandible twitch cycle
        phase = math.sin(col / 8.0 * math.pi * 2)
        open_amt = 0.25 + 0.15 * phase

        # Mostly buried
        visible = 0.35 + 0.05 * phase

        # Small sand disturbance around the creature
        rng = random.Random(col * 31 + 5)
        for _ in range(6):
            sx = cx + rng.randint(-8, 8)
            sy = cy - rng.randint(2, 10)
            draw.point((sx, sy), fill=SAND_GRAIN_DARK)

        head_cx, head_cy = draw_antlion_body(draw, cx, cy, scale=1.0, visible_amount=visible)
        draw_mandibles(draw, head_cx, head_cy, open_amount=open_amt)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_antlion_attack(img, row):
    """Row 1: Attack — body emerges, mandibles open wide, sand sprays."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2, FRAME // 2 + 4

        # Attack phases: 0-2 emerge, 3-5 grab, 6-7 retract
        if col < 3:
            visible = 0.35 + col * 0.2
            open_amt = 0.3 + col * 0.2
        elif col < 6:
            visible = 0.85 + (col - 3) * 0.05
            open_amt = 0.8 + (col - 3) * 0.07
        else:
            visible = 0.95 - (col - 5) * 0.2
            open_amt = 1.0 - (col - 5) * 0.25

        # Sand spray during attack frames
        if 2 <= col <= 6:
            draw_sand_spray(draw, cx, cy - 8, col)

        head_cx, head_cy = draw_antlion_body(draw, cx, cy, scale=1.0, visible_amount=visible)
        draw_mandibles(draw, head_cx, head_cy, open_amount=open_amt)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_antlion_death(img, row):
    """Row 2: Death — body exposed briefly then sinks into sand."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2, FRAME // 2 + 4

        # Sink: starts visible, gradually disappears
        visible = max(0.05, 0.9 - col * 0.12)
        open_amt = max(0.0, 0.6 - col * 0.1)

        # Fade alpha
        alpha_mult = max(0.3, 1.0 - col * 0.1)
        death_pal = (
            tuple(int(c * alpha_mult) if i < 3 else c for i, c in enumerate(BODY_DARK)),
            tuple(int(c * alpha_mult) if i < 3 else c for i, c in enumerate(BODY_MID)),
            tuple(int(c * alpha_mult) if i < 3 else c for i, c in enumerate(BODY_LIGHT)),
            tuple(int(c * alpha_mult) if i < 3 else c for i, c in enumerate(BODY_SEGMENT)),
        )

        # Sand filling in
        rng = random.Random(col * 61 + 17)
        num_fill = col * 4
        for _ in range(num_fill):
            sx = cx + rng.randint(-6, 6)
            sy = cy + rng.randint(-8, 4)
            draw.point((sx, sy), fill=SAND_MID)

        head_cx, head_cy = draw_antlion_body(
            draw, cx, cy, scale=1.0, visible_amount=visible, palette=death_pal
        )
        draw_mandibles(draw, head_cx, head_cy, open_amount=open_amt)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_pit(img, row):
    """Row 3: Sand pit funnel — concentric rings with grain animation."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2, FRAME // 2

        # Concentric rings from outer to inner (funnel shape)
        rings = [
            (42, SAND_OUTER),
            (36, SAND_MID),
            (28, SAND_INNER),
            (20, SAND_DEEP),
            (12, SAND_DARKEST),
        ]
        for radius, color in rings:
            # Slight irregularity per frame
            rng = random.Random(col * 41 + radius)
            rx_off = rng.randint(-1, 1)
            ry_off = rng.randint(-1, 1)
            filled_ellipse(draw, cx, cy, radius + rx_off, radius - 2 + ry_off, color)

        # Sand grain texture — shifts per frame for subtle movement
        rng = random.Random(col * 83 + 29)
        for _ in range(80):
            angle = rng.uniform(0, math.pi * 2)
            dist = rng.uniform(3, 40)
            gx = cx + int(dist * math.cos(angle))
            gy = cy + int(dist * math.sin(angle))

            # Grains spiral inward slightly per frame (sand sliding)
            drift = (col / 8.0) * 2.0
            gx -= int(drift * math.cos(angle) * 0.3)
            gy -= int(drift * math.sin(angle) * 0.3)

            if 0 <= gx < FRAME and 0 <= gy < FRAME:
                # Darker grains near center
                if dist < 15:
                    color = SAND_GRAIN_DARK
                else:
                    color = SAND_GRAIN
                draw.point((gx, gy), fill=color)

        # Dark center void
        filled_ellipse(draw, cx, cy, 5, 4, SAND_DARKEST)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def main():
    img = Image.new("RGBA", (WIDTH, HEIGHT), TRANSPARENT)

    render_antlion_idle(img, row=0)
    render_antlion_attack(img, row=1)
    render_antlion_death(img, row=2)
    render_pit(img, row=3)

    out_path = "assets/antlion_spritesheet.png"
    import os
    os.makedirs("assets", exist_ok=True)
    img.save(out_path)
    print(f"Antlion spritesheet saved to {out_path}  ({WIDTH}×{HEIGHT}, {COLS}×{ROWS} frames of {FRAME}×{FRAME})")


if __name__ == "__main__":
    main()
