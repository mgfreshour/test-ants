#!/usr/bin/env python3
"""Generate a pixel-art spider spritesheet for the colony simulation.

Layout (48×48 per frame, 8 columns):
  Row 0: Idle / walk cycle (8 frames)
  Row 1: Attack / lunge (8 frames)
  Row 2: Death curl (8 frames)

Spider faces RIGHT (+X). The game can rotate sprites to match movement.
"""

import math
from PIL import Image, ImageDraw

FRAME = 48
COLS = 8
ROWS = 3
WIDTH = FRAME * COLS
HEIGHT = FRAME * ROWS

# Palette – RGBA
TRANSPARENT = (0, 0, 0, 0)

# Body colors
BODY_DARK = (50, 35, 25, 255)
BODY_MID = (75, 50, 35, 255)
BODY_LIGHT = (95, 65, 45, 255)
ABDOMEN_DARK = (60, 40, 28, 255)
ABDOMEN_MID = (85, 58, 38, 255)
ABDOMEN_MARK = (110, 75, 50, 255)  # hourglass / chevron marking

# Accents
LEG_DARK = (45, 30, 20, 255)
LEG_MID = (65, 45, 30, 255)
LEG_JOINT = (80, 55, 38, 255)
EYE_COLOR = (180, 30, 30, 255)  # red spider eyes
EYE_GLOW = (220, 60, 50, 255)
FANG_COLOR = (90, 60, 40, 255)
FANG_TIP = (140, 100, 70, 255)

# Attack flash
ATTACK_FLASH = (200, 60, 30, 255)

# Death
DEATH_DARK = (40, 30, 22, 200)
DEATH_MID = (60, 45, 32, 180)


def filled_ellipse(draw, cx, cy, rx, ry, fill):
    """Draw a filled ellipse centered at (cx, cy) with radii (rx, ry)."""
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)


def draw_spider_leg(draw, base_x, base_y, angle, length, frame_idx, side, leg_idx, scale=1.0):
    """Draw a single spider leg with two segments (femur + tibia) and animated joint.

    side: -1 for top, +1 for bottom
    leg_idx: 0-3 from front to back
    """
    s = scale
    # Animate leg phase — alternating gait like a real spider
    phase = (frame_idx / 8.0) * math.pi * 2
    # Legs alternate: even legs move together, odd legs together
    offset = 0.3 * math.sin(phase + (leg_idx * math.pi))

    # First segment (femur) — extends outward
    seg1_len = length * 0.5 * s
    seg1_angle = angle + offset * side
    mid_x = base_x + seg1_len * math.cos(seg1_angle)
    mid_y = base_y + seg1_len * math.sin(seg1_angle)

    # Second segment (tibia) — bends further out/down
    seg2_len = length * 0.6 * s
    seg2_angle = seg1_angle + (0.6 * side)  # bend at joint
    end_x = mid_x + seg2_len * math.cos(seg2_angle)
    end_y = mid_y + seg2_len * math.sin(seg2_angle)

    # Draw segments
    draw.line([(int(base_x), int(base_y)), (int(mid_x), int(mid_y))], fill=LEG_DARK, width=1)
    draw.line([(int(mid_x), int(mid_y)), (int(end_x), int(end_y))], fill=LEG_MID, width=1)
    # Joint dot
    draw.point((int(mid_x), int(mid_y)), fill=LEG_JOINT)


