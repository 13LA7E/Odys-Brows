exec(open("gen2.py").read())   # reuse helpers, article, panelB, rail_collapsed etc. (rewrites the first three boards identically)

def root_bg(html, bg):
    return html.replace("background:#000;overflow:hidden;font-family", "background:%s;overflow:hidden;font-family" % bg, 1)

PANEL_L = 56
PAGE_W = W - PANEL_L
HX = PANEL_L + (PAGE_W - 560) // 2          # helm left for a collapsed rail

# ================= D: Helm answers =================
panelD = ('<div style="position:absolute;left:%dpx;top:0;right:0;bottom:0;border-radius:16px 0 0 16px;background:#f8f8f6;overflow:hidden">%s<div style="position:absolute;left:0;top:0;right:0;bottom:0;background:rgba(0,0,0,.5)"></div></div>' % (PANEL_L, article))
typed = ('<div style="position:absolute;left:%dpx;bottom:28px;width:560px;height:52px;box-sizing:border-box;border-radius:26px;background:#13161a;box-shadow:0 12px 32px rgba(0,0,0,.5);display:flex;align-items:center;gap:12px;padding:0 8px 0 18px">'
         '%s<div style="flex:1;font-size:15px;color:%s;display:flex;align-items:center;gap:2px">What did navigators use when the guide star was hidden?<span style="width:1.5px;height:19px;background:{{accent}}"></span></div>'
         '<span style="font-size:12px;color:%s">Enter</span><button aria-label="Ask" style="%swidth:36px;height:36px;border-radius:18px;background:{{accent}};color:#00131b;display:flex;align-items:center;justify-content:center">%s</button></div>'
         % (HX, star(16, "{{accent}}"), MAIN, MUTED, RESET, ico("M12 19V6m0 0l-5 5m5-5l5 5", 18, sw=2)))
mini = ('<svg width="76" height="30" viewBox="0 0 76 30" aria-hidden="true"><line x1="8" y1="21" x2="36" y2="8" stroke="#fff" stroke-opacity=".25"/><line x1="36" y1="8" x2="66" y2="20" stroke="#fff" stroke-opacity=".25"/>'
        '<circle cx="8" cy="21" r="3" fill="#fff" opacity=".9"/><circle cx="36" cy="8" r="3.4" fill="#fff" opacity=".9"/><circle cx="66" cy="20" r="2.6" fill="#fff" opacity=".9"/></svg>')
chip = lambda t: '<span style="height:26px;padding:0 10px;border-radius:8px;background:rgba(255,255,255,.06);display:inline-flex;align-items:center;font-size:12.5px;color:%s">%s</span>' % (SOFT, t)
card = ('<div style="position:absolute;left:%dpx;bottom:90px;width:560px;box-sizing:border-box;border-radius:20px;background:#13161a;box-shadow:0 16px 40px rgba(0,0,0,.55);padding:22px 24px 16px">'
        '<div style="font-size:15.5px;line-height:1.6;color:%s">They fell back on other guide stars that rose or set near the same point on the horizon, and on swell direction and wind. The essay describes it as reading the same sky in a different order.</div>'
        '<div style="display:flex;align-items:center;gap:10px;margin-top:18px">%s%s%s</div>'
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-top:18px;font-size:12.5px"><span style="color:%s">Answered by your Odysseus server</span>'
        '<button style="%scolor:{{accent}};font-size:13px;padding:4px 0">Continue in Odysseus</button></div></div>'
        % (HX, MAIN, mini, chip("northstar.example"), chip("Star chart (PDF)"), MUTED, RESET))
bodyD = sky_svg(sky(0, 0, 56, H, 14, 6)) + rail_collapsed(5, 1) + panelD + card + typed + notch()
open("project/HelmAnswer.dc.html", "w").write(page("Odyssey helm answer", bodyD, PROPS, LOGIC))

