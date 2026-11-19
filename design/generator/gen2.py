import random, json

W, H = 1280, 800
MAIN = "#e8ecef"; MUTED = "#7d868f"; SOFT = "#aeb7bf"; NODE = "#5b656f"
FONT = "'Familjen Grotesk',system-ui,sans-serif"
STAR = "M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z"
RESET = "border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;"
I = {
 "back": "M15 6l-6 6 6 6", "fwd": "M9 6l6 6-6 6", "reload": "M20 12a8 8 0 1 1-2.3-5.6M20 4v5h-5",
 "plus": "M12 5v14M5 12h14", "dl": "M12 7v8m0 0l-3-3m3 3l3-3", "shield": "M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z",
 "pause": "M9 6v12M15 6v12", "chev": "M7 10l5 5 5-5", "mail": "M4 7l8 6 8-6", "cal": "M4 10h16M8 3v4M16 3v4",
 "note": "M6 4h9l3 3v13H6zM9 11h6M9 15h6", "folder": "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
 "map": "M9 4l-5 2v14l5-2 6 2 5-2V4l-5 2zM9 4v14M15 6v14", "min": "M6 12h12", "close": "M6 6l12 12M18 6L6 18",
 "lock": "M8 11V8a4 4 0 0 1 8 0v3", "side": "M9 4v16",
}

def ico(d, size=18, color="currentColor", sw=1.6, extra=""):
    return ('<svg width="%d" height="%d" viewBox="0 0 24 24" fill="none" stroke="%s" stroke-width="%s" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">%s%s</svg>'
            % (size, size, color, sw, '<path d="%s"/>' % d if d else "", extra))

def star(size, fill, x=None, y=None):
    pos = "" if x is None else ' style="position:absolute;left:%spx;top:%spx"' % (x - size / 2, y - size / 2)
    return '<svg width="%d" height="%d" viewBox="0 0 24 24"%s aria-hidden="true"><path d="%s" fill="%s"/></svg>' % (size, size, pos, STAR, fill)

def btn(label, inner, w=32, h=32, r=8, color=SOFT, bg="transparent", extra=""):
    return ('<button aria-label="%s" style="%swidth:%dpx;height:%dpx;border-radius:%dpx;display:flex;align-items:center;justify-content:center;color:%s;background:%s;%s">%s</button>'
            % (label, RESET, w, h, r, color, bg, extra, inner))

def sky(x0, y0, w, h, n, seed, keep=None):
    random.seed(seed); out = []
    for _ in range(n):
        x = x0 + random.random() * w; y = y0 + random.random() * h
        if keep and not keep(x, y): continue
        r = random.choice([0.6, 0.7, 0.8, 0.9, 1.1, 1.3]); o = random.choice([0.25, 0.35, 0.45, 0.55, 0.7])
        out.append('<circle cx="%.1f" cy="%.1f" r="%s" fill="#fff" opacity="%s"/>' % (x, y, r, o))
    return "".join(out)

def sky_svg(inner):
    return '<svg width="%d" height="%d" viewBox="0 0 %d %d" style="position:absolute;left:0;top:0" aria-hidden="true">%s</svg>' % (W, H, W, H, inner)

PINS = [("Mail", ico(I["mail"], 18, extra='<rect x="3" y="5" width="18" height="14" rx="3"/>')),
        ("Calendar", ico(I["cal"], 18, extra='<rect x="4" y="5" width="16" height="15" rx="3"/>')),
        ("Notes", ico(I["note"], 18)), ("Files", ico(I["folder"], 18)), ("Maps", ico(I["map"], 18)), ("Odysseus", star(16, "{{accent}}"))]
SETTINGS_ICO = ico("M4 7h9M17 7h3M4 17h3M11 17h9", 18, extra='<circle cx="15" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>')

