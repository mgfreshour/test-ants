#!/usr/bin/env python3
"""Generate a pixel-art ant spritesheet for the colony simulation.

Layout (32×32 per frame, 8 columns):
  Row 0: Worker walk cycle (8 frames)
  Row 1: Worker carrying food walk cycle (8 frames)
  Row 2: Soldier walk cycle (8 frames)
  Row 3: Soldier fight/attack (8 frames)
  Row 4: Queen idle/walk (8 frames)

All ants face RIGHT (east). The game rotates sprites to match movement direction.
"""

from PIL import Image, ImageDraw

FRAME = 32
COLS = 8
ROWS = 5
WIDTH = FRAME * COLS
HEIGHT = FRAME * ROWS

# Palette – RGBA
TRANSPARENT = (0, 0, 0, 0)
# Worker body
BODY_DARK = (40, 28, 20, 255)
BODY_MID = (65, 45, 30, 255)
BODY_LIGHT = (90, 65, 45, 255)
# Soldier body (slightly redder)
SOL_DARK = (55, 25, 18, 255)
SOL_MID = (85, 40, 25, 255)
SOL_LIGHT = (110, 60, 35, 255)
# Queen body (darker, larger)
QUEEN_DARK = (30, 22, 16, 255)
QUEEN_MID = (55, 40, 28, 255)
QUEEN_LIGHT = (80, 58, 40, 255)
# Accents
LEG_COLOR = (50, 35, 25, 255)
ANTENNA_COLOR = (60, 42, 30, 255)
EYE_COLOR = (200, 180, 140, 255)
MANDIBLE_COLOR = (80, 55, 35, 255)
FOOD_COLOR = (180, 210, 60, 255)
FOOD_DARK = (140, 170, 40, 255)
# Fight flash
FIGHT_FLASH = (220, 80, 40, 255)


def filled_ellipse(draw, cx, cy, rx, ry, fill):
    """Draw a filled ellipse centered at (cx, cy) with radii (rx, ry)."""
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)


def draw_ant_body(draw, cx, cy, scale=1.0, palette=None):
    """Draw basic ant body (head, thorax, abdomen) facing right.
    
    Returns (head_cx, head_cy) for placing antennae/mandibles.
    """
    if palette is None:
        palette = (BODY_DARK, BODY_MID, BODY_LIGHT)
    dark, mid, light = palette

    s = scale

    # Abdomen (rear, largest) – slightly left of center
    abd_cx = cx - int(4 * s)
    filled_ellipse(draw, abd_cx, cy, int(6 * s), int(5 * s), dark)
    filled_ellipse(draw, abd_cx, cy, int(5 * s), int(4 * s), mid)
    # Abdomen stripe
    filled_ellipse(draw, abd_cx - int(2 * s), cy, int(2 * s), int(3 * s), light)

    # Thorax (middle, smaller)
    thor_cx = cx + int(3 * s)
    filled_ellipse(draw, thor_cx, cy, int(3 * s), int(3 * s), mid)
    filled_ellipse(draw, thor_cx, cy, int(2 * s), int(2 * s), light)

    # Head (front)
    head_cx = cx + int(8 * s)
    filled_ellipse(draw, head_cx, cy, int(3 * s), int(3 * s), dark)
    filled_ellipse(draw, head_cx, cy, int(2 * s), int(2 * s), mid)

    # Eyes
    draw.point((head_cx + int(1 * s), cy - int(2 * s)), fill=EYE_COLOR)
    draw.point((head_cx + int(1 * s), cy + int(2 * s)), fill=EYE_COLOR)

    return head_cx, cy, thor_cx, abd_cx


def draw_legs(draw, thor_cx, cy, frame_idx, scale=1.0):
    """Draw 3 pairs of legs with walk animation.
    
    frame_idx 0..7 controls leg phase.
    """
    s = scale
    leg_len = int(5 * s)
    offsets = [-int(2 * s), 0, int(2 * s)]

    # Alternating gait: legs on opposite sides move in anti-phase
    phase = (frame_idx / 8.0) * 3.14159 * 2  # full cycle over 8 frames
    import math

    for i, ox in enumerate(offsets):
        base_x = thor_cx + ox - int(1 * s)
        # Tripod gait: legs 0,2 on one side, leg 1 on the other alternate
        if i % 2 == 0:
            angle_top = -1.2 + 0.4 * math.sin(phase)
            angle_bot = 1.2 - 0.4 * math.sin(phase)
        else:
            angle_top = -1.2 - 0.4 * math.sin(phase)
            angle_bot = 1.2 + 0.4 * math.sin(phase)

        # Top leg
        end_x = base_x + int(leg_len * math.cos(angle_top))
        end_y = cy - int(3 * s) + int(leg_len * math.sin(angle_top))
        draw.line([(base_x, cy - int(3 * s)), (end_x, end_y)], fill=LEG_COLOR, width=1)

        # Bottom leg
        end_x = base_x + int(leg_len * math.cos(angle_bot))
        end_y = cy + int(3 * s) + int(leg_len * math.sin(angle_bot))
        draw.line([(base_x, cy + int(3 * s)), (end_x, end_y)], fill=LEG_COLOR, width=1)


