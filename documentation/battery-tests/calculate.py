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
print("Test B: 0.3.0 (semi-stock) software standby, modified build held awake (lid closed, 2 h)")
b = read("test-b-software-standby.txt")
interval = num(r"Whole test interval \(uptime\): ([\d.]+) s", b)
p0, p1 = (int(x) for x in re.search(r"capacity: (\d+)% -> (\d+)%", b).groups())
end_ma = (p0 - p1) * STEP_MAH / (interval / 3600)
end_err = STEP_MAH / (interval / 3600)
print(f"  battery {p0}% -> {p1}% over {interval:.0f} s: {p0 - p1} points, {(p0 - p1) / (interval / 3600):.1f} %/h observed")
print(f"  whole-percent readings: {(p0 - p1 - 1) / (interval / 3600):.1f} to {(p0 - p1 + 1) / (interval / 3600):.1f} %/h")
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
# Lid wake armed, USB and Wi-Fi off: two nights on 0.4.0, then one long sleep on the 0.4.1
# candidate, whose only difference is that the stale Wi-Fi card is no longer registered.
NIGHTS = [
    # name, start %, end %, hours asleep, how the end reading was taken
    ("0.4.0-beta.3, 8 hours (tester's notes; no log)", 98, 82, 8.0,
     "read the next morning, after the 8-hour timer had shut the device down"),
    ("0.4.0, night A (overnight-0.4.0-night-a.txt)", 94, 74, 27987 / 3600,
     "read by the watcher straight after a lid wake at 7 h 46 min"),
]
NIGHT_B = ("0.4.1-beta.1, night B (extended-0.4.1-beta.1-night-b.txt)", 99, 75, 27420 / 3600,
           "read by the watcher straight after a lid wake at 7 h 37 min; started at 99%, "
           "about a minute after power-on")
print("Extended sleeps: 0.4.0 (two nights), then 0.4.1-beta.1")
rates = []
for name, a0, a1, h, how in NIGHTS:
    d = a0 - a1
    rates.append(d / h)
    print(f"  {name}")
    print(f"     {a0}% -> {a1}% over {h:.2f} h: {d} points, {d / h:.2f} %/h observed "
          f"(whole-percent readings: {(d - 1) / h:.2f} to {(d + 1) / h:.2f} %/h)")
    print(f"     end reading {how}")
print(f"  0.4.0's observed range (two nights): {min(rates):.1f} to {max(rates):.1f} %/h")
name, a0, a1, h, how = NIGHT_B
d = a0 - a1
print(f"  {name}")
print(f"     {a0}% -> {a1}% over {h:.2f} h: {d} points, {d / h:.2f} %/h observed "
      f"(whole-percent readings: {(d - 1) / h:.2f} to {(d + 1) / h:.2f} %/h)")
print(f"     end reading {how}")
all_rates = rates + [d / h]
print(f"  observed range, all three: {min(all_rates):.1f} to {max(all_rates):.1f} %/h")
print("  Night B started at 99%; night A started at 94% after the gauge fell 5 points in 22")
print("  minutes awake. The top of the gauge is unmeasured, so the two cannot show whether")
print("  0.4.0 and 0.4.1 differ in standby use: inconclusive, not a parity or a regression.")
print("  Night B is not converted to a current.")

# Converting either night to a current needs the size of each 1% step, and that is only
# measured below about 84%: Experiment 5's awake ticks give 26.3 mAh at 83%, 31 at 78%,
# 36 at 73% and 36 at 69%, each assuming about 250 mA awake. Nothing measures 85% to 100%.
# Applying 26.3 mAh to every step above 80% and 35 below, as an illustration only:
HIGH_STEP_MAH = 26.3
def mah(a0, a1):
    hi = max(0, a0 - max(a1, 80))
    lo = max(0, min(a0, 80) - a1)
    return hi * HIGH_STEP_MAH + lo * STEP_MAH
print("  illustration only, with 26.3 mAh per step above 80% and 35 below:")
for name, a0, a1, h, how in NIGHTS:
    print(f"     {name.split(',')[0]}: {mah(a0, a1):.0f} mAh, {mah(a0, a1) / h:.0f} mA")
print("  These two do not agree, and the evidence cannot say why: the step sizes above 85%")
print("  are unmeasured, the end readings were taken differently, and in the 22 minutes")
print("  before night A, awake, the gauge fell from 99% to 94%: about 20 mAh a step at")
print("  250 to 300 mA awake, if the gauge is right, which would suggest small steps near full.")
print("  Neither night is used as a current. The current figure stays Experiment 5's,")
print("  measured where the step sizes were estimated.")

print("=" * 72)
print("Comparisons and projections")
print(f"  reduction: (250 - 70) / 250 = {(B - S) / B:.0%}")
for s in (min(seg3[1].values()), max(seg3[1].values())):
    print(f"    with {s:.1f} mA instead of 70 (segment 3's corrected range): {(B - s) / B:.0%}")
print(f"  0.4.0 at Experiment 5's ~70 mA, where one step is ~35 mAh: {S / STEP_MAH:.1f} %/h")
LO_RATE, HI_RATE = 2.0, 3.2   # the three extended sleeps, rounded
print(f"  shown on the battery gauge over three extended sleeps: {LO_RATE} to {HI_RATE} %/h")
print("  projected battery used, if asleep the whole time (projections, not measurements):")
for h in (0.25, 0.5, 1, 2, 4, 8):
    print(f"    {h:>5} h asleep: {LO_RATE * h:.1f} to {HI_RATE * h:.1f}%")
