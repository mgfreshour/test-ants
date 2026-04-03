#!/usr/bin/env python3
"""Generate a pixel-art nest entrance spritesheet for the colony simulation.

Layout (64×64 per frame, 8 columns):
  Row 0: Player colony nest — idle ambient (8 frames)
  Row 1: Player colony nest — active / busy (8 frames)
  Row 2: Enemy colony nest — idle ambient (8 frames)
  Row 3: Enemy colony nest — active / busy (8 frames)

Each frame shows a top-down dirt mound with a central entrance hole,
scattered soil particles, and tiny ant silhouettes for the "active" rows.
"""

import math
import random
from PIL import Image, ImageDraw

FRAME = 64
COLS = 8
ROWS = 4
WIDTH = FRAME * COLS
HEIGHT = FRAME * ROWS

TRANSPARENT = (0, 0, 0, 0)

# ── Player colony palette (earthy browns) ──
P_MOUND_OUTER = (100, 72, 48, 255)
P_MOUND_MID = (120, 88, 60, 255)
P_MOUND_INNER = (140, 105, 72, 255)
P_MOUND_HIGHLIGHT = (165, 128, 90, 255)
P_HOLE_OUTER = (35, 24, 14, 255)
P_HOLE_INNER = (15, 10, 5, 255)
P_DIRT_DARK = (90, 65, 42, 255)
P_DIRT_LIGHT = (150, 115, 78, 255)
P_DIRT_PARTICLE = (130, 98, 65, 200)
P_ANT_SILHOUETTE = (40, 28, 20, 240)

# ── Enemy colony palette (reddish-browns) ──
E_MOUND_OUTER = (120, 55, 35, 255)
E_MOUND_MID = (145, 70, 42, 255)
E_MOUND_INNER = (165, 88, 52, 255)
E_MOUND_HIGHLIGHT = (185, 110, 68, 255)
E_HOLE_OUTER = (45, 18, 10, 255)
E_HOLE_INNER = (20, 8, 4, 255)
E_DIRT_DARK = (110, 50, 30, 255)
E_DIRT_LIGHT = (170, 90, 55, 255)
E_DIRT_PARTICLE = (150, 72, 45, 200)
E_ANT_SILHOUETTE = (55, 25, 18, 240)


def filled_ellipse(draw, cx, cy, rx, ry, fill):
    """Draw a filled ellipse centered at (cx, cy)."""
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)


def draw_mound(draw, cx, cy, palette, frame_idx):
    """Draw the dirt mound — an irregular raised circle of soil.

    palette: (outer, mid, inner, highlight, hole_outer, hole_inner)
    """
    outer, mid, inner, highlight, hole_outer, hole_inner = palette

    # Slight wobble to mound shape per frame for organic feel
    rng = random.Random(frame_idx * 37 + 7)
    wobble = [rng.uniform(-0.8, 0.8) for _ in range(8)]

    # Outer mound ring (largest)
    filled_ellipse(draw, cx, cy, 26 + int(wobble[0]), 24 + int(wobble[1]), outer)

    # Mid ring
    filled_ellipse(draw, cx, cy, 22 + int(wobble[2]), 20 + int(wobble[3]), mid)

    # Inner ring
    filled_ellipse(draw, cx, cy, 17 + int(wobble[4]), 15 + int(wobble[5]), inner)

    # Highlight crescent (top-left, simulating light from above-left)
    filled_ellipse(draw, cx - 5, cy - 4, 12, 10, highlight)
    # Cover the bottom of the highlight to make it a crescent
    filled_ellipse(draw, cx - 3, cy, 14, 12, inner)

    # Raised rim texture — small bumps around the mound edge
    for i in range(16):
        angle = (i / 16) * math.pi * 2 + wobble[i % 8] * 0.3
        r = 23 + rng.uniform(-2, 2)
        bx = cx + int(r * math.cos(angle))
        by = cy + int(r * math.sin(angle))
        bump_r = rng.randint(2, 4)
        filled_ellipse(draw, bx, by, bump_r, bump_r - 1, mid if i % 3 else outer)

    # Entrance hole (dark center)
    filled_ellipse(draw, cx + 1, cy + 1, 9, 8, hole_outer)
    filled_ellipse(draw, cx, cy, 7, 6, hole_inner)

    # Hole depth shadow
    filled_ellipse(draw, cx - 1, cy - 1, 5, 4, hole_inner)