# ================= E: closed tabs fade out of the trail =================
def rail_faded(open_rows, faded_titles, hover, W_=264, y0=150, step=44):
    cys = [y0 + i * step for i in range(len(open_rows))]
    newcy = cys[-1] + step
    fy0 = newcy + step
    fcys = [fy0 + j * step for j in range(len(faded_titles))]
    paths = '<line x1="28" y1="%d" x2="28" y2="%d" stroke="#fff" stroke-opacity=".16"/>' % (cys[0], newcy)
    paths += '<circle cx="28" cy="%d" r="10" fill="#000" stroke="#fff" stroke-opacity=".28" stroke-dasharray="2.5 3"/><path d="M24.5 %d h7 M28 %d v7" stroke="#aeb7bf" stroke-width="1.4" stroke-linecap="round" transform="translate(0,-3.5)"/>' % (newcy, newcy, newcy)
    paths += '<line x1="28" y1="%d" x2="28" y2="%d" stroke="#fff" stroke-opacity=".12" stroke-dasharray="2 5"/>' % (fcys[0], fcys[-1])
    parent = None
    for i, (t, lvl, act) in enumerate(open_rows):
        if lvl == 0: parent = i
        else:
            py, cy = cys[parent], cys[i]
            paths += '<path d="M28 %d V%d Q28 %d 36 %d H46" fill="none" stroke="#fff" stroke-opacity=".16"/>' % (py, cy - 8, cy, cy)
    nodes = ""; btns = ""
    for i, (t, lvl, act) in enumerate(open_rows):
        cx = 28 if lvl == 0 else 46; cy = cys[i]
        if act:
            nodes += '<circle cx="%d" cy="%d" r="12" fill="#000" stroke="{{accent}}" stroke-opacity=".4"/><svg x="%d" y="%d" width="16" height="16" viewBox="0 0 24 24"><path d="%s" fill="{{accent}}"/></svg>' % (cx, cy, cx - 8, cy - 8, STAR)
        else:
            nodes += '<circle cx="%d" cy="%d" r="3.5" fill="%s"/>' % (cx, cy, NODE)
        bg = "background:rgba(255,255,255,.07);color:%s;" % MAIN if act else "color:%s;" % SOFT
        btns += '<button style="%sposition:absolute;left:8px;top:%dpx;width:%dpx;height:36px;border-radius:8px;text-align:left;padding-left:%dpx;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;box-sizing:border-box;%s">%s</button>' % (RESET, cy - 18, W_ - 16, cx + 10, bg, t)
    btns += '<button style="%sposition:absolute;left:8px;top:%dpx;width:%dpx;height:36px;border-radius:8px;text-align:left;padding-left:40px;font-size:14px;color:%s;box-sizing:border-box">New tab</button>' % (RESET, newcy - 18, W_ - 16, MUTED)
    label = '<span style="position:absolute;left:48px;top:%dpx;font-size:12.5px;color:%s">Closed earlier</span>' % (newcy + step // 2 - 22 + 6, MUTED)
    ops = [0.4, 0.27, 0.16]
    for j, t in enumerate(faded_titles):
        cy = fcys[j]
        if j == hover:
            nodes += '<circle cx="28" cy="%d" r="11" fill="#000" stroke="{{accent}}" stroke-opacity=".5"/><circle cx="28" cy="%d" r="4" fill="{{accent}}"/>' % (cy, cy)
            bg = "background:rgba(255,255,255,.05);color:%s;" % MAIN
        else:
            nodes += '<circle cx="28" cy="%d" r="3.5" fill="#fff" opacity="%s"/>' % (cy, ops[j])
            bg = "color:%s;" % "#6b757e"
        btns += '<button style="%sposition:absolute;left:8px;top:%dpx;width:%dpx;height:36px;border-radius:8px;text-align:left;padding-left:38px;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;box-sizing:border-box;%s">%s</button>' % (RESET, cy - 18, W_ - 16, bg, t)
    svg = '<svg width="%d" height="%d" viewBox="0 0 %d %d" style="position:absolute;left:0;top:0;pointer-events:none" aria-hidden="true">%s%s</svg>' % (W_, H, W_, H, paths, nodes)
    head = (star(20, "#f2f5f7", 22, 28) + '<span style="position:absolute;left:44px;top:17px;font-size:17px;font-weight:600;letter-spacing:-.2px;color:#f2f5f7">odyssey</span>'
            + btn("Collapse trail", ico(I["side"], 18, extra='<rect x="3" y="4" width="18" height="16" rx="3.5"/>'), extra="position:absolute;left:%dpx;top:12px;" % (W_ - 44)))
    pins = "".join(btn(n_, s, extra="position:absolute;left:%dpx;top:68px;color:#8a949d" % (12 + 38 * i)) for i, (n_, s) in enumerate(PINS))
    bottom = ('<div style="position:absolute;left:20px;top:%dpx;display:flex;align-items:center;gap:10px;font-size:14px;color:%s">%s<span>Home</span><span style="width:5px;height:5px;border-radius:3px;background:#3a424a;margin-left:2px"></span><span style="width:5px;height:5px;border-radius:3px;background:#3a424a"></span></div>' % (H - 40, SOFT, star(13, "#f2f5f7"))
              + btn("Settings", SETTINGS_ICO, extra="position:absolute;left:%dpx;top:%dpx;" % (W_ - 44, H - 52)))
    return svg + head + pins + btns + label + bottom, fcys

openE = [("Search: celestial navigation", 0, False), ("Reading the sky before maps", 1, True), ("Star chart (PDF)", 1, False), ("Lecture notes, week 9", 0, False)]
railE, fc = rail_faded(openE, ["Train timetable", "Tide tables", "Recipe: lentil soup"], hover=1)
cy = fc[1]
thumb = ('<div style="height:112px;border-radius:14px 14px 0 0;background:#dfe3e6;position:relative;overflow:hidden">'
         '<div style="position:absolute;left:18px;top:18px;width:96px;height:10px;border-radius:5px;background:#b9c0c6"></div>'
         '<div style="position:absolute;left:18px;top:42px;width:240px;height:8px;border-radius:4px;background:#cbd1d6"></div><div style="position:absolute;left:18px;top:58px;width:210px;height:8px;border-radius:4px;background:#cbd1d6"></div>'
         '<div style="position:absolute;left:18px;top:74px;width:150px;height:8px;border-radius:4px;background:#cbd1d6"></div></div>')
prev = ('<div style="position:absolute;left:276px;top:%dpx;width:296px;border-radius:14px;background:#13161a;box-shadow:0 16px 40px rgba(0,0,0,.55);overflow:hidden">%s'
        '<div style="padding:14px 16px 12px"><div style="font-size:15px;font-weight:500;color:%s">Tide tables</div><div style="font-size:12.5px;color:%s;margin-top:3px">tides.example</div>'
        '<div style="font-size:12.5px;color:%s;margin-top:10px">Closed 12 minutes ago</div>'
        '<div style="display:flex;gap:18px;margin-top:12px"><button style="%scolor:{{accent}};font-size:13.5px;font-weight:500">Relight</button><button style="%scolor:%s;font-size:13.5px">Forget</button></div></div></div>'
        % (cy - 76, thumb, MAIN, MUTED, MUTED, RESET, RESET, SOFT))
helmE = helm(RWB + (PWB - 560) // 2, addrB)
bodyE = sky_svg(sky(0, 0, RWB, H, 46, 13)) + railE + panelB + helmE + prev + notch()
open("project/ClosedTabs.dc.html", "w").write(page("Odyssey closed tabs", bodyE, PROPS, LOGIC))

# ================= F: private voyage =================
random.seed(31); pv = []
for _ in range(120):
    x = random.random() * W; y = random.random() * H
    if random.random() < 0.4:
        pv.append('<circle cx="%.1f" cy="%.1f" r="%s" fill="none" stroke="#cdd9ee" stroke-opacity="%s" stroke-width="1"/>' % (x, y, random.choice([1.6, 2.0, 2.6]), random.choice([.25, .35, .5])))
    else:
        pv.append('<circle cx="%.1f" cy="%.1f" r="%s" fill="#cdd9ee" opacity="%s"/>' % (x, y, random.choice([.6, .8, 1.0]), random.choice([.2, .3, .4])))
STAR_O = lambda s, col, x=None, y=None: ('<svg width="%d" height="%d" viewBox="0 0 24 24"%s aria-hidden="true"><path d="%s" fill="none" stroke="%s" stroke-width="1.6" stroke-linejoin="round"/></svg>' % (s, s, "" if x is None else ' style="position:absolute;left:%spx;top:%spx"' % (x - s / 2, y - s / 2), STAR, col))
def rail_private(n=2, active=0, y0=214, step=44):
    cys = [y0 + i * step for i in range(n)]
    svg = ('<svg width="56" height="%d" viewBox="0 0 56 %d" style="position:absolute;left:0;top:0" aria-hidden="true"><line x1="28" y1="%d" x2="28" y2="%d" stroke="#cdd9ee" stroke-opacity=".25" stroke-dasharray="3 4"/>' % (H, H, cys[0], cys[-1] + step))
    pc = cys[-1] + step
    svg += '<circle cx="28" cy="%d" r="10" fill="#02040a" stroke="#cdd9ee" stroke-opacity=".3" stroke-dasharray="2.5 3"/><path d="M24.5 %d h7 M28 %d v7" stroke="#aeb7bf" stroke-width="1.4" stroke-linecap="round" transform="translate(0,-3.5)"/>' % (pc, pc, pc)
    for i, cy in enumerate(cys):
        if i == active:
            svg += '<circle cx="28" cy="%d" r="13" fill="#02040a" stroke="{{accent}}" stroke-opacity=".45"/><svg x="19" y="%d" width="18" height="18" viewBox="0 0 24 24"><path d="%s" fill="none" stroke="{{accent}}" stroke-width="1.8" stroke-linejoin="round"/></svg>' % (cy, cy - 9, STAR)
        else:
            svg += '<circle cx="28" cy="%d" r="4" fill="none" stroke="%s" stroke-width="1.4"/>' % (cy, "#6b7a90")
    svg += '</svg>'
    moon = '<svg width="20" height="20" viewBox="0 0 24 24" style="position:absolute;left:18px;top:70px" fill="none" stroke="{{accent}}" stroke-width="1.7" stroke-linejoin="round" aria-hidden="true"><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/></svg>'
    logo = btn("Odyssey", STAR_O(20, "#cdd9ee"), extra="position:absolute;left:12px;top:14px;")
    bottom = btn("Settings", SETTINGS_ICO, extra="position:absolute;left:12px;top:%dpx;" % (H - 52))
    return svg + logo + moon + bottom
bullet = lambda t: ('<div style="display:flex;align-items:center;gap:12px;font-size:15px;color:%s;height:34px"><svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="{{accent}}" stroke-width="1.3"/></svg>%s</div>' % (SOFT, t))
copyF = ('<div style="position:absolute;left:150px;top:214px;width:520px"><div style="font-size:46px;font-weight:600;letter-spacing:-1px;line-height:1.1;color:#f2f5f7">Private voyage</div>'
         '<div style="font-size:16px;line-height:1.6;color:%s;margin-top:18px">This window leaves no trail behind. Nothing you open here is kept once you close it.</div>'
         '<div style="margin-top:28px">%s%s%s</div></div>' % (MUTED, bullet("History is not saved"), bullet("Cookies and site data are cleared on close"), bullet("Shields stay on")))
helmF = helm(HX, '<span style="color:%s">Ask or enter an address</span>' % MUTED)
bodyF = sky_svg("".join(pv)) + copyF + rail_private() + helmF + notch()
htmlF = root_bg(page("Odyssey private window", bodyF, PROPS.replace("#5ad0f0", "#b6a8ff").replace('"options":["#b6a8ff","#6ee7b7","#b6a8ff","#f2c46d"]', '"options":["#b6a8ff","#5ad0f0","#6ee7b7","#f2c46d"]'), LOGIC.replace("#5ad0f0", "#b6a8ff")), "#02040a")
htmlF = htmlF.replace("outline:2px solid #5ad0f0", "outline:2px solid #b6a8ff").replace("body{margin:0;background:#000}", "body{margin:0;background:#02040a}")
open("project/Private.dc.html", "w").write(htmlF)

# ================= G: history as a trail =================
RX = 120; TIMEW = 52; CX = RX + TIMEW + 14; TL = CX + 30
def hist_block(top, entries):
    rows = ""; svg = ""; ys = [top + 22 + i * 44 for i in range(len(entries))]
    roots = [i for i, e in enumerate(entries) if e[3] == 0]
    svg += '<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#fff" stroke-opacity=".16"/>' % (CX, ys[roots[0]], CX, ys[roots[-1]])
    parent_for = {}
    for i, e in enumerate(entries):
        if e[3] == 1:
            p = next(j for j in range(i + 1, len(entries)) if entries[j][3] == 0)
            svg += '<path d="M%d %d V%d Q%d %d %d %d H%d" fill="none" stroke="#fff" stroke-opacity=".16"/>' % (CX, ys[p], ys[i] + 8, CX, ys[i], CX + 8, ys[i], CX + 20)
    for i, (tm, title, host, lvl, kind) in enumerate(entries):
        cx = CX if lvl == 0 else CX + 20; cy = ys[i]; tx = TL if lvl == 0 else TL + 20
        if kind == "now":
            svg += '<circle cx="%d" cy="%d" r="11" fill="#0a0c0f" stroke="{{accent}}" stroke-opacity=".4"/><svg x="%d" y="%d" width="14" height="14" viewBox="0 0 24 24"><path d="%s" fill="{{accent}}"/></svg>' % (cx, cy, cx - 7, cy - 7, STAR)
        elif kind == "again":
            svg += '<circle cx="%d" cy="%d" r="5" fill="#fff" opacity=".95"/>' % (cx, cy)
        else:
            svg += '<circle cx="%d" cy="%d" r="3.4" fill="%s"/>' % (cx, cy, NODE)
        hov = kind == "hover"
        if hov:
            svg += '<circle cx="%d" cy="%d" r="3.4" fill="#fff"/>' % (cx, cy)
        rowbg = "background:rgba(255,255,255,.05);" if hov else ""
        acts = ('<span style="margin-left:auto;display:flex;gap:16px;font-size:13px"><span style="color:{{accent}}">Open</span><span style="color:%s">Forget</span></span>' % SOFT) if hov else ""
        rows += ('<div style="position:absolute;left:%dpx;top:%dpx;width:660px;height:38px;border-radius:8px;%sdisplay:flex;align-items:center;font-size:14.5px;box-sizing:border-box">'
                 '<span style="width:%dpx;font-size:13px;color:%s;font-variant-numeric:tabular-nums">%s</span><span style="margin-left:%dpx;color:%s;white-space:nowrap">%s</span><span style="margin-left:10px;font-size:13px;color:%s;white-space:nowrap">%s</span>%s<span style="width:12px"></span></div>'
                 % (RX - 12, cy - 19, rowbg, TIMEW + 12, MUTED, tm, (TL - (RX - 12) - (TIMEW + 12)) + (20 if lvl else 0), MAIN if kind in ("now", "hover", "again") else SOFT, title, MUTED, host, acts))
    return svg, rows
t_entries = [("20:04", "Reading the sky before maps", "northstar.example", 1, "now"), ("19:58", "Search: celestial navigation", "search.example", 0, "n"),
             ("19:41", "Tide tables", "tides.example", 0, "hover"), ("19:22", "Lecture notes, week 9", "campus.example", 0, "n")]
y_entries = [("22:10", "Train timetable", "rail.example", 0, "n"), ("21:47", "Recipe: lentil soup", "cooking.example", 0, "n"),
             ("21:30", "Orbital mechanics, lecture 4", "campus.example", 0, "n"), ("20:55", "Tide tables", "tides.example", 0, "again")]
s1, r1 = hist_block(240, t_entries); s2, r2 = hist_block(480, y_entries)
grp = lambda top, t, act: '<div style="position:absolute;left:%dpx;top:%dpx;width:660px;display:flex;justify-content:space-between;align-items:baseline"><span style="font-size:15px;font-weight:600">%s</span><span style="font-size:13px;color:%s">%s</span></div>' % (RX, top, t, MUTED, act)
srch = ('<div style="position:absolute;left:%dpx;top:118px;width:660px;height:46px;border-radius:12px;background:rgba(255,255,255,.06);display:flex;align-items:center;gap:12px;padding:0 16px;box-sizing:border-box;font-size:15px;color:%s">%s<span>Search your trail</span></div>'
        % (RX, MUTED, ico("", 18, MUTED, extra='<circle cx="11" cy="11" r="6"/><path d="M16 16l4 4"/>')))
mm = [(40, 120), (96, 60), (150, 104), (206, 44), (254, 92)]
mme = [(0, 1), (1, 2), (2, 3), (2, 4)]
mmsvg = ('<svg width="300" height="170" viewBox="0 0 300 170" aria-hidden="true">%s%s</svg>' %
         ("".join('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#fff" stroke-opacity=".18"/>' % (mm[a] + mm[b]) for a, b in mme),
          "".join('<circle cx="%d" cy="%d" r="%s" fill="#fff" opacity=".9"/>' % (x, y, r) for (x, y), r in zip(mm, [3, 3, 4.6, 3, 3]))
          + '<circle cx="40" cy="120" r="12" fill="none" stroke="{{accent}}" stroke-opacity=".4"/><svg x="30" y="110" width="20" height="20" viewBox="0 0 24 24"><path d="%s" fill="{{accent}}"/></svg>' % STAR))
side = ('<div style="position:absolute;left:880px;top:236px;width:300px"><div style="font-size:15px;font-weight:600">Today</div><div style="font-size:13px;color:%s;margin-top:4px;line-height:1.5">Four pages, one branch. The bright star is the page you came back to.</div><div style="margin-top:18px">%s</div></div>' % (MUTED, mmsvg))
panelG = ('<div style="position:absolute;left:%dpx;top:0;right:0;bottom:0;border-radius:16px 0 0 16px;background:#0a0c0f;overflow:hidden">'
          '<svg width="%d" height="%d" viewBox="0 0 %d %d" style="position:absolute;left:0;top:0" aria-hidden="true">%s</svg>'
          '<div style="position:absolute;left:%dpx;top:56px;font-size:32px;font-weight:600;letter-spacing:-.5px">History</div>%s%s%s%s%s%s%s</div>'
          % (PANEL_L, PAGE_W, H, PAGE_W, H, "", RX, srch,
             grp(206, "Today", "Clear this day"), '<svg width="%d" height="%d" viewBox="0 0 %d %d" style="position:absolute;left:0;top:0" aria-hidden="true">%s%s</svg>' % (PAGE_W, H, PAGE_W, H, s1, s2),
             grp(446, "Yesterday", "Clear this day"), r1, r2, side))
helmG = helm(HX, '<span style="color:%s">odyssey://</span><span>history</span>' % MUTED)
bodyG = sky_svg(sky(0, 0, 56, H, 14, 8)) + rail_collapsed(5, 4) + panelG + helmG + notch()
open("project/History.dc.html", "w").write(page("Odyssey history", bodyG, PROPS, LOGIC))

# ================= canvas =================
c = json.load(open("project/canvas.json"))
Y2 = 1100; GAP = W + 80
new = [("HelmAnswer.dc.html", "Helm: ask Odysseus, answer above the bar"), ("ClosedTabs.dc.html", "Closed tabs fade, then relight"),
       ("Private.dc.html", "Private voyage"), ("History.dc.html", "History drawn as a trail")]
for i, (f, t) in enumerate(new):
    c["boards"][f] = {"x": i * GAP, "y": Y2, "w": W, "h": H, "title": t}
    if f not in c["order"]: c["order"].append(f)
c["notes"]["t2"] = {"x": 0, "y": 850, "text": "Next: four more screens", "kind": "title1", "maxW": 4 * W + 240}
labels2 = [("Helm answers. The bar grows into the question, and the answer sits right above it with its sources drawn as linked stars. It names where the answer came from.", "blue"),
           ("Closed tabs dim into faint stars below the open trail. Hover for a preview, then relight to bring the page back. They fade out after a day.", "teal"),
           ("Private. Stars become hollow rings, the trail line turns dashed and the accent shifts to lilac, so the mode is visible without a banner.", "purple"),
           ("History keeps the trail shape. Pages opened from another branch off it, and a page you returned to shows as a brighter star.", "gray")]
for i, (t, f) in enumerate(labels2):
    c["notes"]["m%d" % (i + 1)] = {"x": i * GAP, "y": Y2 + H + 50, "w": 320, "text": t, "fill": f}
json.dump(c, open("project/canvas.json", "w"), indent=1)
print("ok")
