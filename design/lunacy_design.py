"""Create and inspect Lightcal concepts through Lunacy's local stdio MCP only.

Requires the installed Lunacy application and its local MCP endpoint.
Creation adds new layers to the active document; it never edits existing layers.
Run `python design/lunacy_design.py probe` before `create`.
"""

import argparse
import json
import queue
import subprocess
import threading
import uuid
import base64


class Lunacy:
    def __init__(self, executable):
        self.process = subprocess.Popen(
            [executable, "-mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, encoding="utf-8",
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        self.responses = queue.Queue()
        self.sequence = 0
        threading.Thread(target=self._read, daemon=True).start()
        self.rpc("initialize", {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "lightcal-design", "version": "1.0"},
        })
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def _read(self):
        for line in self.process.stdout:
            try:
                self.responses.put(json.loads(line))
            except ValueError:
                continue

    def _send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def rpc(self, method, params):
        self.sequence += 1
        self._send({"jsonrpc": "2.0", "id": self.sequence,
                    "method": method, "params": params})
        while True:
            response = self.responses.get(timeout=45)
            if response.get("id") != self.sequence:
                continue
            if "error" in response:
                raise RuntimeError(response["error"])
            return response["result"]

    def tool(self, name, arguments=None):
        result = self.rpc("tools/call", {"name": name, "arguments": arguments or {}})
        if result.get("isError"):
            raise RuntimeError(result)
        return result

    def close(self):
        self.process.terminate()


def guid():
    return base64.urlsafe_b64encode(uuid.uuid4().bytes).decode().rstrip("=")


def rect(name, x, y, width, height, color, radius=0, **props):
    return {"_t": "RECT", "id": guid(), "name": name,
            "frame": [x, y, width, height], "fill": color,
            "cornerRadius": [radius] * 4, **props}


def text(name, value, x, y, width, height, size, color, bold=False, align=0):
    return {"_t": "TEXT", "id": guid(), "name": name, "text": value,
            "frame": [x, y, width, height], "font": "SegoeUI-Semibold" if bold else "SegoeUI",
            "fontSize": size, "fill": color, "behavior": 2,
            "align": align, "valign": 1}


def path(name, data, x, y, color, width=1.5):
    return {"_t": "PATH", "id": guid(), "name": name, "frame": [x, y, 16, 16],
            "pathData": data, "border": color, "thickness": width,
            "lineCap": 1, "lineJoin": 1, "open": True}


def calendar(expanded, dark, x, y):
    theme = "Dark" if dark else "Light"
    mode = "Expanded" if expanded else "Compact"
    palette = ({"bg": "24262B", "ink": "F1F3F7", "muted": "A4AAB7",
                "faint": "A4AAB7", "line": "3A3D45", "accent": "9AC1FF",
                "selected": "9AC1FF", "on": "102A50", "soft": "303A4C"}
               if dark else
               {"bg": "FFFFFF", "ink": "192230", "muted": "596579",
                "faint": "596579", "line": "E1E5EC", "accent": "245FD0",
                "selected": "245FD0", "on": "FFFFFF", "soft": "EDF3FF"})
    w, h = (440, 480) if expanded else (320, 360)
    pad = 28 if expanded else 20
    content = []
    content.append(text("Month title", "October 2026", pad, 22, w-120, 34,
                        24 if expanded else 19, palette["ink"], True))
    if expanded:
        content.append(text("Current local date", "Today · Thursday, 29 October", pad, 61,
                            300, 20, 12, palette["muted"]))
    for previous, bx in [(True, w-88), (False, w-52)]:
        content.append(rect("Previous month" if previous else "Next month", bx, 25, 30, 30,
                            palette["bg"], 8, border=palette["line"], thickness=1))
        content.append(path("Previous chevron" if previous else "Next chevron",
                            "M10 4 L6 8 L10 12" if previous else "M6 4 L10 8 L6 12",
                            bx+7, 32, palette["ink"]))
    head_y = 104 if expanded else 77
    grid_y = 138 if expanded else 108
    step = 39 if expanded else 31
    column = (w-2*pad)/7
    cell_size = 36 if expanded else 28
    for i, day in enumerate(["MO", "TU", "WE", "TH", "FR", "SA", "SU"]):
        content.append(text("Weekday " + day, day, pad+i*column, head_y, column, 20,
                            10, palette["muted"], True, 2))
    for i in range(42):
        day = i-2
        outside = day < 1 or day > 31
        number = day+30 if day < 1 else day-31 if day > 31 else day
        cx = pad+(i % 7)*column
        cy = grid_y+(i//7)*step
        today, selected = day == 29, day == 30
        if today or selected:
            content.append(rect("Selected day" if selected else "Today outline",
                                cx+(column-cell_size)/2, cy, cell_size, cell_size,
                                palette["selected"] if selected else palette["bg"], 9,
                                border=palette["selected"], thickness=1.5))
        color = (palette["on"] if selected else palette["accent"] if today
                 else palette["faint"] if outside else palette["ink"])
        content.append(text(("Adjacent month " if outside else "October ")+str(number),
                            str(number), cx, cy, column, cell_size,
                            16 if expanded else 13, color, today or selected, 2))
    divider_y = 390 if expanded else 308
    content.append(rect("Footer divider", pad, divider_y, w-2*pad, 1, palette["line"]))
    if expanded:
        content.append(text("Selection caption", "SELECTED DATE", pad, 406, 200, 16,
                            9, palette["muted"], True))
        content.append(text("Selected date", "Friday, 30 October", pad, 426, 250, 24,
                            15, palette["ink"], True))
        bx, by, bw = w-pad-80, 418, 80
    else:
        bx, by, bw = pad, 320, 64
        content.append(text("ISO week number", "Week 44", 171, 320, 80, 26, 11,
                            palette["muted"], align=1))
        content.append(path("Settings sliders", "M3 4 L13 4 M3 8 L13 8 M3 12 L13 12 M6 2 L6 6 M10 6 L10 10 M6 10 L6 14",
                            280, 325, palette["muted"], 1.3))
    content.append(rect("Return to today", bx, by, bw, 30 if expanded else 26,
                        palette["soft"], 8))
    content.append(text("Today button label", "Today", bx, by, bw,
                        30 if expanded else 26, 12, palette["accent"], True, 2))
    return {"_t": "FRAME", "id": guid(), "name": f"Lightcal / {mode} / {theme}",
            "frame": [x, y, w, h], "fill": palette["bg"], "cornerRadius": [16]*4,
            "border": palette["line"], "thickness": 1,
            "shadows": [{"radius": 28, "offset": [0, 12], "color": "18000000"}],
            "layers": content}


def board(dark, x, y):
    ink, muted = ("F0F2F7", "A4AAB7") if dark else ("192230", "596579")
    return {"_t": "FRAME", "id": guid(),
            "name": "Lightcal concepts / " + ("Dark" if dark else "Light"),
            "frame": [x, y, 920, 690], "fill": "17191E" if dark else "F0F2F6",
            "layers": [
                text("Board title", "Lightcal", 40, 28, 280, 40, 28, ink, True),
                text("Board description", "A little calendar. Right where you need it.",
                     40, 76, 700, 24, 14, muted),
                text("Compact caption", "TRAY  /  COMPACT", 40, 170, 320, 20, 10, muted, True),
                calendar(False, dark, 40, 204),
                text("Expanded caption", "TASKBAR  /  EXPANDED", 420, 120, 440, 20, 10, muted, True),
                calendar(True, dark, 420, 154),
                text("Board note", "29  Today outline     30  Selected date     ·     Monday first / ISO weeks",
                     40, 650, 830, 20, 11, muted),
            ]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["probe", "area", "create", "layers", "contrast", "inspect", "export"])
    parser.add_argument("ids", nargs="*")
    parser.add_argument("--exe", default=r"C:\Program Files\Lunacy\Lunacy.exe")
    args = parser.parse_args()
    client = Lunacy(args.exe)
    try:
        if args.action == "probe":
            result = client.tool("get_viewport_info")
        elif args.action == "area":
            result = client.tool("get_layers_in_area", {
                "x": 0, "y": 0, "width": 1960, "height": 800, "maxDepth": 1})
        elif args.action == "create":
            client.tool("get_format_documentation")
            # Keep existing document content intact: inspect the destination first.
            occupied = client.tool("get_layers_in_area", {
                "x": 0, "y": 0, "width": 1960, "height": 800, "maxDepth": 1})
            print(json.dumps({"destination": occupied}), flush=True)
            layers = [board(False, 0, 0), board(True, 1000, 0)]
            result = client.tool("create_layers", {"parentId": "", "layersJson": json.dumps(layers)})
            print(json.dumps({"board_ids": [layer["id"] for layer in layers]}), flush=True)
        elif args.action == "layers":
            client.tool("get_format_documentation")
            result = client.tool("get_layers_by_ids", {"ids": args.ids})
        elif args.action == "contrast":
            client.tool("get_format_documentation")
            existing = client.tool("get_layers_by_ids", {"ids": args.ids})
            updates = []

            def collect(layer, color):
                if layer.get("name", "").startswith("Adjacent month "):
                    layer["fill"] = color
                    updates.append(layer)
                for child in layer.get("layers", []):
                    collect(child, color)

            for item in existing.get("content", []):
                if item.get("type") == "text":
                    for layer in json.loads(item["text"]):
                        collect(layer, "A4AAB7" if layer.get("name", "").endswith("Dark") else "596579")
            if not updates:
                raise RuntimeError("No adjacent-month text layers found; no changes applied")
            result = client.tool("update_layers", {"layersJson": json.dumps(updates)})
        elif args.action == "inspect":
            result = client.tool("get_screenshot_by_ids", {"ids": args.ids})
        else:
            result = client.tool("export", {"layerIds": args.ids, "format": "PNG", "scale": 1})
        print(json.dumps(result), flush=True)
    finally:
        client.close()


if __name__ == "__main__":
    main()
