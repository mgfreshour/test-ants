#!/usr/bin/env python3
"""Add entity definitions and placements to colony.ldtk for Sprint L3."""

import json
import uuid
import random

LDTK_PATH = "assets/maps/colony.ldtk"

# Level IIDs
SURFACE_IID = "9590239f-aaf8-463e-a9d3-2a54c2d23d3a"
PLAYER_NEST_IID = "1228f450-47bb-4e81-83fd-246ad316535b"
RED_NEST_IID = "29005165-cb0a-48eb-8899-991f1fa612a2"

# World dimensions
WORLD_W, WORLD_H = 2048, 2048
NEST_W, NEST_H = 960, 640  # 60*16, 40*16

# Nest position in Bevy coords
NEST_POS_BEVY = (512.0, 512.0)
RED_NEST_POS_BEVY = (1536.0, 1536.0)

# UIDs (must not conflict with existing: Terrain=1, NestCells=2)
ENTITIES_LAYER_UID = 3
ENT_FOOD_UID = 100
ENT_PORTAL_UID = 101
ENT_QUEEN_UID = 102
ENT_ENTRANCE_UID = 103

# Field UIDs
FLD_AMOUNT_UID = 200
FLD_MAX_UID = 201
FLD_SIZE_UID = 202
FLD_PORTAL_ID_UID = 203
FLD_COLONY_ID_UID = 204


def make_iid():
    return str(uuid.uuid4())


def bevy_to_ldtk_surface(bx, by):
    """Convert Bevy world pos to LDtk Surface pixel coords (Y-down)."""
    return (int(bx), int(WORLD_H - by))


def nest_grid_to_ldtk_px(gx, gy):
    """Convert NestGrid (gx, gy) to LDtk nest level pixel coords (Y-down).
    Centers in the cell (adds 8px offset for 16px cells)."""
    return (gx * 16 + 8, gy * 16 + 8)


def make_field_def(uid, identifier, field_type, default_val=None):
    """Create a minimal FieldDefinition JSON object."""
    fd = {
        "__type": field_type,
        "acceptFileTypes": None,
        "allowedRefs": "Any",
        "allowedRefsEntityUid": None,
        "allowedRefTags": [],
        "allowOutOfLevelRef": True,
        "arrayMaxLength": None,
        "arrayMinLength": None,
        "autoChainRef": True,
        "canBeNull": False,
        "defaultOverride": None,
        "doc": None,
        "editorAlwaysShow": False,
        "editorCutLongValues": True,
        "editorDisplayColor": None,
        "editorDisplayMode": "Hidden",
        "editorDisplayPos": "Above",
        "editorDisplayScale": 1.0,
        "editorLinkStyle": "StraightArrow",
        "editorShowInWorld": True,
        "editorTextPrefix": None,
        "editorTextSuffix": None,
        "exportToToc": False,
        "identifier": identifier,
        "isArray": False,
        "max": None,
        "min": None,
        "regex": None,
        "searchable": True,
        "symmetricalRef": False,
        "textLanguageMode": None,
        "tilesetUid": None,
        "type": f"F_{field_type}",
        "uid": uid,
        "useForSmartColor": False,
    }
    if default_val is not None:
        fd["defaultOverride"] = {"id": "V_Float" if field_type == "Float" else "V_Int" if field_type == "Int" else "V_String", "params": [str(default_val)]}
    return fd


def make_entity_def(uid, identifier, field_defs, color="#BE4A2F", w=16, h=16,
                    pivot_x=0.5, pivot_y=0.5):
    """Create a minimal EntityDefinition JSON object."""
    return {
        "allowOutOfBounds": False,
        "color": color,
        "doc": None,
        "exportToToc": False,
        "fieldDefs": field_defs,
        "fillOpacity": 1.0,
        "height": h,
        "hollow": False,
        "identifier": identifier,
        "keepAspectRatio": False,
        "limitBehavior": "MoveLastOne",
        "limitScope": "PerLevel",
        "lineOpacity": 1.0,
        "maxCount": 0,
        "maxHeight": None,
        "maxWidth": None,
        "minHeight": None,
        "minWidth": None,
        "nineSliceBorders": [],
        "pivotX": pivot_x,
        "pivotY": pivot_y,
        "renderMode": "Rectangle",
        "resizableX": False,
        "resizableY": False,
        "showName": True,
        "tags": [],
        "tileId": None,
        "tileOpacity": 1.0,
        "tileRect": None,
        "tileRenderMode": "FitInside",
        "tilesetId": None,
        "uid": uid,
        "uiTileRect": None,
        "width": w,
    }