# ---------------- Trail rail, collapsed ----------------
def rail_collapsed(n, active, y0=214, step=44):
    cys = [y0 + i * step for i in range(n)]
    lines = '<line x1="28" y1="%d" x2="28" y2="%d" stroke="#fff" stroke-opacity=".16" stroke-width="1"/>' % (cys[0], cys[-1] + step)
    plus_cy = cys[-1] + step
    lines += '<circle cx="28" cy="%d" r="10" fill="#000" stroke="#fff" stroke-opacity=".28" stroke-dasharray="2.5 3"/>' % plus_cy
    lines += '<path d="M24.5 %d h7 M28 %d v7" stroke="#aeb7bf" stroke-width="1.4" stroke-linecap="round" transform="translate(0,-3.5)"/>' % (plus_cy, plus_cy)
    svg = '<svg width="56" height="%d" viewBox="0 0 56 %d" style="position:absolute;left:0;top:0" aria-hidden="true">%s</svg>' % (H, H, lines)
    nodes = ""
    for i, cy in enumerate(cys):
        if i == active:
            inner = '<svg width="32" height="32" viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="16" r="13" fill="#000" stroke="{{accent}}" stroke-opacity=".4"/><svg x="7" y="7" width="18" height="18" viewBox="0 0 24 24"><path d="%s" fill="{{accent}}"/></svg></svg>' % STAR
        else:
            inner = '<span style="width:7px;height:7px;border-radius:4px;background:%s"></span>' % NODE
        nodes += btn("Tab %d" % (i + 1), inner, 32, 32, 16, extra="position:absolute;left:12px;top:%dpx;" % (cy - 16))
    pins = "".join(btn(n_, s, extra="position:absolute;left:12px;top:%dpx;color:#8a949d" % (68 + 36 * i)) for i, (n_, s) in enumerate(PINS[:3]))
    logo = btn("Odyssey", star(20, "#f2f5f7"), extra="position:absolute;left:12px;top:14px;")
    bottom = btn("Settings", SETTINGS_ICO, extra="position:absolute;left:12px;top:%dpx;" % (H - 52))
    spaces = ('<div style="position:absolute;left:21px;top:%dpx;display:flex;flex-direction:column;gap:12px;align-items:center;width:14px">%s<span style="width:5px;height:5px;border-radius:3px;background:#3a424a"></span><span style="width:5px;height:5px;border-radius:3px;background:#3a424a"></span></div>' % (H - 122, star(13, "#f2f5f7")))
    return svg + logo + pins + nodes + spaces + bottom

