#!/usr/bin/env python3
"""Update colony.ldtk with new tilesets and auto-rules for Sprint L4.

Changes:
  1. Update terrain tileset dimensions (112→256 px wide)
  2. Add nest tileset definition
  3. Add auto-rules to Terrain layer (IntGrid value → tileset tile)
  4. Add auto-rules to NestCells layer
  5. Generate autoLayerTiles for all levels based on rules
"""

import json
import random
import uuid

random.seed(42)

LDTK_PATH = "assets/maps/colony.ldtk"

with open(LDTK_PATH) as f:
    data = json.load(f)

next_uid = data["nextUid"]

def alloc_uid():
    global next_uid
    uid = next_uid
    next_uid += 1
    return uid

# ─── 1. Update terrain tileset dimensions ────────────────────────────

for ts in data["defs"]["tilesets"]:
    if ts["identifier"] == "Terrain_tileset":
        ts["pxWid"] = 256
        ts["__cWid"] = 16
        # Recalculate cached tile data if present
        if "cachedPixelData" in ts:
            ts["cachedPixelData"] = None
        if "savedSelections" in ts:
            ts["savedSelections"] = []
        print(f"Updated terrain tileset: {ts['pxWid']}x{ts['pxHei']}, {ts['__cWid']} cols")

# ─── 2. Add nest tileset ─────────────────────────────────────────────

NEST_TILESET_UID = alloc_uid()
nest_tileset = {
    "__cHei": 1,
    "__cWid": 24,
    "cachedPixelData": None,
    "customData": [],
    "embedAtlas": None,
    "enumTags": [],
    "identifier": "Nest_tileset",
    "padding": 0,
    "pxHei": 16,
    "pxWid": 384,
    "relPath": "../tilesets/nest.png",
    "savedSelections": [],
    "spacing": 0,
    "tags": [],
    "tagsSourceEnumUid": None,
    "tileGridSize": 16,
    "uid": NEST_TILESET_UID
}
data["defs"]["tilesets"].append(nest_tileset)
print(f"Added nest tileset uid={NEST_TILESET_UID}")

# ─── 3. Auto-rule helpers ────────────────────────────────────────────

def make_rule(intgrid_value, tile_ids, chance=1.0, size=1, pattern=None):
    """Create an LDtk auto-rule.

    For size=1: pattern is just [intgrid_value]
    For size=3: pattern is 9 values (row-major, center = index 4)
    """
    if pattern is None:
        pattern = [intgrid_value]

    # tile_ids can be a list (for random selection) or single int
    if isinstance(tile_ids, int):
        tile_ids = [tile_ids]

    return {
        "uid": alloc_uid(),
        "active": True,
        "alpha": 1.0,
        "breakOnMatch": True,
        "chance": chance,
        "checker": "None",
        "flipX": False,
        "flipY": False,
        "invalidated": False,
        "outOfBoundsValue": None,
        "pattern": pattern,
        "perlinActive": False,
        "perlinOctaves": 2,
        "perlinScale": 0.2,
        "perlinSeed": 1234,
        "pivotX": 0.0,
        "pivotY": 0.0,
        "size": size,
        "tileIds": tile_ids,
        "tileMode": "Single" if len(tile_ids) == 1 else "Stamp",
        "tileRectsIds": [[t] for t in tile_ids],
        "tileRandomXMax": 0,
        "tileRandomXMin": 0,
        "tileRandomYMax": 0,
        "tileRandomYMin": 0,
        "tileXOffset": 0,
        "tileYOffset": 0,
        "xModulo": 1,
        "xOffset": 0,
        "yModulo": 1,
        "yOffset": 0,
    }

def make_rule_group(name, rules):
    return {
        "uid": alloc_uid(),
        "name": name,
        "active": True,
        "biomeRequirementMode": 0,
        "collapsed": False,
        "color": None,
        "icon": None,
        "isOptional": False,
        "requiredBiomeValues": [],
        "rules": rules,
        "usesWizard": False,
    }

# ─── 4. Add auto-rules to Terrain layer ──────────────────────────────

# Terrain IntGrid values: 1=grass_dark, 2=grass_light, 3=dirt, 4=sand,
#   5=concrete, 6=nest_mound, 7=nest_hole
# Terrain tileset indices: 0=grass_dark, 1=grass_light, 2=dirt, 3=sand,
#   4=concrete, 5=nest_mound, 6=nest_hole, 7=water, 8-10=grass_variants,
#   11=dirt-grass, 12=stones, 13=concrete_cracked, 14=sand-dirt, 15=dead_leaves

TERRAIN_TILESET_UID = 1  # existing uid

terrain_rules = []