def draw_antennae(draw, head_cx, cy, frame_idx, scale=1.0):
    """Draw two antennae with slight animation."""
    import math
    s = scale
    phase = (frame_idx / 8.0) * 3.14159 * 2
    wobble = 0.3 * math.sin(phase)

    ant_len = int(5 * s)
    base_x = head_cx + int(2 * s)

    # Top antenna
    mid_x = base_x + int(3 * s)
    mid_y = cy - int(3 * s) + int(wobble * s)
    end_x = mid_x + int(2 * s)
    end_y = mid_y - int(2 * s)
    draw.line([(base_x, cy - int(1 * s)), (mid_x, mid_y), (end_x, end_y)], fill=ANTENNA_COLOR, width=1)

    # Bottom antenna
    mid_y2 = cy + int(3 * s) - int(wobble * s)
    end_y2 = mid_y2 + int(2 * s)
    draw.line([(base_x, cy + int(1 * s)), (mid_x, mid_y2), (end_x, end_y2)], fill=ANTENNA_COLOR, width=1)


def draw_mandibles(draw, head_cx, cy, scale=1.0, open=False):
    """Draw mandibles (visible on soldiers and during fighting)."""
    s = scale
    base_x = head_cx + int(3 * s)
    spread = int(3 * s) if open else int(1 * s)

    draw.line([(base_x, cy - int(1 * s)), (base_x + int(3 * s), cy - spread)],
              fill=MANDIBLE_COLOR, width=1)
    draw.line([(base_x, cy + int(1 * s)), (base_x + int(3 * s), cy + spread)],
              fill=MANDIBLE_COLOR, width=1)


def draw_food_morsel(draw, cx, cy, scale=1.0):
    """Draw a small food chunk being carried above the ant."""
    s = scale
    filled_ellipse(draw, cx + int(5 * s), cy - int(2 * s), int(3 * s), int(2 * s), FOOD_COLOR)
    filled_ellipse(draw, cx + int(5 * s), cy - int(2 * s), int(2 * s), int(1 * s), FOOD_DARK)


def render_worker_walk(img, row, carrying=False):
    """Render 8 frames of worker walk cycle."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2 - 2, FRAME // 2

        draw_legs(draw, cx + 3, cy, col, scale=1.0)
        head_cx, head_cy, thor_cx, abd_cx = draw_ant_body(draw, cx, cy)
        draw_antennae(draw, head_cx, cy, col)

        if carrying:
            draw_food_morsel(draw, cx, cy)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_soldier_walk(img, row):
    """Render 8 frames of soldier walk cycle (bigger, with mandibles)."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2 - 3, FRAME // 2

        draw_legs(draw, cx + 3, cy, col, scale=1.15)
        head_cx, head_cy, thor_cx, abd_cx = draw_ant_body(
            draw, cx, cy, scale=1.15, palette=(SOL_DARK, SOL_MID, SOL_LIGHT)
        )
        draw_mandibles(draw, head_cx, cy, scale=1.15, open=False)
        draw_antennae(draw, head_cx, cy, col, scale=1.15)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_soldier_fight(img, row):
    """Render 8 frames of soldier fight animation (mandibles open/close, flash)."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2 - 3, FRAME // 2

        draw_legs(draw, cx + 3, cy, col, scale=1.15)

        # Flash on even frames
        if col % 2 == 0:
            pal = (SOL_DARK, SOL_MID, SOL_LIGHT)
        else:
            pal = (FIGHT_FLASH, SOL_MID, SOL_LIGHT)

        head_cx, head_cy, thor_cx, abd_cx = draw_ant_body(
            draw, cx, cy, scale=1.15, palette=pal
        )
        mandible_open = col % 4 < 2
        draw_mandibles(draw, head_cx, cy, scale=1.15, open=mandible_open)
        draw_antennae(draw, head_cx, cy, col, scale=1.15)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_queen(img, row):
    """Render 8 frames of queen idle/sway (larger abdomen)."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2 - 4, FRAME // 2

        draw_legs(draw, cx + 3, cy, col, scale=1.3)

        dark, mid, light = QUEEN_DARK, QUEEN_MID, QUEEN_LIGHT
        s = 1.3

        # Queen has an extra-large abdomen
        abd_cx = cx - int(5 * s)
        filled_ellipse(draw, abd_cx, cy, int(8 * s), int(6 * s), dark)
        filled_ellipse(draw, abd_cx, cy, int(7 * s), int(5 * s), mid)
        filled_ellipse(draw, abd_cx - int(3 * s), cy, int(3 * s), int(4 * s), light)

        # Thorax
        thor_cx = cx + int(4 * s)
        filled_ellipse(draw, thor_cx, cy, int(3 * s), int(3 * s), mid)
        filled_ellipse(draw, thor_cx, cy, int(2 * s), int(2 * s), light)

        # Head
        head_cx = cx + int(9 * s)
        filled_ellipse(draw, head_cx, cy, int(3 * s), int(3 * s), dark)
        filled_ellipse(draw, head_cx, cy, int(2 * s), int(2 * s), mid)
        draw.point((head_cx + int(1 * s), cy - int(2 * s)), fill=EYE_COLOR)
        draw.point((head_cx + int(1 * s), cy + int(2 * s)), fill=EYE_COLOR)

        draw_antennae(draw, head_cx, cy, col, scale=1.3)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def main():
    img = Image.new("RGBA", (WIDTH, HEIGHT), TRANSPARENT)

    render_worker_walk(img, row=0, carrying=False)
    render_worker_walk(img, row=1, carrying=True)
    render_soldier_walk(img, row=2)
    render_soldier_fight(img, row=3)
    render_queen(img, row=4)

    out_path = "assets/ant_spritesheet.png"
    import os
    os.makedirs("assets", exist_ok=True)
    img.save(out_path)
    print(f"Spritesheet saved to {out_path}  ({WIDTH}×{HEIGHT}, {COLS}×{ROWS} frames of {FRAME}×{FRAME})")


if __name__ == "__main__":
    main()