# ---------------- Trail rail, expanded (tab tree) ----------------
def rail_expanded(rows, W_=264, y0=150, step=44):
    cys = [y0 + i * step for i in range(len(rows))]
    paths = ""; lastroot = None; roots = [i for i, r in enumerate(rows) if r[1] == 0]
    paths += '<line x1="28" y1="%d" x2="28" y2="%d" stroke="#fff" stroke-opacity=".16"/>' % (cys[0], cys[roots[-1]])
    parent = None
    for i, (t, lvl, act) in enumerate(rows):
        if lvl == 0: parent = i
        else:
            py, cy = cys[parent], cys[i]
            paths += '<path d="M28 %d V%d Q28 %d 36 %d H46" fill="none" stroke="#fff" stroke-opacity=".16"/>' % (py, cy - 8, cy, cy)
    nodes = ""; rowsh = ""
    for i, (t, lvl, act) in enumerate(rows):
        cx = 28 if lvl == 0 else 46; cy = cys[i]; tl = cx + 18
        if act:
            nodes += '<circle cx="%d" cy="%d" r="12" fill="#000" stroke="{{accent}}" stroke-opacity=".4"/><svg x="%d" y="%d" width="16" height="16" viewBox="0 0 24 24"><path d="%s" fill="{{accent}}"/></svg>' % (cx, cy, cx - 8, cy - 8, STAR)
        else:
            nodes += '<circle cx="%d" cy="%d" r="3.5" fill="%s"/>' % (cx, cy, NODE)
        bg = "background:rgba(255,255,255,.07);color:%s;" % MAIN if act else "color:%s;" % SOFT
        rowsh += ('<button style="%sposition:absolute;left:8px;top:%dpx;width:%dpx;height:36px;border-radius:8px;text-align:left;padding-left:%dpx;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;box-sizing:border-box;%s">%s</button>'
                  % (RESET, cy - 18, W_ - 16, tl - 8, bg, t))
    pcy = cys[-1] + step
    paths += '<circle cx="28" cy="%d" r="10" fill="#000" stroke="#fff" stroke-opacity=".28" stroke-dasharray="2.5 3"/><path d="M24.5 %d h7 M28 %d v7" stroke="#aeb7bf" stroke-width="1.4" stroke-linecap="round" transform="translate(0,-3.5)"/>' % (pcy, pcy, pcy)
    newb = '<button style="%sposition:absolute;left:8px;top:%dpx;width:%dpx;height:36px;border-radius:8px;text-align:left;padding-left:%dpx;font-size:14px;color:%s;box-sizing:border-box">New tab</button>' % (RESET, pcy - 18, W_ - 16, 20, MUTED)
    svg = '<svg width="%d" height="%d" viewBox="0 0 %d %d" style="position:absolute;left:0;top:0;pointer-events:none" aria-hidden="true">%s%s</svg>' % (W_, H, W_, H, paths, nodes)
    # the 'New tab' label sits right of the dashed ring
    newb = newb.replace("padding-left:20px", "padding-left:40px")
    head = (star(20, "#f2f5f7", 22, 28) + '<span style="position:absolute;left:44px;top:17px;font-size:17px;font-weight:600;letter-spacing:-.2px;color:#f2f5f7">odyssey</span>'
            + btn("Collapse trail", ico(I["side"], 18, extra='<rect x="3" y="4" width="18" height="16" rx="3.5"/>'), extra="position:absolute;left:%dpx;top:12px;" % (W_ - 44)))
    pins = "".join(btn(n_, s, extra="position:absolute;left:%dpx;top:68px;color:#8a949d" % (12 + 38 * i)) for i, (n_, s) in enumerate(PINS))
    bottom = ('<div style="position:absolute;left:20px;top:%dpx;display:flex;align-items:center;gap:10px;font-size:14px;color:%s">%s<span>Home</span><span style="width:5px;height:5px;border-radius:3px;background:#3a424a;margin-left:2px"></span><span style="width:5px;height:5px;border-radius:3px;background:#3a424a"></span></div>' % (H - 40, SOFT, star(13, "#f2f5f7"))
              + btn("Settings", SETTINGS_ICO, extra="position:absolute;left:%dpx;top:%dpx;" % (W_ - 44, H - 52)))
    return svg + head + pins + rowsh + newb + bottom

# ---------------- Helm (bottom command bar) ----------------
def helm(left, addr_html, dl_ring=None):
    ring = ""
    if dl_ring is not None:
        c = 69.1; d = c * dl_ring / 100
        ring = ('<button aria-label="Downloads, %d percent" style="%swidth:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;position:relative;color:%s">'
                '<svg width="32" height="32" viewBox="0 0 32 32" style="position:absolute;left:0;top:0" aria-hidden="true"><circle cx="16" cy="16" r="11" fill="none" stroke="#fff" stroke-opacity=".14" stroke-width="2"/><circle cx="16" cy="16" r="11" fill="none" stroke="{{accent}}" stroke-width="2" stroke-linecap="round" stroke-dasharray="%.1f %.1f" transform="rotate(-90 16 16)"/></svg>%s</button>'
                % (dl_ring, RESET, SOFT, d, c - d, ico(I["dl"], 14)))
    asst = btn("Assistant", star(15, "{{accent}}"), 36, 36, 18, bg="rgba(255,255,255,.08)")
    return ('<div style="position:absolute;left:%dpx;bottom:28px;width:560px;height:52px;box-sizing:border-box;border-radius:26px;background:#13161a;box-shadow:0 12px 32px rgba(0,0,0,.5);display:flex;align-items:center;gap:2px;padding:0 8px">'
            '%s%s<div style="flex:1;display:flex;align-items:center;gap:10px;padding:0 12px;font-size:15px;white-space:nowrap;overflow:hidden">%s</div>%s%s<span style="width:4px"></span>%s</div>'
            % (left, btn("Back", ico(I["back"], 18), 32, 32, 16), btn("Forward", ico(I["fwd"], 18), 32, 32, 16), addr_html,
               btn("Reload", ico(I["reload"], 17), 32, 32, 16), ring, asst))

