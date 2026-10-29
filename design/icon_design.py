"""Import the repository's original SVG artwork into Lunacy using MCP only.

Use a free canvas area at y=2100. Repeated runs create additional boards.
"""

import base64
import argparse
import json
from pathlib import Path
from lunacy_design import Lunacy, guid, rect, text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--board-id", help="Resume imports into an existing icon board")
    parser.add_argument("--remove-duplicate", help="Remove a known duplicate generated icon board")
    parser.add_argument("--repair-placement", action="store_true", help="Repair older relative-coordinate imports")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    client = Lunacy(r"C:\Program Files\Lunacy\Lunacy.exe")
    try:
        client.tool("get_format_documentation")
        board_id = args.board_id or guid()
        print(json.dumps({"board_id": board_id}), flush=True)
        if args.repair_placement:
            if not args.board_id:
                raise RuntimeError("Placement repair requires an existing board ID")
            existing = client.tool("get_layers_by_ids", {"ids": [board_id]})
            raw = json.loads(next(item["text"] for item in existing["content"] if item.get("type") == "text"))
            root_layer = raw[0] if isinstance(raw, list) else raw
            updates = []
            for layer in root_layer.get("layers", []):
                if layer["_t"] == "FRAME" and layer.get("pos", [0, 0])[1] < 0:
                    layer["pos"][1] += 2100
                    updates.append(layer)
            for layer in root_layer.get("layers", []):
                if layer.get("name", "").startswith("Size "):
                    layer["pos"][1] = 280
            root_layer["layers"].sort(key=lambda layer: 0 if layer.get("name") == "Dark tray test background" else 1)
            print(json.dumps(client.tool("update_layers", {"layersJson": json.dumps([root_layer])})), flush=True)
            return
        if args.remove_duplicate:
            existing = client.tool("get_layers_by_ids", {"ids": [args.remove_duplicate]})
            data = []
            for item in existing["content"]:
                if item.get("type") == "text":
                    parsed = json.loads(item["text"])
                    data.extend(parsed if isinstance(parsed, list) else [parsed])
            matching = [layer for layer in data if layer.get("id") == args.remove_duplicate]
            if len(matching) != 1 or matching[0].get("name") != "Lightcal / Icon exploration":
                raise RuntimeError("Refusing duplicate removal: " + repr([
                    {"id": layer.get("id"), "name": layer.get("name")} for layer in data]))
            print(json.dumps(client.tool("delete_layers", {"layerIds": [args.remove_duplicate]})), flush=True)
        layers = [
            text("Icon board title", "Lightcal / App icon", 40, 28, 900, 44, 28, "192230", True),
            text("Icon board subtitle", "One calendar. One bright day. Vector artwork, no text or raster effects.",
                 40, 83, 950, 24, 14, "596579"),
            text("Large label", "APP ICON", 40, 135, 340, 24, 11, "596579", True),
            text("Small label", "WINDOWS / ACTUAL PIXEL SIZES", 420, 135, 590, 24, 11, "596579", True),
            rect("Dark tray test background", 420, 330, 540, 150, "24262B", 16),
            text("Dark tray test label", "MONOCHROME / DARK SURFACE", 444, 350, 480, 20, 11, "F1F3F7", True),
            text("Light tray test label", "MONOCHROME / LIGHT SURFACE", 420, 520, 540, 20, 11, "596579", True),
            text("Scale note", "1024 × 1024 master · Separate foreground + unmasked background for macOS adaptation",
                 40, 690, 950, 28, 13, "596579"),
        ]
        for size, xx in [(16, 420), (24, 500), (32, 590), (48, 690), (64, 810)]:
            layers.append(text(f"Size {size}", str(size), xx, 280, 70, 24, 12, "596579"))
        board = {"_t": "FRAME", "id": board_id, "name": "Lightcal / Icon exploration",
                 "frame": [0, 2100, 1040, 760], "fill": "F0F2F6", "layers": layers}
        if not args.board_id:
            print(json.dumps(client.tool("create_layers", {"parentId": "", "layersJson": json.dumps([board])})), flush=True)
        outputs = []

        def insert(file_name, size, xx, yy, color=None):
            svg = (root / "assets" / "icons" / file_name).read_text(encoding="utf-8")
            original = "24" if file_name == "lightcal-tray.svg" else "1024"
            svg = svg.replace(f'width="{original}" height="{original}"',
                              f'width="{size}" height="{size}"', 1)
            if color:
                svg = svg.replace("currentColor", color)
            result = client.tool("insert_image", {
                "parentId": board_id, "imageBase64": base64.b64encode(svg.encode()).decode(),
                "fileName": file_name, "x": xx, "y": yy+2100})
            outputs.append({"file": file_name, "size": size, "result": result})

        insert("lightcal.svg", 320, 40, 180)
        for size, xx in [(16, 420), (24, 500), (32, 590), (48, 690), (64, 810)]:
            insert("lightcal.svg", size, xx, 202)
        for size, xx in [(16, 444), (24, 524), (32, 614)]:
            insert("lightcal-tray.svg", size, xx, 400, "#ffffff")
            insert("lightcal-tray.svg", size, xx-24, 570, "#192230")
        # SVG imports are inserted behind existing children. Keep the dark test
        # background behind those imports so white tray marks remain visible.
        existing = client.tool("get_layers_by_ids", {"ids": [board_id]})
        raw = json.loads(next(item["text"] for item in existing["content"] if item.get("type") == "text"))
        root_layer = raw[0] if isinstance(raw, list) else raw
        root_layer["layers"].sort(key=lambda layer: 0 if layer.get("name") == "Dark tray test background" else 1)
        client.tool("update_layers", {"layersJson": json.dumps([root_layer])})
        print(json.dumps({"board_id": board_id, "imports": outputs}), flush=True)
    finally:
        client.close()


if __name__ == "__main__":
    main()
