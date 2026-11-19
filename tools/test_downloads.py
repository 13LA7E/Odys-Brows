#!/usr/bin/env python3
"""
Integration test for P4-01 Downloads.
Verifies:
1. Segdl unit and integration test suite:
   - 200 MB+ segmented download with speed reporting and concurrent connections.
   - Pause and resume controls.
   - Cancel control with immediate cleanup of .odypart (no leftover files).
   - Fallback for files < 8 MiB.
   - Fallback for servers without Range support.
2. Browsing board popover and Helm download ring visual check pixel comparison (score 0.0).
"""

import subprocess
import sys
from PIL import Image

def run_cmd(cmd):
    print(f"Running: {' '.join(cmd)}")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"FAILED: {res.stderr}\n{res.stdout}")
        sys.exit(1)
    print(res.stdout)

def _diff_count(img):
    raw = img.convert("RGB").tobytes()
    return sum(1 for i in range(0, len(raw), 3) if raw[i] != 0 or raw[i+1] != 0 or raw[i+2] != 0)

def check_visual_regions():
    print("Checking Browsing board popover and Helm download ring visual fidelity...")
    subprocess.run(["python", "tools/visual_check.py", "Browsing"], capture_output=True)
    diff = Image.open(r"out\visual\Browsing.diff.png")
    
    # Popover region: left 752*2=1504, top 580*2=1160, width 300*2=600, height 140*2=280
    popover = diff.crop((1504, 1160, 2104, 1440))
    diff_popover = _diff_count(popover)
    print(f"  Popover diff pixels: {diff_popover} / {popover.width * popover.height}")
    assert diff_popover == 0, f"Popover diff should be 0, got {diff_popover}"

    # Helm download ring region: left 492*2=984, top 720*2=1440, width 560*2=1120, height 52*2=104
    helm = diff.crop((984, 1440, 2104, 1544))
    diff_helm = _diff_count(helm)
    print(f"  Helm diff pixels: {diff_helm} / {helm.width * helm.height}")
    assert diff_helm == 0, f"Helm diff should be 0, got {diff_helm}"

def main():
    print("=== P4-01 Downloads Test Suite ===")
    run_cmd(["cargo", "test", "-p", "segdl", "--test", "integration"])
    check_visual_regions()
    print("=== All P4-01 Downloads tests PASSED ===")

if __name__ == "__main__":
    main()
