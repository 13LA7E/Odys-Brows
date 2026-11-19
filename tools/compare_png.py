"""Compare two screenshots and report how different they are.

Usage:
  python tools/compare_png.py EXPECTED.png ACTUAL.png [--out diff.png]
                              [--region x,y,w,h] [--threshold 5.0]

How the score works (tuned on the design boards, 19 Nov 2026):
- Both images are scaled to the 1280x800 design size and smoothed slightly, so
  anti-aliasing and font-rendering differences between Windows and Linux don't count.
- A pixel is "wrong" when a colour channel still differs by more than 48 (of 255).
- The score is wrong pixels as a percentage of the *drawn* pixels (everything that
  isn't the board's plain background), so mostly-dark screens are judged fairly.
- Calibration: identical = 0, slight blur = 0, 1 px shift = ~1, 2 px shift = ~8,
  a different board = 22 or more. Default pass limit: 5.
- --region x,y,w,h (in 1280x800 coordinates) compares only that part.
- Exit code 0 on pass, 1 on fail.

The diff image shows the expected screen dimmed, with wrong pixels in red.
A good score does not prove the screen is right: also look at it.
Needs: pip install pillow
"""
import argparse
import sys

from PIL import Image, ImageChops, ImageFilter

DESIGN_W, DESIGN_H = 1280, 800
BLUR = 1.5
TOLERANCE = 48
INK = 24


def _maxch(img):
    r, g, b = img.split()
    return ImageChops.lighter(ImageChops.lighter(r, g), b)


def _ink(img):
    """Pixels that differ from the image's most common colour (its background)."""
    small = img.resize((img.size[0] // 4 or 1, img.size[1] // 4 or 1))
    bg = max(small.getcolors(1 << 24), key=lambda c: c[0])[1]
    return _maxch(ImageChops.difference(img, Image.new("RGB", img.size, bg))).point(
        lambda v: 255 if v > INK else 0)


def _prep(path):
    img = Image.open(path).convert("RGB")
    if img.size != (DESIGN_W, DESIGN_H):
        img = img.resize((DESIGN_W, DESIGN_H), Image.LANCZOS)
    return img


def compare(expected_path, actual_path, region=None, diff_out=None):
    """Return (score, raw_percent). score: % of drawn pixels that are wrong."""
    exp, act = _prep(expected_path), _prep(actual_path)
    if region:
        x, y, w, h = (int(float(v)) for v in str(region).split(","))
        box = (x, y, x + w, y + h)
        exp, act = exp.crop(box), act.crop(box)
    e_s, a_s = exp.filter(ImageFilter.GaussianBlur(BLUR)), act.filter(ImageFilter.GaussianBlur(BLUR))
    wrong = _maxch(ImageChops.difference(e_s, a_s)).point(lambda v: 255 if v > TOLERANCE else 0)
    drawn = ImageChops.lighter(_ink(e_s), _ink(a_s))
    n_wrong = wrong.histogram()[255]
    n_drawn = drawn.histogram()[255]
    total = exp.size[0] * exp.size[1]
    score = n_wrong / max(n_drawn, 1) * 100
    raw = n_wrong / total * 100
    if diff_out:
        base = Image.blend(exp, Image.new("RGB", exp.size, (0, 0, 0)), 0.6)
        Image.composite(Image.new("RGB", exp.size, (255, 40, 40)), base, wrong).save(diff_out)
    return score, raw


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("expected")
    ap.add_argument("actual")
    ap.add_argument("--out", help="write a diff image here")
    ap.add_argument("--region", help="x,y,w,h in 1280x800 design coordinates")
    ap.add_argument("--threshold", type=float, default=5.0, help="pass limit for the score (default 5)")
    a = ap.parse_args(argv)
    score, raw = compare(a.expected, a.actual, a.region, a.out)
    ok = score <= a.threshold
    print("%s  score %.1f (limit %.1f; %% of drawn pixels that differ), %.2f%% of all pixels%s"
          % ("PASS" if ok else "FAIL", score, a.threshold, raw, ", diff -> " + a.out if a.out else ""))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