def draw_spider_legs(draw, cx, cy, frame_idx, scale=1.0, curl=0.0):
    """Draw all 8 legs (4 pairs). curl: 0.0=normal, 1.0=fully curled (death)."""
    s = scale
    # 4 pairs of legs, roughly evenly spaced along cephalothorax
    leg_bases = [
        cx + int(4 * s),   # front pair
        cx + int(1 * s),   # second pair
        cx - int(2 * s),   # third pair
        cx - int(5 * s),   # rear pair
    ]

    # Base angles for each pair (spreading outward from body)
    base_angles_top = [-0.8, -1.1, -1.4, -1.7]  # top legs
    base_angles_bot = [0.8, 1.1, 1.4, 1.7]       # bottom legs

    leg_length = 14

    for i, bx in enumerate(leg_bases):
        # Apply curl — legs rotate inward toward body center
        curl_offset_top = curl * (1.5 - base_angles_top[i])
        curl_offset_bot = curl * -(1.5 + base_angles_bot[i])

        # Top leg
        draw_spider_leg(
            draw, bx, cy - int(4 * s),
            base_angles_top[i] + curl_offset_top,
            leg_length, frame_idx, -1, i, scale=s
        )
        # Bottom leg
        draw_spider_leg(
            draw, bx, cy + int(4 * s),
            base_angles_bot[i] + curl_offset_bot,
            leg_length, frame_idx, 1, i, scale=s
        )


def draw_spider_body(draw, cx, cy, scale=1.0, palette=None, abdomen_scale=1.0):
    """Draw spider body: cephalothorax (front) + abdomen (rear).

    Returns (head_cx, cy) for placing eyes/fangs.
    """
    if palette is None:
        palette = (BODY_DARK, BODY_MID, BODY_LIGHT, ABDOMEN_DARK, ABDOMEN_MID, ABDOMEN_MARK)
    bd, bm, bl, ad, am, mark = palette
    s = scale
    a_s = abdomen_scale

    # Abdomen (rear, larger, rounder)
    abd_cx = cx - int(6 * s)
    filled_ellipse(draw, abd_cx, cy, int(8 * s * a_s), int(7 * s * a_s), ad)
    filled_ellipse(draw, abd_cx, cy, int(7 * s * a_s), int(6 * s * a_s), am)

    # Abdomen markings — chevron pattern
    for i in range(3):
        mx = abd_cx - int((3 - i * 2) * s * a_s)
        mr = int((3 - i * 0.5) * s * a_s)
        filled_ellipse(draw, mx, cy, int(1.5 * s), mr, mark)

    # Cephalothorax (front, smaller)
    ceph_cx = cx + int(4 * s)
    filled_ellipse(draw, ceph_cx, cy, int(5 * s), int(5 * s), bd)
    filled_ellipse(draw, ceph_cx, cy, int(4 * s), int(4 * s), bm)

    # Slight waist between sections
    head_cx = ceph_cx + int(3 * s)

    return head_cx, cy, ceph_cx, abd_cx


def draw_spider_eyes(draw, head_cx, cy, scale=1.0, glow=False):
    """Draw cluster of 8 eyes on the spider's head."""
    s = scale
    color = EYE_GLOW if glow else EYE_COLOR

    # Main pair (largest, front-facing)
    x = head_cx + int(2 * s)
    draw.point((x, cy - int(2 * s)), fill=color)
    draw.point((x, cy + int(2 * s)), fill=color)

    # Secondary pair
    x2 = head_cx + int(1 * s)
    draw.point((x2, cy - int(3 * s)), fill=color)
    draw.point((x2, cy + int(3 * s)), fill=color)

    # Small lateral pair
    x3 = head_cx
    draw.point((x3, cy - int(3 * s)), fill=EYE_COLOR)
    draw.point((x3, cy + int(3 * s)), fill=EYE_COLOR)

    # Tiny rear pair
    x4 = head_cx - int(1 * s)
    draw.point((x4, cy - int(2 * s)), fill=EYE_COLOR)
    draw.point((x4, cy + int(2 * s)), fill=EYE_COLOR)


def draw_spider_fangs(draw, head_cx, cy, scale=1.0, open=False):
    """Draw chelicerae / fangs."""
    s = scale
    base_x = head_cx + int(3 * s)
    spread = int(4 * s) if open else int(2 * s)

    # Fang base
    draw.line([
        (base_x, cy - int(1 * s)),
        (base_x + int(2 * s), cy - spread)
    ], fill=FANG_COLOR, width=1)
    draw.line([
        (base_x, cy + int(1 * s)),
        (base_x + int(2 * s), cy + spread)
    ], fill=FANG_COLOR, width=1)

    # Fang tips
    tip_x = base_x + int(2 * s)
    draw.point((tip_x, cy - spread), fill=FANG_TIP)
    draw.point((tip_x, cy + spread), fill=FANG_TIP)


