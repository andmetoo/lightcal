"""Create and inspect Lightcal concepts through Lunacy's local stdio MCP only.

Requires the installed Lunacy application and its local MCP endpoint.
Creation adds new layers; explicit revise/contrast actions update existing layers.
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
    content = [rect("Current month title tint", pad-8, 22, 226 if expanded else 180,
                    34, palette["soft"], 9)]
    content.append(text("Month title", "October 2026", pad, 22, w-120, 34,
                        24 if expanded else 19, palette["accent"], True))
    if expanded:
        content.append(text("Current local date", "Thursday, 29 October · Current month", pad, 61,
                            360, 20, 12, palette["muted"]))
    else:
        content.append(text("Current local date", "Thu, 29 Oct · Current month", pad, 57,
                            270, 16, 10, palette["muted"]))
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
    for column_index in [5, 6]:
        content.append(rect("Gray weekday " + str(column_index),
                            pad+column_index*column+1, head_y-2, column-2,
                            grid_y-head_y+6*step, "2D3036" if dark else "F0F2F5", 7))
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
        # Synthetic examples only; live data comes from a separate Windows adapter.
        if not outside and day in [2, 8, 14, 20, 29, 30]:
            count = 2 if day in [14, 29, 30] else 1
            for dot in range(count):
                dot_color = palette["on"] if selected else palette["accent"] if dot == 0 else ("72C5B5" if dark else "167B68")
                content.append({"_t": "OVAL", "id": guid(),
                                "name": f"Event dot {day}/{dot}",
                                "frame": [cx+column/2-2+(dot-(count-1)/2)*5, cy+cell_size-5, 3, 3],
                                "fill": dot_color})
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
                text("Board note", "Today outlined · Selection filled · Gray weekends · Event dots (sample data)",
                     40, 650, 830, 20, 11, muted),
            ]}


def preferences_board(dark, x, y):
    ink, muted, surface, line, accent, soft = (
        ("F1F3F7", "A4AAB7", "24262B", "3A3D45", "9AC1FF", "303A4C") if dark else
        ("192230", "596579", "FFFFFF", "E1E5EC", "245FD0", "EDF3FF"))
    screens = []
    for index, title in enumerate(["Appearance", "General", "Calendars", "About"]):
        layers = [text("Settings title", title, 28, 22, 360, 36, 24, ink, True),
                  text("Settings breadcrumb", "Lightcal / Settings", 28, 64, 350, 20, 12, muted),
                  rect("Header divider", 28, 103, 384, 1, line)]

        def label(name, value, xx, yy, width=360, height=24, size=13, color=ink, bold=False):
            layers.append(text(name, value, xx, yy, width, height, size, color, bold))

        def button(name, value, xx, yy, width, active=False):
            layers.append(rect(name, xx, yy, width, 34, soft if active else surface,
                               8, border=accent if active else line, thickness=1))
            layers.append(text(name+" label", value, xx, yy, width, 34, 12,
                               accent if active else ink, active, 2))

        if title == "Appearance":
            label("Theme label", "Theme", 28, 124, bold=True)
            for i, value in enumerate(["Light", "System", "Dark"]):
                button("Theme " + value, value, 28+i*132, 159, 120, value == "System")
            label("Week start label", "First day of week", 28, 221, bold=True)
            button("First weekday", "Monday", 270, 218, 142)
            label("Gray weekdays label", "Gray weekdays", 28, 287, bold=True)
            label("Gray weekdays help", "Choose the days to shade in the calendar.", 28, 316,
                  size=12, color=muted)
            for i, day in enumerate(["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]):
                button("Gray " + day, day, 28+i*56, 353, 48, i >= 5)
            label("Weekday display label", "Show weekday in date", 28, 417, bold=True)
            button("Weekday display enabled", "On", 350, 412, 62, True)
        elif title == "General":
            label("Startup label", "Start with Windows", 28, 137, bold=True)
            label("Startup help", "Start quietly in the tray after sign-in.", 28, 167,
                  size=12, color=muted)
            button("Startup disabled", "Off", 350, 134, 62)
            layers.append(rect("Startup divider", 28, 213, 384, 1, line))
            label("Shortcut heading", "Global shortcut", 28, 245, bold=True)
            button("Unassigned shortcut", "Not set", 28, 287, 246)
            button("Record shortcut", "Record", 290, 287, 122)
            label("Shortcut help", "Choose a shortcut to toggle the compact calendar.",
                  28, 340, size=12, color=muted)
            label("Optional settings hint", "Both options are optional and off by default.",
                  28, 439, size=12, color=muted)
            label("Language heading", "Language", 28, 382, bold=True)
            button("Language selector", "English ▾", 270, 377, 142)
        elif title == "Calendars":
            label("Calendar source heading", "Windows calendars", 28, 126, bold=True)
            label("Calendar source help", "Choose which calendars appear in Lightcal.",
                  28, 157, size=12, color=muted)
            for i, (value, checked, color) in enumerate([
                    ("Work", True, accent), ("Personal", True, "167B68" if not dark else "72C5B5"),
                    ("Holidays", False, muted)]):
                yy = 205+i*66
                layers.append(rect("Calendar checkbox " + value, 28, yy, 20, 20,
                                   soft if checked else surface, 5,
                                   border=accent if checked else line, thickness=1))
                if checked:
                    layers.append(path("Checked " + value, "M3 8 L6 11 L13 4", 30, yy+2, accent))
                layers.append({"_t": "OVAL", "id": guid(), "name": "Calendar color " + value,
                               "frame": [65, yy+6, 8, 8], "fill": color})
                label("Calendar " + value, value, 86, yy-3, width=300, bold=True)
                layers.append(rect("Calendar row divider " + value, 28, yy+44, 384, 1, line))
            label("Calendar selection help", "Only selected calendars contribute event dots.",
                  28, 421, size=12, color=muted)
        else:
            label("App name", "Lightcal", 28, 130, height=42, size=30, bold=True)
            label("App version", "Development build", 28, 182, size=12, color=muted)
            label("App description", "A little calendar. Right where you need it.",
                  28, 222, size=13, color=muted)
            button("GitHub link", "GitHub ↗", 28, 273, 180)
            button("Koona link", "koona.app ↗", 224, 273, 188)
            label("License", "GPL-3.0-only · Open source", 28, 326, size=12, color=muted)
            button("Manual update check", "Check for updates", 28, 379, 384, True)
            label("Update help", "Checks only when requested. No automatic installation.",
                  28, 430, size=11, color=muted)
        screens.append({"_t": "FRAME", "id": guid(),
                        "name": f"Lightcal / Settings / {title} / {'Dark' if dark else 'Light'}",
                        "frame": [20+(index % 2)*460, 120+(index//2)*510, 440, 480],
                        "fill": surface, "cornerRadius": [16]*4,
                        "border": line, "thickness": 1, "layers": layers})
    return {"_t": "FRAME", "id": guid(),
            "name": "Lightcal preferences / " + ("Dark" if dark else "Light"),
            "frame": [x, y, 940, 1150], "fill": "17191E" if dark else "F0F2F6",
            "layers": [text("Preferences board title", "Make it yours", 28, 24, 800, 42, 28, ink, True),
                       text("Preferences board note", "Settings concepts · Calendar names and selections are illustrative",
                            28, 76, 870, 24, 13, muted), *screens]}


def reuse_ids(new_layer, old_layer):
    new_layer["id"] = old_layer["id"]
    old_children = {(layer["_t"], layer.get("name")): layer
                    for layer in old_layer.get("layers", [])}
    for child in new_layer.get("layers", []):
        previous = old_children.get((child["_t"], child.get("name")))
        if previous:
            reuse_ids(child, previous)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["probe", "area", "create", "revise", "preferences", "language", "layers", "contrast", "inspect", "export"])
    parser.add_argument("ids", nargs="*")
    parser.add_argument("--y", type=int, default=0, help="Top coordinate for an area inspection")
    parser.add_argument("--exe", default=r"C:\Program Files\Lunacy\Lunacy.exe")
    args = parser.parse_args()
    client = Lunacy(args.exe)
    try:
        if args.action == "probe":
            result = client.tool("get_viewport_info")
        elif args.action == "area":
            result = client.tool("get_layers_in_area", {
                "x": 0, "y": args.y, "width": 1960, "height": 1150, "maxDepth": 1})
        elif args.action == "create":
            client.tool("get_format_documentation")
            # Keep existing document content intact: inspect the destination first.
            occupied = client.tool("get_layers_in_area", {
                "x": 0, "y": 0, "width": 1960, "height": 800, "maxDepth": 1})
            print(json.dumps({"destination": occupied}), flush=True)
            layers = [board(False, 0, 0), board(True, 1000, 0)]
            result = client.tool("create_layers", {"parentId": "", "layersJson": json.dumps(layers)})
            print(json.dumps({"board_ids": [layer["id"] for layer in layers]}), flush=True)
        elif args.action == "revise":
            client.tool("get_format_documentation")
            existing = client.tool("get_layers_by_ids", {"ids": args.ids})
            updates = []
            for item in existing.get("content", []):
                if item.get("type") != "text":
                    continue
                for old in json.loads(item["text"]):
                    dark = old.get("name", "").endswith("Dark")
                    new = board(dark, 1000 if dark else 0, 0)
                    reuse_ids(new, old)
                    updates.append(new)
            if len(updates) != 2:
                raise RuntimeError("Expected exactly two existing concept boards")
            result = client.tool("update_layers", {"layersJson": json.dumps(updates)})
        elif args.action == "preferences":
            client.tool("get_format_documentation")
            layers = [preferences_board(False, 0, 820), preferences_board(True, 1000, 820)]
            result = client.tool("create_layers", {"parentId": "", "layersJson": json.dumps(layers)})
            print(json.dumps({"preferences_ids": [layer["id"] for layer in layers],
                              "screen_ids": [layer["id"] for board_layer in layers
                                             for layer in board_layer["layers"] if layer["_t"] == "FRAME"]}), flush=True)
        elif args.action == "language":
            client.tool("get_format_documentation")
            existing = client.tool("get_layers_by_ids", {"ids": args.ids})
            updates = []
            for item in existing.get("content", []):
                if item.get("type") != "text":
                    continue
                for old in json.loads(item["text"]):
                    dark = old.get("name", "").endswith("Dark")
                    template = preferences_board(dark, 0, 0)
                    general = next(layer for layer in template["layers"]
                                   if "/ General /" in layer.get("name", ""))
                    if "/ General /" not in old.get("name", ""):
                        raise RuntimeError("Language action requires General screen IDs")
                    reuse_ids(general, old)
                    updates.append(general)
            result = client.tool("update_layers", {"layersJson": json.dumps(updates)})
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
