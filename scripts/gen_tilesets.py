#!/usr/bin/env python3
"""Generate pixel-art tilesets for LDtk integration (Sprint L4).

Creates:
  assets/tilesets/terrain.png  — 16 tiles (256×16), surface terrain
  assets/tilesets/nest.png     — 24 tiles (384×16), underground nest
"""

import random
from PIL import Image

TILE = 16
random.seed(42)  # deterministic output


def rgb(r, g, b):
    """Convert 0.0-1.0 floats to 0-255 ints."""
    return (int(r * 255), int(g * 255), int(b * 255))


def vary(base, amount=10):
    """Return a color slightly varied from base."""
    return tuple(max(0, min(255, c + random.randint(-amount, amount))) for c in base)


def fill_textured(img, x0, y0, base_color, noise=8):
    """Fill a 16×16 tile with a noisy texture around base_color."""
    for dy in range(TILE):
        for dx in range(TILE):
            img.putpixel((x0 + dx, y0 + dy), vary(base_color, noise))


def fill_with_specks(img, x0, y0, base_color, speck_color, density=0.15, noise=6):
    """Fill tile with base color + random specks."""
    for dy in range(TILE):
        for dx in range(TILE):
            if random.random() < density:
                img.putpixel((x0 + dx, y0 + dy), vary(speck_color, noise))
            else:
                img.putpixel((x0 + dx, y0 + dy), vary(base_color, noise))


def fill_grass(img, x0, y0, base_color, blade_color):
    """Grass tile: base with vertical blade-like specks."""
    for dy in range(TILE):
        for dx in range(TILE):
            # Grass blades: darker vertical streaks
            if random.random() < 0.12 and dy > 2:
                img.putpixel((x0 + dx, y0 + dy), vary(blade_color, 8))
            elif random.random() < 0.05:
                # Tiny flowers / lighter spots
                img.putpixel((x0 + dx, y0 + dy), vary((min(base_color[0]+30, 255), base_color[1]+15, base_color[2]), 5))
            else:
                img.putpixel((x0 + dx, y0 + dy), vary(base_color, 6))


def fill_rock(img, x0, y0, base_color, crack_color, noise=6):
    """Rock tile with crack lines."""
    fill_textured(img, x0, y0, base_color, noise)
    # Add 1-2 crack lines
    for _ in range(random.randint(1, 2)):
        cx, cy = random.randint(2, 13), random.randint(2, 13)
        for step in range(random.randint(3, 7)):
            if 0 <= cx < TILE and 0 <= cy < TILE:
                img.putpixel((x0 + cx, y0 + cy), vary(crack_color, 4))
            cx += random.choice([-1, 0, 1])
            cy += random.choice([0, 1])


def fill_tunnel(img, x0, y0):
    """Tunnel tile: darker edges (ceiling/floor), lighter center."""
    dark = rgb(0.28, 0.28, 0.28)
    mid = rgb(0.35, 0.35, 0.35)
    light = rgb(0.40, 0.40, 0.40)
    for dy in range(TILE):
        for dx in range(TILE):
            # Edges are darker
            edge_dist = min(dy, TILE - 1 - dy, dx, TILE - 1 - dx)
            if edge_dist <= 1:
                img.putpixel((x0 + dx, y0 + dy), vary(dark, 4))
            elif edge_dist <= 3:
                img.putpixel((x0 + dx, y0 + dy), vary(mid, 5))
            else:
                img.putpixel((x0 + dx, y0 + dy), vary(light, 5))


def fill_chamber(img, x0, y0, base_color, accent_color):
    """Chamber tile: base with subtle border accent."""
    for dy in range(TILE):
        for dx in range(TILE):
            edge_dist = min(dy, TILE - 1 - dy, dx, TILE - 1 - dx)
            if edge_dist <= 1:
                img.putpixel((x0 + dx, y0 + dy), vary(accent_color, 4))
            else:
                img.putpixel((x0 + dx, y0 + dy), vary(base_color, 5))


# ─── TERRAIN TILESET (surface) ───────────────────────────────────────
# 16 tiles in a row: 256×16