def notch():
    def wc(label, d, extra=""):
        return '<button aria-label="%s" style="%swidth:42px;height:34px;display:flex;align-items:center;justify-content:center;color:%s">%s</button>' % (label, RESET, SOFT, ico(d, 15, extra=extra))
    sh = '<button aria-label="Shields, 12 blocked" style="%sheight:34px;padding:0 12px 0 8px;display:flex;align-items:center;gap:6px;font-size:13px;color:%s">%s<span>12</span></button>' % (RESET, SOFT, ico(I["shield"], 16))
    return '<div style="position:absolute;right:0;top:0;height:34px;background:#000;border-radius:0 0 0 14px;display:flex;align-items:center;padding-left:6px">%s%s%s%s</div>' % (sh, wc("Minimize", I["min"]), wc("Maximize", "", '<rect x="5" y="5" width="14" height="14" rx="2"/>'), wc("Close", I["close"]))

def page(title, body_html, props_json, logic):
    return '''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>%s</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link href="https://fonts.googleapis.com/css2?family=Familjen+Grotesk:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
<style>body{margin:0;background:#000}button:focus-visible{outline:2px solid #5ad0f0;outline-offset:2px}</style>
</helmet>
<div style="position:relative;width:%dpx;height:%dpx;background:#000;overflow:hidden;font-family:%s;color:%s">
%s
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='%s'>
%s
</script>
</body>
</html>
''' % (title, W, H, FONT, MAIN, body_html, props_json, logic)

PROPS = '{"accent":{"editor":"color","default":"#5ad0f0","options":["#5ad0f0","#6ee7b7","#b6a8ff","#f2c46d"]},"$preview":{"width":1280,"height":800}}'
LOGIC = "class Component extends DCLogic {\n  renderVals() {\n    return { accent: this.props.accent ?? '#5ad0f0' };\n  }\n}"

# ---------------- Board A: new tab = the whole window is sky; the trail is a constellation ----------------
titles = ["New tab", "Tide tables", "Lecture notes, week 9", "Train timetable", "Recipe: lentil soup", "Search: celestial navigation"]
pos = [(520, 330), (668, 236), (806, 352), (960, 262), (888, 508), (700, 560)]
edges = [(0, 1), (1, 2), (2, 3), (2, 4), (4, 5)]
cl = "".join('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#fff" stroke-opacity=".18" stroke-width="1"/>' % (pos[a] + pos[b]) for a, b in edges)
cdots = ""
for i, (x, y) in enumerate(pos):
    if i == 0:
        cdots += '<circle cx="%d" cy="%d" r="14" fill="none" stroke="{{accent}}" stroke-opacity=".4"/>' % (x, y) + star(20, "{{accent}}", x, y)
    else:
        cdots += '<circle cx="%d" cy="%d" r="%s" fill="#fff" opacity=".92"/>' % (x, y, [0, 3, 3.6, 3, 3.3, 2.8][i])
# raw svg star inside svg needs nested svg: replace helper output
cdots = cdots.replace(star(20, "{{accent}}", pos[0][0], pos[0][1]), '<svg x="%d" y="%d" width="20" height="20" viewBox="0 0 24 24"><path d="%s" fill="{{accent}}"/></svg>' % (pos[0][0] - 10, pos[0][1] - 10, STAR))
labels = "".join('<span style="position:absolute;left:%dpx;top:%dpx;font-size:13px;color:%s;white-space:nowrap">%s</span>' % (x + 16, y - 9, MAIN if i == 0 else MUTED, titles[i]) for i, (x, y) in enumerate(pos))
sky_all = sky(0, 0, W, H, 170, 21, keep=lambda x, y: not any(abs(x - px) < 24 and abs(y - py) < 24 for px, py in pos))
constel = '<svg width="%d" height="%d" viewBox="0 0 %d %d" style="position:absolute;left:0;top:0" aria-hidden="true">%s%s</svg>' % (W, H, W, H, cl, cdots)
clock = ('<div style="position:absolute;left:120px;top:92px"><div style="font-size:15px;color:%s">Good evening</div>'
         '<div style="font-size:84px;font-weight:400;letter-spacing:-3px;line-height:1.05;color:#f2f5f7;margin-top:4px">20:11</div>'
         '<div style="font-size:15px;color:%s;margin-top:2px">Wednesday, 18 November</div></div>' % (MUTED, MUTED))