# Grass (value 1): randomly pick from all grass variant tiles
# Value 2 (grass_light) will be normalized to 1 in IntGrid data below.
terrain_rules.append(make_rule_group("Grass variants", [
    make_rule(1, [0], chance=0.25),   # grass dark
    make_rule(1, [1], chance=0.30),   # grass light
    make_rule(1, [8], chance=0.30),   # variant A
    make_rule(1, [9], chance=0.35),   # variant B
    make_rule(1, [10], chance=1.0),   # variant C (fallback)
]))

# Direct 1:1 mappings for other values
terrain_rules.append(make_rule_group("Dirt", [make_rule(3, [2])]))
terrain_rules.append(make_rule_group("Sand", [make_rule(4, [3])]))
terrain_rules.append(make_rule_group("Concrete", [
    make_rule(5, [13], chance=0.15),  # cracked variant
    make_rule(5, [4], chance=1.0),
]))
terrain_rules.append(make_rule_group("Nest mound", [make_rule(6, [5])]))
terrain_rules.append(make_rule_group("Nest hole", [make_rule(7, [6])]))

for layer in data["defs"]["layers"]:
    if layer["identifier"] == "Terrain":
        layer["autoTilesetDefUid"] = TERRAIN_TILESET_UID
        layer["autoRuleGroups"] = terrain_rules
        print(f"Added {len(terrain_rules)} auto-rule groups to Terrain layer")

# ─── 5. Add auto-rules to NestCells layer ────────────────────────────

# NestCells IntGrid values: 1=Soil, 2=SoftSoil, 3=Clay, 4=Rock,
#   5=Tunnel, 6=ChamberQueen, 7=ChamberBrood, 8=ChamberFood, 9=ChamberMidden
# Nest tileset indices: 0=soil, 1=soft_soil, 2=clay, 3=rock, 4=tunnel,
#   5=chamber_queen, 6=chamber_brood, 7=chamber_food, 8=chamber_midden,
#   9=tunnel_h, 10=tunnel_v, 11=tunnel_cross, 12=tunnel_T,
#   13-15=rock_variants, 16-17=soil_variants, 18=clay_variant, 19=soft_soil_variant

nest_rules = []

# Soil with variants
nest_rules.append(make_rule_group("Soil variants", [
    make_rule(1, [16], chance=0.2),
    make_rule(1, [17], chance=0.15),
    make_rule(1, [0], chance=1.0),
]))

# Soft soil with variant
nest_rules.append(make_rule_group("SoftSoil variants", [
    make_rule(2, [19], chance=0.2),
    make_rule(2, [1], chance=1.0),
]))

# Clay with variant
nest_rules.append(make_rule_group("Clay variants", [
    make_rule(3, [18], chance=0.15),
    make_rule(3, [2], chance=1.0),
]))

# Rock with variants
nest_rules.append(make_rule_group("Rock variants", [
    make_rule(4, [13], chance=0.25),
    make_rule(4, [14], chance=0.25),
    make_rule(4, [15], chance=0.2),
    make_rule(4, [3], chance=1.0),
]))

# Tunnel: use directional tiles based on neighbors
# 3×3 pattern: 0=anything, positive=must match, negative=must NOT match
# Pattern indices: [0,1,2,3,4,5,6,7,8] = 3×3 grid row-major, center=4

# Tunnel horizontal: tunnel left+right, not tunnel above+below
nest_rules.append(make_rule_group("Tunnel directional", [
    # Horizontal tunnel (neighbors left+right are tunnel/chamber, top+bottom are NOT)
    make_rule(5, [9], size=3, pattern=[
        0, -5, 0,
        5,  5, 5,
        0, -5, 0,
    ], chance=1.0),
    # Vertical tunnel
    make_rule(5, [10], size=3, pattern=[
        0,  5, 0,
       -5,  5,-5,
        0,  5, 0,
    ], chance=1.0),
    # Cross (all 4 neighbors are tunnel/chamber)
    make_rule(5, [11], size=3, pattern=[
        0, 5, 0,
        5, 5, 5,
        0, 5, 0,
    ], chance=1.0),
    # T-junction top (left+right+top)
    make_rule(5, [12], size=3, pattern=[
        0,  5, 0,
        5,  5, 5,
        0, -5, 0,
    ], chance=1.0),
    # Fallback: base tunnel tile
    make_rule(5, [4], chance=1.0),
]))

# Chambers: direct mapping
nest_rules.append(make_rule_group("Chambers", [
    make_rule(6, [5]),
    make_rule(7, [6]),
    make_rule(8, [7]),
    make_rule(9, [8]),
]))

for layer in data["defs"]["layers"]:
    if layer["identifier"] == "NestCells":
        layer["autoTilesetDefUid"] = NEST_TILESET_UID
        layer["autoRuleGroups"] = nest_rules
        print(f"Added {len(nest_rules)} auto-rule groups to NestCells layer")


# ─── 5b. Normalize surface IntGrid: merge grass_dark(1) + grass_light(2) → 1
# This eliminates the checkerboard pattern; auto-rules handle variation.