terrain = Image.new("RGBA", (TILE * 16, TILE), (0, 0, 0, 0))

# 0: grass_dark
fill_grass(terrain, 0 * TILE, 0, rgb(0.22, 0.45, 0.15), rgb(0.15, 0.32, 0.10))
# 1: grass_light
fill_grass(terrain, 1 * TILE, 0, rgb(0.28, 0.52, 0.18), rgb(0.20, 0.40, 0.12))
# 2: dirt
fill_with_specks(terrain, 2 * TILE, 0, rgb(0.42, 0.27, 0.14), rgb(0.35, 0.22, 0.10), 0.2)
# 3: sand
fill_textured(terrain, 3 * TILE, 0, rgb(0.76, 0.70, 0.50), 8)
# 4: concrete
fill_textured(terrain, 4 * TILE, 0, rgb(0.60, 0.60, 0.60), 5)
# 5: nest_mound
fill_with_specks(terrain, 5 * TILE, 0, rgb(0.35, 0.25, 0.15), rgb(0.28, 0.20, 0.12), 0.25)
# 6: nest_hole
fill_textured(terrain, 6 * TILE, 0, rgb(0.08, 0.05, 0.02), 3)
# 7: water/puddle
fill_textured(terrain, 7 * TILE, 0, rgb(0.25, 0.40, 0.65), 6)
# 8: grass variant A
fill_grass(terrain, 8 * TILE, 0, rgb(0.24, 0.47, 0.14), rgb(0.16, 0.34, 0.09))
# 9: grass variant B
fill_grass(terrain, 9 * TILE, 0, rgb(0.26, 0.50, 0.16), rgb(0.18, 0.38, 0.11))
# 10: grass variant C (yellower)
fill_grass(terrain, 10 * TILE, 0, rgb(0.30, 0.48, 0.15), rgb(0.22, 0.36, 0.10))
# 11: dirt-grass transition
fill_with_specks(terrain, 11 * TILE, 0, rgb(0.32, 0.38, 0.15), rgb(0.40, 0.28, 0.14), 0.35)
# 12: stones/pebbles
fill_with_specks(terrain, 12 * TILE, 0, rgb(0.50, 0.48, 0.44), rgb(0.60, 0.58, 0.55), 0.3, 6)
# 13: concrete cracked
fill_rock(terrain, 13 * TILE, 0, rgb(0.58, 0.58, 0.58), rgb(0.42, 0.42, 0.42), 4)
# 14: sand-dirt transition
fill_with_specks(terrain, 14 * TILE, 0, rgb(0.60, 0.50, 0.32), rgb(0.50, 0.38, 0.22), 0.3)
# 15: dead leaves
fill_with_specks(terrain, 15 * TILE, 0, rgb(0.40, 0.30, 0.15), rgb(0.55, 0.38, 0.18), 0.4, 8)

terrain.save("assets/tilesets/terrain.png")
print(f"terrain.png: {terrain.size[0]}×{terrain.size[1]} ({terrain.size[0]//TILE} tiles)")


# ─── NEST TILESET (underground) ──────────────────────────────────────
# 24 tiles in a row: 384×16

nest = Image.new("RGBA", (TILE * 24, TILE), (0, 0, 0, 0))