def make_entity_instance(def_uid, identifier, px, width, height,
                         field_instances=None, pivot=(0.5, 0.5)):
    """Create an EntityInstance JSON object."""
    return {
        "__grid": [px[0] // 16, px[1] // 16],
        "__identifier": identifier,
        "__pivot": list(pivot),
        "__smartColor": "#BE4A2F",
        "__tags": [],
        "__tile": None,
        "__worldX": None,
        "__worldY": None,
        "defUid": def_uid,
        "fieldInstances": field_instances or [],
        "height": height,
        "iid": make_iid(),
        "px": list(px),
        "width": width,
    }


def make_field_instance(identifier, field_type, value, def_uid):
    """Create a FieldInstance JSON object."""
    return {
        "__identifier": identifier,
        "__tile": None,
        "__type": field_type,
        "__value": value,
        "defUid": def_uid,
        "realEditorValues": [{"id": "V_Float" if field_type == "Float" else "V_Int" if field_type == "Int" else "V_String", "params": [str(value)]}],
    }


def make_entities_layer_def():
    """Create the Entities layer definition."""
    return {
        "__type": "Entities",
        "autoRuleGroups": [],
        "autoSourceLayerDefUid": None,
        "autoTilesetDefUid": None,
        "autoTilesKilledByOtherLayerUid": None,
        "biomeFieldUid": None,
        "canSelectWhenInactive": True,
        "displayOpacity": 1.0,
        "doc": None,
        "excludedTags": [],
        "gridSize": 16,
        "guideGridHei": 0,
        "guideGridWid": 0,
        "hideFieldsWhenInactive": False,
        "hideInList": False,
        "identifier": "Entities",
        "inactiveOpacity": 0.6,
        "intGridValues": [],
        "intGridValuesGroups": [],
        "parallaxFactorX": 0.0,
        "parallaxFactorY": 0.0,
        "parallaxScaling": True,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "renderInWorldView": True,
        "requiredTags": [],
        "tilePivotX": 0.0,
        "tilePivotY": 0.0,
        "tilesetDefUid": None,
        "type": "Entities",
        "uid": ENTITIES_LAYER_UID,
        "uiColor": None,
        "uiFilterTags": [],
        "useAsyncRender": False,
    }


def make_entities_layer_instance(level_width_px, level_height_px, entity_instances):
    """Create an entity layer instance for a level."""
    return {
        "__cHei": level_height_px // 16,
        "__cWid": level_width_px // 16,
        "__gridSize": 16,
        "__identifier": "Entities",
        "__opacity": 1.0,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": None,
        "__tilesetRelPath": None,
        "__type": "Entities",
        "autoLayerTiles": [],
        "entityInstances": entity_instances,
        "gridTiles": [],
        "iid": make_iid(),
        "intGridCsv": [],
        "layerDefUid": ENTITIES_LAYER_UID,
        "levelId": 0,
        "optionalRules": [],
        "overrideTilesetUid": None,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "seed": random.randint(1000000, 9999999),
        "visible": True,
    }


def generate_food_sources():
    """Generate ~20 food source entity instances for the surface level."""
    margin = 150
    nest = NEST_POS_BEVY
    foods = []

    # Predefined food positions (in Bevy coords) spread across the map
    food_configs = [
        # (bevy_x, bevy_y, amount, max, size)
        # Large fruit clusters
        (900, 400, 15.0, 18.0, 18.0),
        (300, 1200, 15.0, 18.0, 18.0),
        (1600, 800, 15.0, 18.0, 18.0),
        (1100, 1500, 15.0, 18.0, 18.0),
        (400, 700, 15.0, 18.0, 18.0),
        (1800, 1300, 15.0, 18.0, 18.0),
        (700, 1800, 15.0, 18.0, 18.0),
        # Dead insects
        (1200, 300, 12.0, 12.0, 12.0),
        (500, 1600, 12.0, 12.0, 12.0),
        (1700, 500, 12.0, 12.0, 12.0),
        (800, 1100, 12.0, 12.0, 12.0),
        (1400, 1700, 12.0, 12.0, 12.0),
        (200, 900, 12.0, 12.0, 12.0),
        # Crumbs
        (600, 300, 5.0, 6.0, 6.0),
        (1000, 700, 5.0, 6.0, 6.0),
        (1500, 1100, 5.0, 6.0, 6.0),
        (350, 1500, 5.0, 6.0, 6.0),
        (1850, 600, 5.0, 6.0, 6.0),
        (750, 1400, 5.0, 6.0, 6.0),
        (1300, 200, 5.0, 6.0, 6.0),
        (450, 450, 5.0, 6.0, 6.0),
    ]

    for bx, by, amount, max_val, size in food_configs:
        px = bevy_to_ldtk_surface(bx, by)
        fields = [
            make_field_instance("amount", "Float", amount, FLD_AMOUNT_UID),
            make_field_instance("max_amount", "Float", max_val, FLD_MAX_UID),
            make_field_instance("size", "Float", size, FLD_SIZE_UID),
        ]
        foods.append(make_entity_instance(
            ENT_FOOD_UID, "FoodSource", px, 16, 16,
            field_instances=fields,
        ))

    return foods


def generate_surface_entities():
    """Generate all entity instances for the Surface level."""
    entities = generate_food_sources()

    # Player nest entrance marker
    px = bevy_to_ldtk_surface(*NEST_POS_BEVY)
    entities.append(make_entity_instance(
        ENT_ENTRANCE_UID, "NestEntrance", px, 28, 28,
    ))

    # Red nest entrance marker
    px = bevy_to_ldtk_surface(*RED_NEST_POS_BEVY)
    entities.append(make_entity_instance(
        ENT_ENTRANCE_UID, "NestEntrance", px, 28, 28,
    ))

    # Portal: surface side of player nest entrance
    px = bevy_to_ldtk_surface(*NEST_POS_BEVY)
    fields = [
        make_field_instance("portal_id", "String", "player_nest", FLD_PORTAL_ID_UID),
        make_field_instance("colony_id", "Int", 0, FLD_COLONY_ID_UID),
    ]
    entities.append(make_entity_instance(
        ENT_PORTAL_UID, "PortalPoint", px, 16, 16,
        field_instances=fields,
    ))

    # Portal: surface side of red nest entrance
    px = bevy_to_ldtk_surface(*RED_NEST_POS_BEVY)
    fields = [
        make_field_instance("portal_id", "String", "red_nest", FLD_PORTAL_ID_UID),
        make_field_instance("colony_id", "Int", 1, FLD_COLONY_ID_UID),
    ]
    entities.append(make_entity_instance(
        ENT_PORTAL_UID, "PortalPoint", px, 16, 16,
        field_instances=fields,
    ))

    return entities


def generate_nest_entities(portal_id, colony_id):
    """Generate entity instances for a nest level."""
    entities = []

    # Queen spawn at queen chamber center: grid (30, 16)
    qx, qy = nest_grid_to_ldtk_px(30, 16)
    entities.append(make_entity_instance(
        ENT_QUEEN_UID, "QueenSpawn", (qx, qy), 16, 16,
    ))

    # Portal: nest side at entrance tunnel top: grid (30, 0)
    px, py = nest_grid_to_ldtk_px(30, 0)
    fields = [
        make_field_instance("portal_id", "String", portal_id, FLD_PORTAL_ID_UID),
        make_field_instance("colony_id", "Int", colony_id, FLD_COLONY_ID_UID),
    ]
    entities.append(make_entity_instance(
        ENT_PORTAL_UID, "PortalPoint", (px, py), 16, 16,
        field_instances=fields,
    ))

    return entities


def main():
    with open(LDTK_PATH, "r") as f:
        data = json.load(f)

    # --- Add entity definitions ---
    food_fields = [
        make_field_def(FLD_AMOUNT_UID, "amount", "Float"),
        make_field_def(FLD_MAX_UID, "max_amount", "Float"),
        make_field_def(FLD_SIZE_UID, "size", "Float"),
    ]
    portal_fields = [
        make_field_def(FLD_PORTAL_ID_UID, "portal_id", "String"),
        make_field_def(FLD_COLONY_ID_UID, "colony_id", "Int"),
    ]

    data["defs"]["entities"] = [
        make_entity_def(ENT_FOOD_UID, "FoodSource", food_fields,
                       color="#E8C170", w=16, h=16),
        make_entity_def(ENT_PORTAL_UID, "PortalPoint", portal_fields,
                       color="#4A90D9", w=16, h=16),
        make_entity_def(ENT_QUEEN_UID, "QueenSpawn", [],
                       color="#D4A017", w=16, h=16),
        make_entity_def(ENT_ENTRANCE_UID, "NestEntrance", [],
                       color="#5C3A1E", w=28, h=28),
    ]

    # --- Add Entities layer definition ---
    # Insert at beginning so it renders on top
    data["defs"]["layers"].insert(0, make_entities_layer_def())

    # --- Add entity layer instances to each level ---
    for level in data["levels"]:
        iid = level["iid"]

        if iid == SURFACE_IID:
            entity_instances = generate_surface_entities()
            layer = make_entities_layer_instance(WORLD_W, WORLD_H, entity_instances)
        elif iid == PLAYER_NEST_IID:
            entity_instances = generate_nest_entities("player_nest", 0)
            layer = make_entities_layer_instance(NEST_W, NEST_H, entity_instances)
        elif iid == RED_NEST_IID:
            entity_instances = generate_nest_entities("red_nest", 1)
            layer = make_entities_layer_instance(NEST_W, NEST_H, entity_instances)
        else:
            continue

        # Insert entity layer at beginning of layerInstances
        if level.get("layerInstances") is None:
            level["layerInstances"] = []
        level["layerInstances"].insert(0, layer)

    # --- Write back ---
    with open(LDTK_PATH, "w") as f:
        json.dump(data, f, separators=(",", ":"))

    print("Done! Added entity definitions and placements to colony.ldtk")
    print(f"  - 4 entity types (FoodSource, PortalPoint, QueenSpawn, NestEntrance)")
    print(f"  - Entities layer definition (uid={ENTITIES_LAYER_UID})")
    print(f"  - Surface: {len(generate_surface_entities())} entities")
    print(f"  - PlayerNest: {len(generate_nest_entities('x', 0))} entities")
    print(f"  - RedNest: {len(generate_nest_entities('x', 1))} entities")


if __name__ == "__main__":
    main()
