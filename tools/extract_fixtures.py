"""Extract exact fixture data from the approved design previews (P1-04).

Writes:
  ui/fixtures/sky.json            star positions per board (the first <svg> of each preview)
  ui/fixtures/page-<Board>.html   the sample website inside the page panel (Browsing,
                                  ClosedTabs, HelmAnswer), so fixture mode can show it

Re-run only when design/preview changes (which needs the owner's approval):
  python tools/extract_fixtures.py
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PREVIEW = ROOT / 'design' / 'preview'
OUT = ROOT / 'ui' / 'fixtures'
BOARDS = ['Main', 'Browsing', 'Settings', 'HelmAnswer', 'ClosedTabs', 'Private', 'History']
PAGE_BOARDS = ['Browsing', 'ClosedTabs', 'HelmAnswer']


def attrs(tag):
    return dict(re.findall(r'([a-z-]+)="([^"]*)"', tag))


def sky(html):
    first = re.search(r'<svg[^>]*>(.*?)</svg>', html, re.S).group(1)
    stars = []
    for tag in re.findall(r'<circle[^>]*/>', first):
        a = attrs(tag)
        ring = a.get('fill') == 'none'
        stars.append({
            'x': float(a['cx']), 'y': float(a['cy']), 'r': float(a['r']),
            'o': float(a.get('stroke-opacity' if ring else 'opacity', '1')),
            **({'ring': True, 'stroke': a.get('stroke'), 'w': float(a.get('stroke-width', '1'))} if ring else
               {'fill': a.get('fill', '#fff')}),
        })
    return stars


def inner_div(html, start):
    """Inner HTML of the <div> whose opening tag starts at `start` (balanced div matching)."""
    open_end = html.index('>', start) + 1
    depth, i = 1, open_end
    for m in re.finditer(r'<(/?)div\b', html[open_end:]):
        depth += -1 if m.group(1) else 1
        if depth == 0:
            return html[open_end:open_end + m.start()]
    raise ValueError('unbalanced div')


def page_panel(html):
    m = re.search(r'<div style="position:absolute;left:\d+px;top:0;right:0;bottom:0;border-radius:16px 0 0 16px;background:#f8f8f6', html)
    if not m:
        raise ValueError('page panel not found')
    return inner_div(html, m.start())


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    data = {}
    for b in BOARDS:
        html = (PREVIEW / f'{b}.html').read_text(encoding='utf-8')
        data[b] = sky(html)
        if b in PAGE_BOARDS:
            (OUT / f'page-{b}.html').write_text(page_panel(html) + '\n', encoding='utf-8', newline='\n')
    (OUT / 'sky.json').write_text(json.dumps(data, separators=(',', ':')) + '\n', encoding='utf-8', newline='\n')
    print('stars per board:', {b: len(s) for b, s in data.items()})
    print('pages:', ', '.join(f'page-{b}.html' for b in PAGE_BOARDS))


if __name__ == '__main__':
    main()