# 0: soil
fill_with_specks(nest, 0 * TILE, 0, rgb(0.45, 0.32, 0.18), rgb(0.38, 0.26, 0.14), 0.15)
# 1: soft soil
fill_with_specks(nest, 1 * TILE, 0, rgb(0.50, 0.36, 0.20), rgb(0.44, 0.30, 0.16), 0.12)
# 2: clay
fill_textured(nest, 2 * TILE, 0, rgb(0.55, 0.40, 0.25), 6)
# 3: rock
fill_rock(nest, 3 * TILE, 0, rgb(0.40, 0.40, 0.40), rgb(0.30, 0.30, 0.30))
# 4: tunnel (base)
fill_tunnel(nest, 4 * TILE, 0)
# 5: chamber queen (purple-brown tint)
fill_chamber(nest, 5 * TILE, 0, rgb(0.25, 0.12, 0.18), rgb(0.32, 0.15, 0.22))
# 6: chamber brood (warm brown)
fill_chamber(nest, 6 * TILE, 0, rgb(0.22, 0.15, 0.10), rgb(0.28, 0.18, 0.13))
# 7: chamber food (gold-brown)
fill_chamber(nest, 7 * TILE, 0, rgb(0.20, 0.18, 0.08), rgb(0.26, 0.22, 0.10))
# 8: chamber midden (grey-brown)
fill_chamber(nest, 8 * TILE, 0, rgb(0.18, 0.16, 0.12), rgb(0.22, 0.20, 0.15))
# 9: tunnel horizontal (dark top/bottom edges)
for dy in range(TILE):
    for dx in range(TILE):
        if dy <= 2 or dy >= 13:
            nest.putpixel((9*TILE+dx, dy), vary(rgb(0.25, 0.22, 0.16), 4))
        else:
            nest.putpixel((9*TILE+dx, dy), vary(rgb(0.38, 0.38, 0.38), 5))
# 10: tunnel vertical (dark left/right edges)
for dy in range(TILE):
    for dx in range(TILE):
        if dx <= 2 or dx >= 13:
            nest.putpixel((10*TILE+dx, dy), vary(rgb(0.25, 0.22, 0.16), 4))
        else:
            nest.putpixel((10*TILE+dx, dy), vary(rgb(0.38, 0.38, 0.38), 5))
# 11: tunnel cross (all four open)
for dy in range(TILE):
    for dx in range(TILE):
        in_h = 3 <= dy <= 12
        in_v = 3 <= dx <= 12
        if in_h or in_v:
            nest.putpixel((11*TILE+dx, dy), vary(rgb(0.38, 0.38, 0.38), 5))
        else:
            nest.putpixel((11*TILE+dx, dy), vary(rgb(0.25, 0.22, 0.16), 4))
# 12: tunnel T-junction (open top/left/right)
for dy in range(TILE):
    for dx in range(TILE):
        in_h = 3 <= dy <= 12
        in_v = 3 <= dx <= 12 and dy <= 12
        if in_h or in_v:
            nest.putpixel((12*TILE+dx, dy), vary(rgb(0.38, 0.38, 0.38), 5))
        else:
            nest.putpixel((12*TILE+dx, dy), vary(rgb(0.25, 0.22, 0.16), 4))
# 13: rock variant A (darker)
fill_rock(nest, 13 * TILE, 0, rgb(0.35, 0.35, 0.35), rgb(0.25, 0.25, 0.25))
# 14: rock variant B (lighter)
fill_rock(nest, 14 * TILE, 0, rgb(0.45, 0.45, 0.45), rgb(0.35, 0.35, 0.35))
# 15: rock variant C (brownish)
fill_rock(nest, 15 * TILE, 0, rgb(0.42, 0.38, 0.35), rgb(0.32, 0.28, 0.25))
# 16: soil variant A
fill_with_specks(nest, 16 * TILE, 0, rgb(0.43, 0.30, 0.16), rgb(0.36, 0.24, 0.12), 0.18)
# 17: soil variant B (with root-like specks)
fill_with_specks(nest, 17 * TILE, 0, rgb(0.46, 0.33, 0.19), rgb(0.30, 0.20, 0.10), 0.10, 8)
# 18: clay variant (redder)
fill_textured(nest, 18 * TILE, 0, rgb(0.58, 0.38, 0.22), 7)
# 19: soft soil variant
fill_with_specks(nest, 19 * TILE, 0, rgb(0.52, 0.38, 0.22), rgb(0.46, 0.32, 0.18), 0.10)
# 20-23: reserved (transparent)
for i in range(20, 24):
    for dy in range(TILE):
        for dx in range(TILE):
            nest.putpixel((i*TILE+dx, dy), (0, 0, 0, 0))

nest.save("assets/tilesets/nest.png")
print(f"nest.png: {nest.size[0]}×{nest.size[1]} ({nest.size[0]//TILE} tiles)")

print("Done!")