for level in data["levels"]:
    if level["identifier"] == "Surface":
        for li in level["layerInstances"]:
            if li["__identifier"] == "Terrain":
                li["intGridCsv"] = [
                    1 if v == 2 else v for v in li["intGridCsv"]
                ]
                print(f"Normalized Surface IntGrid: merged grass_light(2) → grass(1)")

# ─── 6. Generate autoLayerTiles for all levels ───────────────────────

def apply_rules_to_intgrid(intgrid_csv, grid_w, grid_h, rule_groups, tileset_uid):
    """Apply auto-rules to an IntGrid CSV and produce autoLayerTiles."""
    tiles = []

    def get_cell(cx, cy):
        if cx < 0 or cx >= grid_w or cy < 0 or cy >= grid_h:
            return 0
        return intgrid_csv[cy * grid_w + cx]

    def match_pattern(cx, cy, rule):
        """Check if a rule pattern matches at position (cx, cy)."""
        size = rule["size"]
        pattern = rule["pattern"]
        half = size // 2

        for py in range(size):
            for px in range(size):
                pi = py * size + px
                expected = pattern[pi]

                if expected == 0:
                    continue  # wildcard

                ncx = cx + px - half
                ncy = cy + py - half
                actual = get_cell(ncx, ncy)

                if expected > 0:
                    # Must match this value
                    if actual != expected:
                        return False
                elif expected < 0:
                    # Must NOT be this value (also treat chambers as tunnel-like for neighbor matching)
                    forbidden = -expected
                    if forbidden == 5:
                        # "not tunnel" means not tunnel AND not any chamber
                        if actual in (5, 6, 7, 8, 9):
                            return False
                    else:
                        if actual == forbidden:
                            return False
                elif expected == 1000001:
                    # Must be non-empty
                    if actual == 0:
                        return False

        return True

    for cy in range(grid_h):
        for cx in range(grid_w):
            cell_val = intgrid_csv[cy * grid_w + cx]
            if cell_val == 0:
                continue

            matched = False
            for group in rule_groups:
                if not group["active"]:
                    continue
                for rule in group["rules"]:
                    if not rule["active"]:
                        continue

                    # Check if this rule's pattern targets this cell value
                    size = rule["size"]
                    center_idx = (size * size) // 2
                    pattern_center = rule["pattern"][center_idx]
                    if pattern_center != cell_val:
                        continue

                    if not match_pattern(cx, cy, rule):
                        continue

                    # Chance check
                    if rule["chance"] < 1.0 and random.random() > rule["chance"]:
                        continue

                    # Match! Generate tile
                    tile_id = random.choice(rule["tileIds"])
                    tile_x = (tile_id % 100) * 16  # tileset x pixel coord
                    tile_y = (tile_id // 100) * 16  # tileset y pixel coord (single row)

                    # LDtk autoLayerTiles use pixel coordinates
                    # For single-row tilesets: src.x = tileIndex * 16, src.y = 0
                    src_x = tile_id * 16
                    src_y = 0

                    tiles.append({
                        "px": [cx * 16, cy * 16],
                        "src": [src_x, src_y],
                        "f": 0,
                        "t": tile_id,
                        "d": [cy * grid_w + cx],
                        "a": 1.0,
                    })

                    matched = True
                    break  # breakOnMatch
                if matched:
                    break

    return tiles


# Process each level's layer instances
for level in data["levels"]:
    grid_w = level["pxWid"] // 16
    grid_h = level["pxHei"] // 16

    for li in level["layerInstances"]:
        layer_id = li["__identifier"]

        if layer_id == "Terrain":
            rule_groups = terrain_rules
            li["autoLayerTiles"] = apply_rules_to_intgrid(
                li["intGridCsv"], grid_w, grid_h, rule_groups, TERRAIN_TILESET_UID
            )
            li["__tilesetDefUid"] = TERRAIN_TILESET_UID
            li["__tilesetRelPath"] = "../tilesets/terrain.png"
            print(f"  {level['identifier']}/Terrain: {len(li['autoLayerTiles'])} auto-tiles")

        elif layer_id == "NestCells":
            rule_groups = nest_rules
            li["autoLayerTiles"] = apply_rules_to_intgrid(
                li["intGridCsv"], grid_w, grid_h, rule_groups, NEST_TILESET_UID
            )
            li["__tilesetDefUid"] = NEST_TILESET_UID
            li["__tilesetRelPath"] = "../tilesets/nest.png"
            print(f"  {level['identifier']}/NestCells: {len(li['autoLayerTiles'])} auto-tiles")


# ─── 7. Save ─────────────────────────────────────────────────────────

data["nextUid"] = next_uid

with open(LDTK_PATH, "w") as f:
    json.dump(data, f, indent="\t")

print(f"\nSaved {LDTK_PATH} (nextUid={next_uid})")