hint = '<div style="position:absolute;left:120px;top:262px;font-size:13px;color:%s;width:260px;line-height:1.5">Your open tabs, drawn as a trail. Select a star to jump back to it.</div>' % MUTED
helmA = helm(56 + (W - 56 - 560) // 2, '<span style="color:%s">Ask or enter an address</span>' % MUTED)
bodyA = sky_svg(sky_all) + constel + labels + clock + hint + rail_collapsed(6, 0) + helmA + notch()
open("project/Main.dc.html", "w").write(page("Odyssey new tab", bodyA, PROPS, LOGIC))

# ---------------- Board B: reading, expanded trail, downloads ----------------
rowsB = [("Search: celestial navigation", 0, False), ("Reading the sky before maps", 1, True), ("Star chart (PDF)", 1, False), ("Tide tables", 0, False), ("Lecture notes, week 9", 0, False)]
RWB = 264
SERIF = "Georgia,'Times New Roman',serif"
PWB = W - RWB
article = ('<div style="position:absolute;left:0;top:0;right:0;height:64px;display:flex;align-items:center;justify-content:space-between;padding:0 260px 0 56px;font-family:system-ui,sans-serif">'
           '<span style="font-family:%s;font-size:20px;font-weight:700;color:#1b1f24">Northstar</span>'
           '<span style="display:flex;gap:28px;font-size:14px"><a href="#" style="color:#5b636b;text-decoration:none">Essays</a><a href="#" style="color:#5b636b;text-decoration:none">Archive</a><a href="#" style="color:#5b636b;text-decoration:none">About</a></span></div>'
           '<div style="position:absolute;left:188px;top:130px;width:640px;font-family:%s;color:#1b1f24">'
           '<h1 style="margin:0;font-size:46px;line-height:1.1;font-weight:700;letter-spacing:-.5px">Reading the sky before there were maps</h1>'
           '<div style="margin-top:18px;font-family:system-ui,sans-serif;font-size:14px;color:#5b636b">By Ines Marlowe, 8 minute read</div>'
           '<p style="margin:36px 0 0;font-size:19px;line-height:1.65">Long before charts and compasses, sailors crossed open water by memorising where stars rose and set. A single bright point near the horizon could hold a heading for hours.</p>'
           '<p style="margin:20px 0 0;font-size:19px;line-height:1.65">Navigators learned a small set of guide stars and the order they appeared through the night. As one climbed too high to be useful, the next one took over.</p>'
           '<p style="margin:20px 0 0;font-size:19px;line-height:1.65">The method was less about the stars themselves than about patience: the same sky, read the same way, night after night.</p></div>') % (SERIF, SERIF)
panelB = '<div style="position:absolute;left:%dpx;top:0;right:0;bottom:0;border-radius:16px 0 0 16px;background:#f8f8f6;overflow:hidden">%s</div>' % (RWB, article)
addrB = (ico("", 14, MUTED, extra='<rect x="5" y="11" width="14" height="9" rx="2"/><path d="%s"/>' % I["lock"]) +
         '<span><span style="color:%s">northstar.example</span><span style="color:%s">/essays/reading-the-sky</span></span>' % (MAIN, MUTED))
helmB = helm(RWB + (PWB - 560) // 2, addrB, dl_ring=62)
def dl_row(name, sub, pct, ctl):
    bar = ('<div style="height:2px;border-radius:1px;background:rgba(255,255,255,.1);margin-top:10px"><div style="height:2px;border-radius:1px;width:%d%%;background:{{accent}}"></div></div>' % pct) if pct else ""
    return ('<div style="display:flex;gap:12px;align-items:flex-start;padding:12px 14px"><div style="flex:1;min-width:0"><div style="font-size:14px;color:%s;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">%s</div><div style="font-size:12.5px;color:%s;margin-top:3px">%s</div>%s</div>%s</div>' % (MAIN, name, MUTED, sub, bar, ctl))
bx = RWB + (PWB - 560) // 2 + 560 - 300
pop = ('<div style="position:absolute;left:%dpx;bottom:92px;width:300px;border-radius:14px;background:#13161a;box-shadow:0 16px 40px rgba(0,0,0,.55);padding:6px 0">%s%s</div>'
       % (bx, dl_row("northstar-star-chart.pdf", "124 MB of 200 MB, 11.2 MB/s, 8 connections", 62, btn("Pause", ico(I["pause"], 16))),
          dl_row("tide-tables-2026.pdf", "Done", 0, '<button style="%sfont-size:13px;color:{{accent}};padding:6px 2px">Open</button>' % RESET)))
bodyB = sky_svg(sky(0, 0, RWB, H, 46, 9)) + rail_expanded(rowsB) + panelB + helmB + pop + notch()
open("project/Browsing.dc.html", "w").write(page("Odyssey browsing with downloads", bodyB, PROPS, LOGIC))

# ---------------- Board C: settings (interactive) ----------------
navs = ["General", "Appearance", "New tab", "Shields", "Odysseus", "Passwords", "Privacy", "Shortcuts", "About"]
nav_html = "".join('<div style="height:36px;padding:0 12px;border-radius:8px;display:flex;align-items:center;font-size:14px;%s">%s</div>' % (("background:rgba(255,255,255,.08);color:%s" % MAIN) if n == "Appearance" else ("color:%s" % MUTED), n) for n in navs)
def row(label, desc, ctrl):
    return ('<div style="display:flex;align-items:center;gap:32px;padding:18px 0"><div style="flex:1"><div style="font-size:15px;font-weight:500;color:%s">%s</div><div style="font-size:13px;color:%s;margin-top:4px;line-height:1.45">%s</div></div>%s</div>' % (MAIN, label, MUTED, desc, ctrl))
seg = lambda name: ('<div style="display:flex;padding:3px;border-radius:10px;background:rgba(255,255,255,.06)"><sc-for list="{{%s}}" as="o" hint-placeholder-count="3"><button onClick="{{o.pick}}" style="%s{{o.style}}">{{o.label}}</button></sc-for></div>' % (name, RESET + "height:30px;padding:0 14px;border-radius:7px;font-size:13px;"))
tog = ('<button role="switch" aria-checked="{{shootOn}}" aria-label="Shooting stars" onClick="{{toggleShoot}}" style="%swidth:42px;height:24px;border-radius:12px;position:relative;background:{{toggleBg}}"><span style="position:absolute;top:3px;width:18px;height:18px;border-radius:9px;background:#fff;{{knobPos}}"></span></button>' % RESET)
sw = ('<div style="display:flex;gap:12px"><sc-for list="{{swatches}}" as="s" hint-placeholder-count="5"><button aria-label="{{s.name}}" onClick="{{s.pick}}" style="%swidth:22px;height:22px;border-radius:11px;background:{{s.color}};{{s.ring}}"></button></sc-for></div>' % RESET)
sel = ('<div style="height:34px;min-width:140px;padding:0 12px;border-radius:10px;background:rgba(255,255,255,.06);display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:13.5px"><span>8 connections</span><span style="color:%s;display:flex">%s</span></div>' % (MUTED, ico(I["chev"], 16)))
content = ('<div style="position:absolute;left:300px;top:64px;width:560px"><h1 style="margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px">Appearance</h1>'
           '<div style="margin-top:8px;font-size:15px;color:%s">Make the browser feel like yours.</div><div style="margin-top:28px">%s%s%s%s%s</div></div>'
           % (MUTED, row("Stars", "How the sky behaves behind the browser.", seg("starsOpts")), row("Shooting stars", "An occasional comet across the new tab.", tog),
              row("Accent", "Used for the active tab, progress and focus.", sw), row("Trail", "Show the tab trail wide with titles, or narrow with stars only.", seg("trailOpts")),
              row("Faster downloads", "Big files are fetched over several connections when the site allows it.", sel)))
RWC = 56
panelC = ('<div style="position:absolute;left:%dpx;top:0;right:0;bottom:0;border-radius:16px 0 0 16px;background:#0a0c0f;overflow:hidden">'
          '<div style="position:absolute;left:44px;top:64px;width:200px;display:flex;flex-direction:column;gap:2px"><div style="font-size:15px;font-weight:600;padding:0 12px 14px">Settings</div>%s</div>%s</div>' % (RWC, nav_html, content))
helmC = helm(RWC + (W - RWC - 560) // 2, '<span style="color:%s">odyssey://</span><span>settings</span>' % MUTED)
LOGIC_C = '''class Component extends DCLogic {
  renderVals() {
    const st = this.state || {};
    const stars = st.stars ?? 'still';
    const trail = st.trail ?? 'narrow';
    const shoot = st.shoot ?? false;
    const accent = st.accent ?? (this.props.accent ?? '#5ad0f0');
    const seg = (list, cur, key) => list.map((o) => ({
      label: o.label,
      pick: () => this.setState({ [key]: o.id }),
      style: o.id === cur ? 'background:rgba(255,255,255,.14);color:#f2f5f7;' : 'background:transparent;color:#7d868f;'
    }));
    const sw = [
      { name: 'Sea', color: '#5ad0f0' }, { name: 'Mint', color: '#6ee7b7' }, { name: 'Lilac', color: '#b6a8ff' },
      { name: 'Amber', color: '#f2c46d' }, { name: 'Rose', color: '#ff8fa3' }
    ].map((s) => ({
      name: s.name, color: s.color,
      pick: () => this.setState({ accent: s.color }),
      ring: s.color === accent ? 'box-shadow:0 0 0 2px #0a0c0f,0 0 0 4px ' + s.color + ';' : ''
    }));
    return {
      accent,
      starsOpts: seg([{ id: 'off', label: 'Off' }, { id: 'still', label: 'Still' }, { id: 'drift', label: 'Drifting' }], stars, 'stars'),
      trailOpts: seg([{ id: 'narrow', label: 'Narrow' }, { id: 'wide', label: 'Wide' }], trail, 'trail'),
      swatches: sw,
      shootOn: shoot ? 'true' : 'false',
      toggleShoot: () => this.setState({ shoot: !shoot }),
      toggleBg: shoot ? accent : 'rgba(255,255,255,.14)',
      knobPos: shoot ? 'left:21px;' : 'left:3px;'
    };
  }
}'''
bodyC = sky_svg(sky(0, 0, RWC, H, 22, 4)) + rail_collapsed(4, 1) + panelC + helmC + notch()
open("project/Settings.dc.html", "w").write(page("Odyssey settings", bodyC, PROPS, LOGIC_C))

canvas = {
 "v": 3, "createdOnFiles": {"v": 1, "at": "2026-11-18T20:30:00Z"},
 "title": "Odyssey: Arc-style design sketch", "launch": {"view": "canvas"}, "pages": [],
 "boards": {
   "Main.dc.html": {"x": 0, "y": 0, "w": W, "h": H, "title": "New tab: the trail as a constellation"},
   "Browsing.dc.html": {"x": W + 80, "y": 0, "w": W, "h": H, "title": "Reading, trail open, downloads"},
   "Settings.dc.html": {"x": 2 * (W + 80), "y": 0, "w": W, "h": H, "title": "Settings", "is_interactive": True}},
 "order": ["Main.dc.html", "Browsing.dc.html", "Settings.dc.html"],
 "notes": {
   "t1": {"x": 0, "y": -300, "text": "Odyssey: its own identity", "kind": "title1", "maxW": 3 * W + 160},
   "n1": {"x": 0, "y": -230, "w": 300, "text": "The Trail. Tabs are stars joined by a line, and tabs opened from another one branch off it. Narrow shows stars only, wide shows titles.", "fill": "teal"},
   "n2": {"x": 340, "y": -230, "w": 300, "text": "The Helm. One command bar at the bottom, where your thumbs rest and the page keeps its full height. Downloads show as a ring around its icon.", "fill": "blue"},
   "n3": {"x": 680, "y": -230, "w": 300, "text": "One sky. On a new tab the sky runs edge to edge and your trail is drawn as a constellation. On a real page it retreats to the rail.", "fill": "gray"},
   "n4": {"x": 1020, "y": -230, "w": 300, "text": "Corner notch for Shields and window controls. The page is square against the window and rounded only where it meets the trail.", "fill": "orange"}
 }, "designSystems": []
}
json.dump(canvas, open("project/canvas.json", "w"), indent=1)
print("ok")