def draw_dirt_particles(draw, cx, cy, frame_idx, palette_dark, palette_light, palette_particle):
    """Draw scattered soil grains around the mound that shift slightly per frame."""
    rng = random.Random(frame_idx * 53 + 13)
    num_particles = 24

    for i in range(num_particles):
        angle = rng.uniform(0, math.pi * 2)
        dist = rng.uniform(18, 30)
        # Slight drift per frame
        drift = math.sin(frame_idx * 0.4 + i * 0.7) * 1.5

        px = cx + int(dist * math.cos(angle) + drift)
        py = cy + int(dist * math.sin(angle) + drift * 0.5)

        if 0 <= px < FRAME and 0 <= py < FRAME:
            color = rng.choice([palette_dark, palette_light, palette_particle])
            size = rng.randint(1, 2)
            if size == 1:
                draw.point((px, py), fill=color)
            else:
                draw.rectangle([px, py, px + 1, py + 1], fill=color)


def draw_tiny_ant(draw, cx, cy, angle, color):
    """Draw a 3-4 pixel ant silhouette at given position and angle."""
    # Simple 3-pixel body along the angle
    for seg in range(3):
        dist = seg - 1  # -1, 0, 1 from center
        x = cx + int(dist * 1.5 * math.cos(angle))
        y = cy + int(dist * 1.5 * math.sin(angle))
        draw.point((x, y), fill=color)

    # Tiny legs (just dots to the sides)
    perp_angle = angle + math.pi / 2
    for side in [-1, 1]:
        lx = cx + int(side * 1.5 * math.cos(perp_angle))
        ly = cy + int(side * 1.5 * math.sin(perp_angle))
        draw.point((lx, ly), fill=color)


def draw_active_ants(draw, cx, cy, frame_idx, ant_color):
    """Draw 2-3 tiny ants emerging from or entering the hole."""
    rng = random.Random(frame_idx * 71 + 31)
    num_ants = rng.randint(2, 4)

    for i in range(num_ants):
        # Ants move radially outward from hole center
        angle = rng.uniform(0, math.pi * 2)
        # Distance from center cycles: ants crawl in and out
        phase = (frame_idx / 8.0) * math.pi * 2
        base_dist = 6 + i * 4
        dist = base_dist + math.sin(phase + i * 1.5) * 5

        ax = cx + int(dist * math.cos(angle))
        ay = cy + int(dist * math.sin(angle))

        if 3 < dist < 28:
            draw_tiny_ant(draw, ax, ay, angle, ant_color)


def render_nest_row(img, row, palette, ant_color, active=False):
    """Render 8 frames of a nest entrance.

    palette: (outer, mid, inner, highlight, hole_outer, hole_inner,
              dirt_dark, dirt_light, dirt_particle)
    """
    outer, mid, inner, highlight, hole_outer, hole_inner, \
        dirt_dark, dirt_light, dirt_particle = palette

    mound_pal = (outer, mid, inner, highlight, hole_outer, hole_inner)

    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2, FRAME // 2

        # Ground-level dirt scatter (behind mound)
        draw_dirt_particles(draw, cx, cy, col, dirt_dark, dirt_light, dirt_particle)

        # The mound itself
        draw_mound(draw, cx, cy, mound_pal, col)

        # Active row: tiny ants at the entrance
        if active:
            draw_active_ants(draw, cx, cy, col, ant_color)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def main():
    img = Image.new("RGBA", (WIDTH, HEIGHT), TRANSPARENT)

    player_palette = (
        P_MOUND_OUTER, P_MOUND_MID, P_MOUND_INNER, P_MOUND_HIGHLIGHT,
        P_HOLE_OUTER, P_HOLE_INNER, P_DIRT_DARK, P_DIRT_LIGHT, P_DIRT_PARTICLE,
    )
    enemy_palette = (
        E_MOUND_OUTER, E_MOUND_MID, E_MOUND_INNER, E_MOUND_HIGHLIGHT,
        E_HOLE_OUTER, E_HOLE_INNER, E_DIRT_DARK, E_DIRT_LIGHT, E_DIRT_PARTICLE,
    )

    render_nest_row(img, row=0, palette=player_palette, ant_color=P_ANT_SILHOUETTE, active=False)
    render_nest_row(img, row=1, palette=player_palette, ant_color=P_ANT_SILHOUETTE, active=True)
    render_nest_row(img, row=2, palette=enemy_palette, ant_color=E_ANT_SILHOUETTE, active=False)
    render_nest_row(img, row=3, palette=enemy_palette, ant_color=E_ANT_SILHOUETTE, active=True)

    out_path = "assets/nest_spritesheet.png"
    import os
    os.makedirs("assets", exist_ok=True)
    img.save(out_path)
    print(f"Nest spritesheet saved to {out_path}  ({WIDTH}×{HEIGHT}, {COLS}×{ROWS} frames of {FRAME}×{FRAME})")


if __name__ == "__main__":
    main()
