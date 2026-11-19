"""Turn the design boards in design/source/*.dc.html into standalone previews.

The boards are the exact files from the published design sketch. They are written
for a design-canvas runtime (<x-dc>, <helmet>, {{template}} values). This script
strips that wrapper so each board opens in any browser on its own, with the font
loaded from design/fonts. Nothing about the layout is changed: only the wrapper
tags are removed, {{accent}} becomes the default accent, and the Settings board's
small template loops are expanded to their default state.

Usage:  python tools/make_previews.py
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "design" / "source"
OUT = ROOT / "design" / "preview"

ACCENT = "#5ad0f0"

FONT_CSS = "\n".join(
    "@font-face{font-family:'Familjen Grotesk';font-style:normal;font-weight:%d;"
    "font-display:block;src:url('../fonts/familjen-grotesk-latin-%d-normal.woff2') format('woff2')}" % (w, w)
    for w in (400, 500, 600, 700)
)


def default_accent(raw):
    """Each board declares its own default accent in data-props (Private uses lilac)."""
    m = re.search(r"data-props='(.*?)'>", raw, re.S)
    if m:
        return json.loads(m.group(1)).get("accent", {}).get("default", ACCENT)
    return ACCENT


def settings_state(accent=ACCENT):
    """Default state of the Settings board (mirrors its DCLogic renderVals)."""
    stars, trail, shoot = "still", "narrow", False

    def seg(items, cur):
        return [
            {
                "label": label,
                "style": "background:rgba(255,255,255,.14);color:#f2f5f7;" if i == cur
                else "background:transparent;color:#7d868f;",
            }
            for i, label in items
        ]

    swatches = [
        {"name": n, "color": c,
         "ring": ("box-shadow:0 0 0 2px #0a0c0f,0 0 0 4px " + c + ";") if c == accent else ""}
        for n, c in [("Sea", "#5ad0f0"), ("Mint", "#6ee7b7"), ("Lilac", "#b6a8ff"),
                     ("Amber", "#f2c46d"), ("Rose", "#ff8fa3")]
    ]
    return {
        "accent": accent,
        "starsOpts": seg([("off", "Off"), ("still", "Still"), ("drift", "Drifting")], stars),
        "trailOpts": seg([("narrow", "Narrow"), ("wide", "Wide")], trail),
        "swatches": swatches,
        "shootOn": "true" if shoot else "false",
        "toggleBg": accent if shoot else "rgba(255,255,255,.14)",
        "knobPos": "left:21px;" if shoot else "left:3px;",
    }


def expand_loops(html, vals):
    pat = re.compile(r'<sc-for list="\{\{(\w+)\}\}" as="(\w+)"[^>]*>(.*?)</sc-for>', re.S)

    def rep(m):
        items, name, body = vals[m.group(1)], m.group(2), m.group(3)
        out = []
        for it in items:
            b = body
            for k, v in it.items():
                b = b.replace("{{%s.%s}}" % (name, k), str(v))
            # event handlers have no meaning in a static preview
            b = re.sub(r'\s+onClick="\{\{%s\.\w+\}\}"' % name, "", b)
            out.append(b)
        return "".join(out)

    return pat.sub(rep, html)


def convert(path):
    raw = path.read_text(encoding="utf-8")
    title = re.search(r"<title>(.*?)</title>", raw, re.S).group(1)
    helmet = re.search(r"<helmet>(.*?)</helmet>", raw, re.S).group(1)
    styles = "".join(re.findall(r"<style>(.*?)</style>", helmet, re.S))
    body = re.search(r"</helmet>(.*?)<script type=\"text/x-dc\"", raw, re.S).group(1)

    accent = default_accent(raw)
    vals = settings_state(accent) if "sc-for" in body else {"accent": accent}
    body = expand_loops(body, vals)
    body = re.sub(r'\s+onClick="\{\{\w+\}\}"', "", body)
    for k, v in vals.items():
        if isinstance(v, str):
            body = body.replace("{{%s}}" % k, v)
    left = re.findall(r"\{\{[^}]+\}\}", body)
    if left:
        raise SystemExit("%s: unresolved template values %s" % (path.name, sorted(set(left))))

    return (
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n"
        "<title>%s</title>\n<style>%s\n%s\nhtml,body{width:1280px;height:800px;overflow:hidden}</style>\n"
        "</head>\n<body>%s</body>\n</html>\n" % (title, FONT_CSS, styles, body.strip())
    )


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    order = json.loads((SRC / "canvas.json").read_text(encoding="utf-8"))["order"]
    for name in order:
        out = OUT / name.replace(".dc.html", ".html")
        out.write_text(convert(SRC / name), encoding="utf-8")
        print("wrote", out.relative_to(ROOT))


if __name__ == "__main__":
    main()