def render_spider_walk(img, row):
    """Render 8 frames of spider idle/walk cycle."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2 - 2, FRAME // 2

        draw_spider_legs(draw, cx, cy, col, scale=1.0)
        head_cx, _, ceph_cx, abd_cx = draw_spider_body(draw, cx, cy)
        draw_spider_eyes(draw, head_cx, cy)
        draw_spider_fangs(draw, head_cx, cy, open=False)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_spider_attack(img, row):
    """Render 8 frames of spider attack/lunge animation.

    Frames 0-2: rear back
    Frames 3-5: lunge forward with fangs open, eyes glow
    Frames 6-7: return to neutral
    """
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)

        # Lunge offset
        if col < 3:
            lunge = -col * 1.5  # rear back
        elif col < 6:
            lunge = (col - 2) * 2.5  # lunge forward
        else:
            lunge = max(0, (8 - col) * 2)  # return

        cx = int(FRAME // 2 - 2 + lunge)
        cy = FRAME // 2

        fangs_open = 3 <= col <= 5
        eyes_glow = 2 <= col <= 6

        # Flash body during strike
        if 3 <= col <= 5:
            pal = (ATTACK_FLASH, BODY_MID, BODY_LIGHT, ABDOMEN_DARK, ABDOMEN_MID, ABDOMEN_MARK)
        else:
            pal = None

        draw_spider_legs(draw, cx, cy, col, scale=1.0)
        head_cx, _, ceph_cx, abd_cx = draw_spider_body(draw, cx, cy, palette=pal)
        draw_spider_eyes(draw, head_cx, cy, glow=eyes_glow)
        draw_spider_fangs(draw, head_cx, cy, open=fangs_open)

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def render_spider_death(img, row):
    """Render 8 frames of spider death animation — legs curl inward, body fades."""
    for col in range(COLS):
        frame = Image.new("RGBA", (FRAME, FRAME), TRANSPARENT)
        draw = ImageDraw.Draw(frame)
        cx, cy = FRAME // 2 - 2, FRAME // 2

        # Progressive curl: 0.0 → 1.0 over 8 frames
        curl = col / 7.0

        # Fade alpha
        alpha_mult = 1.0 - (curl * 0.4)

        def fade(c):
            return (c[0], c[1], c[2], int(c[3] * alpha_mult))

        death_palette = (
            fade(DEATH_DARK), fade(DEATH_MID), fade(BODY_LIGHT),
            fade(DEATH_DARK), fade(DEATH_MID), fade(ABDOMEN_MARK),
        )

        # Spider flips upside-down as it dies (abdomen scale shrinks slightly)
        abd_scale = 1.0 - curl * 0.15

        draw_spider_legs(draw, cx, cy, 0, scale=1.0, curl=curl)
        head_cx, _, ceph_cx, abd_cx = draw_spider_body(
            draw, cx, cy, palette=death_palette, abdomen_scale=abd_scale
        )

        # Eyes dim
        if col < 5:
            draw_spider_eyes(draw, head_cx, cy)
        draw_spider_fangs(draw, head_cx, cy, open=(col < 3))

        img.paste(frame, (col * FRAME, row * FRAME), frame)


def main():
    img = Image.new("RGBA", (WIDTH, HEIGHT), TRANSPARENT)

    render_spider_walk(img, row=0)
    render_spider_attack(img, row=1)
    render_spider_death(img, row=2)

    out_path = "assets/spider_spritesheet.png"
    import os
    os.makedirs("assets", exist_ok=True)
    img.save(out_path)
    print(f"Spider spritesheet saved to {out_path}  ({WIDTH}×{HEIGHT}, {COLS}×{ROWS} frames of {FRAME}×{FRAME})")


if __name__ == "__main__":
    main()
