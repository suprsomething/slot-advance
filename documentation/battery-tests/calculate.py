#!/usr/bin/env python3
"""Recalculate every battery figure in documentation/sleep-and-battery.md from the raw reports.

Run from the repository root:  python3 documentation/battery-tests/calculate.py

All currents are ESTIMATES derived from the RG SP (2026)'s battery gauge. The gauge only
reports whole percent steps; on this 3,500 mAh battery one step is 35 mAh. No current was
measured with electrical test equipment.
"""
import re
from pathlib import Path

HERE = Path(__file__).parent
STEP_MAH = 35.0  # 1% of the 3,500 mAh design capacity (charge_counter moves in 35,000 uAh steps)


def read(name):
    return (HERE / name).read_text()


def num(pattern, text, group=1, cast=float):
    m = re.search(pattern, text, re.M)
    if not m:
        raise SystemExit(f"pattern not found: {pattern}")
    return cast(m.group(group))


print("=" * 72)
print("Test B: 0.3.0 software standby, test build held awake (lid closed, 2 h)")
b = read("test-b-software-standby.txt")
interval = num(r"Whole test interval \(uptime\): ([\d.]+) s", b)
p0, p1 = (int(x) for x in re.search(r"capacity: (\d+)% -> (\d+)%", b).groups())
end_ma = (p0 - p1) * STEP_MAH / (interval / 3600)
end_err = STEP_MAH / (interval / 3600)
print(f"  battery {p0}% -> {p1}% over {interval:.0f} s: {p0 - p1} points, {(p0 - p1) / (interval / 3600):.1f} %/h observed")
print(f"  from start and end readings: {end_ma:.0f} +/- {end_err:.1f} mA")
# Tick timing: the minute log shows when each 1% step happened, to within one minute.
rows = [r.split(", ") for r in re.findall(r"^\d+, [\d.]+, \d+, \d+, \d+, .*$", b, re.M)]
ticks = []
for prev, row in zip(rows, rows[1:]):
    if row[4] != prev[4]:
        ticks.append((float(prev[1]), float(row[1])))  # the tick lies between these uptimes
first, last = ticks[0], ticks[-1]
steps = len(ticks) - 1
lo = steps * STEP_MAH / ((last[1] - first[0]) / 3600)
hi = steps * STEP_MAH / ((last[0] - first[1]) / 3600)
print(f"  from tick timing ({steps} full steps between the first and last tick): {lo:.0f} to {hi:.0f} mA")
B = 250.0
print(f"  figure used: ~{B:.0f} mA")

print("=" * 72)
print("Test A: first suspend-to-RAM, lid wake, USB and Wi-Fi on (lid closed, 2 h)")
a = read("test-a-suspend-usb-wifi-on.txt")
interval = num(r"Whole test interval \(uptime\): ([\d.]+) s", a)
p0, p1 = (int(x) for x in re.search(r"capacity: (\d+)% -> (\d+)%", a).groups())
a_ma = (p0 - p1) * STEP_MAH / (interval / 3600)
a_err = STEP_MAH / (interval / 3600)
print(f"  battery {p0}% -> {p1}% over {interval:.0f} s: {(p0 - p1) / (interval / 3600):.1f} %/h observed")
print(f"  estimate: {a_ma:.0f} +/- {a_err:.1f} mA ({a_ma - a_err:.1f} to {a_ma + a_err:.1f})")
print("  (the report itself prints +/- 2.5 mA; that assumed 5 mAh gauge steps and is wrong)")

