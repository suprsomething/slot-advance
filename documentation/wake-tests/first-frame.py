#!/usr/bin/env python3
"""First-frame resume times for slot.advance 0.4.1, from the V1 report.

Reads v1-first-frame-0.4.1-beta.1.txt (unedited watcher output) and prints the first-frame
time of every wake, in milliseconds from the start of the kernel's resume, with the median
for each session. The 0.4.0 baseline (R2, 14 October 2026) is recorded below; its report was
not kept. Run from the repository root:

    python3 documentation/wake-tests/first-frame.py
"""
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPORT = HERE / "v1-first-frame-0.4.1-beta.1.txt"

# 0.4.0 baseline, experiment R2: the same watcher and procedure on 0.4.0.
R2_OFF_MEDIAN, R2_OFF_RANGE, R2_OFF_N = 1251, (1236, 1270), 10
R2_ON_MEDIAN, R2_ON_N = 820, 9

text = REPORT.read_text()
body = text.split("p0r-resume.txt", 1)[1]
sessions = re.split(r"\n=+\nBoot: ", body)[1:]
names = ["Session A, Wi-Fi off", "Session B, wifi=on"]
medians = []
for name, s in zip(names, sessions):
    loaded = re.search(r"Wi-Fi driver loaded after start-up: (\w+)", s).group(1)
    ff = [int(m) for m in re.findall(r"TIMELINE \(ms from resume start\):.*first frame (\d+)", s)]
    med = statistics.median(ff)
    medians.append(med)
    # Successful wakes come from the report's own header: session B had 10, of which the
    # watcher timed 7 (it misses a wake when the lid opens too soon after it settles).
    resumed = re.search(rf"{name.split(',')[0]} \(boot[^)]*\): [^.]*\. (\d+) wakes", text).group(1)
    print(f"{name} (driver loaded: {loaded}): {resumed} wakes resumed, {len(ff)} timed")
    print(f"  first frame, ms: {sorted(ff)}")
    print(f"  range {min(ff)} to {max(ff)}, median {med:g}")

off = medians[0]
saved = R2_OFF_MEDIAN - off
print()
print(f"0.4.0 (R2), Wi-Fi off: median {R2_OFF_MEDIAN} ms ({R2_OFF_RANGE[0]} to {R2_OFF_RANGE[1]}, {R2_OFF_N} wakes)")
print(f"0.4.1-beta.1, Wi-Fi off: median {off:g} ms")
print(f"  {saved:g} ms sooner, {100 * saved / R2_OFF_MEDIAN:.1f}% less time from resume start to first frame")
print(f"wifi=on: 0.4.0 median {R2_ON_MEDIAN} ms ({R2_ON_N} wakes), 0.4.1-beta.1 median {medians[1]:g} ms "
      f"(timed wakes only); no improvement is claimed with wifi=on")
print("R2's samples are not in the repository: its figures above are recorded, not recalculated.")