print("=" * 72)
print("Experiment 5: four 90-minute suspends, one change at a time")
e = read("experiment-5-suspend-segments.txt")
segs = re.findall(
    r"Segment (\d): (.+)\n.*\n  start tick ([\d.]+) s, close tick ([\d.]+) s, next tick ([\d.]+) s\n"
    r"  whole interval \d+ s \([\d.]+ h\): suspended ([\d.]+) s, awake [\d.]+ s\n  charge: (\d+) step",
    e,
)
results = {}
for k, name, s0, t1, t2, su, n in segs:
    s0, t1, t2, su, n = float(s0), float(t1), float(t2), float(su), int(n)
    awake = (t1 - s0) - su          # seconds awake inside the segment
    step_s = t2 - t1                # seconds for one step while awake, just after the segment
    raw_awake = STEP_MAH / (step_s / 3600)
    raw = (n * STEP_MAH - raw_awake * awake / 3600) / (su / 3600)
    # Gauge-independent form: suspended/awake current ratio, scaled by an assumed awake current.
    ratio = (n * step_s - awake) / su
    corr = {ia: ratio * ia for ia in (240, 250, 260)}
    results[int(k)] = (raw, corr)
    print(f"  {k}. {name}")
    print(f"     raw (35 mAh steps): {raw:.1f} mA;  assuming 240/250/260 mA awake: "
          + " / ".join(f"{v:.1f}" for v in corr.values()) + " mA"
          + f";  implied step {250 * step_s / 3600:.1f} mAh")

seg3 = results[3]
print("  Segment 3 is the 0.4.0 configuration. Its battery fell 77% -> 74% across the 1.5 h asleep:")
print(f"     observed {3 / 1.5:.1f} %/h (whole steps, so anything from {2 / 1.5:.1f} to {4 / 1.5:.1f} %/h is consistent)")
S = 70.0
print(f"  figure used: ~{S:.0f} mA (raw {seg3[0]:.1f}; corrected {min(seg3[1].values()):.1f} to {max(seg3[1].values()):.1f})")

print("=" * 72)
print("Overnight sleep: 8 hours asleep on 0.4.0-beta.3 (same sleep configuration as 0.4.0)")
# Read by the tester from the battery percentage before and after; no log file.
start, end, hours = 98, 82, 8
drop = start - end
lo_drop, hi_drop = drop - 1, drop + 1   # each whole-percent reading hides up to 1 point
print(f"  battery {start}% -> {end}% over {hours} h: {drop} points, {drop / hours:.1f} %/h observed")
print(f"  whole-percent readings: anything from {lo_drop / hours:.3f} to {hi_drop / hours:.3f} %/h is consistent")
HIGH_STEP_MAH = 26.3   # Experiment 5, segment 1: the implied step above about 80%
print(f"  at 35 mAh per percent: {drop * STEP_MAH / hours:.0f} mA "
      f"({lo_drop * STEP_MAH / hours:.1f} to {hi_drop * STEP_MAH / hours:.1f} mA)")
print(f"  at {HIGH_STEP_MAH} mAh per percent (the run was entirely above 80%): {drop * HIGH_STEP_MAH / hours:.0f} mA "
      f"({lo_drop * HIGH_STEP_MAH / hours:.1f} to {hi_drop * HIGH_STEP_MAH / hours:.1f} mA)")
print("  so about 50 to 74 mA: consistent with segment 3's ~70 mA, or a little lower")

print("=" * 72)
print("Comparisons and projections")
print(f"  reduction: (250 - 70) / 250 = {(B - S) / B:.0%}")
for s in (min(seg3[1].values()), max(seg3[1].values())):
    print(f"    with {s:.1f} mA instead of 70 (segment 3's corrected range): {(B - s) / B:.0%}")
rate = S / STEP_MAH
print(f"  0.4.0 rate: 70 mA / 35 mAh per percent = {rate:.1f} %/h")
band = 1 / drop
print(f"  range on the projection chart, from the overnight sleep's whole-percent readings (percent per hour, independent of step size): +/-{band:.1%} = "
      f"{rate * (1 - band):.2f} to {rate * (1 + band):.2f} %/h")
for h in (0.5, 1, 2, 4, 8):
    print(f"    {h:>4} h asleep: ~{rate * h:.0f}%  ({rate * (1 - band) * h:.1f} to {rate * (1 + band) * h:.1f}%)")
