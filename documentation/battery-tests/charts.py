#!/usr/bin/env python3
"""Generate the slot advance battery charts as self-contained SVGs.

Run from the repository root:  python3 documentation/battery-tests/charts.py

Every figure comes from documentation/battery-tests/calculate.py and the raw reports beside it.
Each chart sits on its own dark card, so it reads the same in GitHub's light and dark
themes. Caveats are drawn inside the card, directly under the data they qualify.
"""
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / ".github/resources/battery"
OUT.mkdir(parents=True, exist_ok=True)

BG = "#17161D"
GRID = "#2E2C38"
INK = "#EEECF4"
INK2 = "#C2BFD0"
INK3 = "#A3A0B3"
ACCENT = "#8B7CF6"   # lilac: the 0.4.0 result
CONTEXT = "#5C596C"  # earlier generations
MID = "#7A7690"
FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif"
W = 720
BAND = 1 / 16        # 0.4.0 %/h range: whole-percent readings of the 16% overnight drop (see calculate.py)


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def card(h, body, title, desc):
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{h}" viewBox="0 0 {W} {h}" role="img" aria-labelledby="t d">
<title id="t">{esc(title)}</title>
<desc id="d">{esc(desc)}</desc>
<rect x="0.5" y="0.5" width="{W-1}" height="{h-1}" rx="14" fill="{BG}" stroke="{GRID}"/>
<g font-family="{FONT}">
{body}
</g>
</svg>
"""


def text(x, y, s, size=18, fill=INK, weight=400, anchor="start", extra=""):
    return (f'<text x="{x}" y="{y}" font-size="{size}" fill="{fill}" font-weight="{weight}" '
            f'text-anchor="{anchor}"{extra}>{esc(s)}</text>')


def header(t, sub):
    return text(32, 50, t, 24, INK, 650) + "\n" + text(32, 80, sub, 16, INK2)


def notes(y, lines):
    """Caveat lines, legible at phone width (16px at 720 wide)."""
    return "\n".join(text(32, y + i * 24, line, 16, INK3) for i, line in enumerate(lines))


def labelled(x, y, name, note, size=18):
    return (f'<text x="{x}" y="{y}" font-size="{size}" fill="{INK}" font-weight="600">{esc(name)}'
            f'<tspan dx="10" font-size="15" font-weight="400" fill="{INK3}">{esc(note)}</tspan></text>')


# ---------------------------------------------------------------- hourly drain (README)
def hourly():
    rows = [
        ("0.3.0 software standby", "test build held awake", 7, "61% to 47% in 2 hours", CONTEXT),
        ("First suspend-to-RAM", "development test", 3, "68% to 62% in 2 hours", MID),
        ("0.4.0-beta.3", "8-hour overnight sleep", 2, "98% to 82% in 8 hours", ACCENT),
    ]
    body = [header("Battery used per hour, lid closed",
                   "Each block is 1% of the battery, as shown by the battery gauge")]
    y = 118
    cell, gap = 46, 8
    for name, note, n, seen, col in rows:
        body.append(labelled(32, y + 18, name, note))
        for i in range(n):
            cx = 32 + i * (cell + gap)
            body.append(f'<rect x="{cx}" y="{y + 32}" width="{cell}" height="34" rx="4" fill="{col}"/>')
        body.append(text(32 + n * (cell + gap) + 6, y + 57, f"~{n}% an hour", 18, INK, 650))
        body.append(text(688, y + 18, seen, 15, INK3, anchor="end"))
        y += 100
    body.append(notes(434, [
        "Observed battery-percentage drops (whole 1% steps).",
        "Ordinary 0.3.0 shuts down 3 minutes after the lid closes; its hourly rate",
        "comes from a test build deliberately held in standby.",
        "The 0.4.0 figure comes from real testing of 0.4.0-beta.3: an 8-hour",
        "overnight sleep, matching an earlier 1.5-hour experiment (77% to 74%).",
    ]))
    return card(566, "\n".join(body),
                "Battery used per hour with the lid closed: about 7% for 0.3.0 software standby, 3% for the first suspend-to-RAM test, 2% for 0.4.0-beta.3",
                "Blocks of 1 percent, from observed battery-gauge drops. 0.3.0 software standby (a test build held awake; ordinary 0.3.0 shuts down after 3 minutes): "
                "61% to 47% in 2 hours, about 7% an hour. First suspend-to-RAM test: 68% to 62% in 2 hours, about 3% an hour. "
                "0.4.0-beta.3, tested overnight: 98% to 82% in 8 hours, about 2% an hour, matching an earlier 1.5-hour experiment (77% to 74%).")


# ---------------------------------------------------------------- current draw (doc)
def current():
    rows = [
        ("0.3.0 software standby", "test build held awake", 250, 248, 253, CONTEXT),
        ("First suspend-to-RAM", "USB and Wi-Fi on", 105, 87.5, 122.5, MID),
        ("0.4.0 configuration", "USB and Wi-Fi off", 70, 69.6, 75.4, ACCENT),
    ]
    x0, x1 = 32, 600
    scale = (x1 - x0) / 280
    body = [header("Estimated lid-closed current draw",
                   "Calculated from battery-gauge readings, not measured with test equipment")]
    for v in (0, 100, 200):
        gx = x0 + v * scale
        body.append(f'<line x1="{gx:.1f}" y1="104" x2="{gx:.1f}" y2="400" stroke="{GRID}" stroke-width="1"/>')
        body.append(text(gx, 424, f"{v} mA", 14, INK3, anchor="middle" if v else "start"))
    y = 112
    for name, note, v, lo, hi, col in rows:
        body.append(labelled(x0, y + 20, name, note))
        by = y + 34
        bw = v * scale
        body.append(f'<rect x="{x0}" y="{by}" width="{bw:.1f}" height="30" rx="4" fill="{col}"/>')
        lx, hx, cy = x0 + lo * scale, x0 + hi * scale, by + 15
        body.append(f'<path d="M{lx:.1f} {cy}H{hx:.1f}M{lx:.1f} {cy-7}V{cy+7}M{hx:.1f} {cy-7}V{cy+7}" '
                    f'stroke="{INK2}" stroke-width="1.5" fill="none" opacity="0.85"/>')
        body.append(text(max(hx, x0 + bw) + 12, by + 21, f"~{v} mA", 18, INK, 650))
        y += 96
    body.append(f'<rect x="452" y="290" width="236" height="90" rx="10" fill="#221F33" stroke="{ACCENT}" stroke-width="1.5"/>')
    body.append(text(570, 336, "~72% less", 30, INK, 750, "middle"))
    body.append(text(570, 362, "than software standby", 15, INK2, anchor="middle"))
    body.append(notes(458, [
        "Whiskers: the battery gauge's resolution over each run. 0.4.0's current is from",
        "a 1.5-hour run below 80% charge, where the gauge is linear. The 8-hour sleep on",
        "0.4.0-beta.3 used the same 2% an hour, high in the gauge's range.",
    ]))
    return card(566, "\n".join(body),
                "Estimated lid-closed current draw: about 250 mA for 0.3.0 software standby, 105 mA for the first suspend-to-RAM test, 70 mA for the 0.4.0 configuration",
                "Horizontal bar chart of currents calculated from battery-gauge readings. 0.3.0 software standby, a test build held awake: about 250 mA (248 to 253). "
                "First suspend-to-RAM with USB and Wi-Fi on: about 105 mA (87.5 to 122.5). 0.4.0 configuration, from a 1.5-hour run where the gauge is linear: "
                "about 70 mA (70 to 75). An 8-hour overnight sleep on 0.4.0-beta.3 used the same 2% an hour. About 72% less than software standby.")


# ---------------------------------------------------------------- projection (doc)
def projection():
    x0, x1, y0, y1 = 84, 680, 420, 120
    def px(h): return x0 + (x1 - x0) * h / 8
    def py(p): return y0 - (y0 - y1) * p / 20
    rate = 2.0
    body = [header("Battery used while asleep",
                   "0.4.0 at about 2% an hour, as measured over an 8-hour overnight sleep on 0.4.0-beta.3")]
    for p in (0, 5, 10, 15, 20):
        body.append(f'<line x1="{x0}" y1="{py(p):.1f}" x2="{x1}" y2="{py(p):.1f}" stroke="{GRID}"/>')
        body.append(text(x0 - 12, py(p) + 5, f"{p}%", 14, INK3, anchor="end"))
    for h in range(0, 9):
        body.append(text(px(h), y0 + 26, f"{h}", 14, INK3, anchor="middle"))
    body.append(text((x0 + x1) / 2, y0 + 52, "Hours asleep", 15, INK2, anchor="middle"))
    body.append(text(26, (y0 + y1) / 2, "Battery used", 15, INK2, anchor="middle",
                     extra=f' transform="rotate(-90 26 {(y0 + y1) / 2})"'))
    lo, hi = rate * (1 - BAND), rate * (1 + BAND)
    band = f"M{px(0)} {py(0)} L{px(8):.1f} {py(8 * hi):.1f} L{px(8):.1f} {py(8 * lo):.1f} Z"
    body.append(f'<path d="{band}" fill="{ACCENT}" opacity="0.16"/>')
    body.append(f'<line x1="{px(0)}" y1="{py(0)}" x2="{px(8):.1f}" y2="{py(8 * rate):.1f}" '
                f'stroke="{ACCENT}" stroke-width="2.5" stroke-dasharray="8 6"/>')
    for h in (0.5, 1, 2, 4, 8):
        body.append(f'<circle cx="{px(h):.1f}" cy="{py(h * rate):.1f}" r="5" fill="{BG}" stroke="{ACCENT}" stroke-width="2.5"/>')
    body.append(text(px(1) - 10, py(2) - 10, "1 hr: ~2%", 15, INK, 600, "end"))
    for h in (2, 4):
        body.append(text(px(h) - 10, py(h * rate) - 12, f"{h} hr: ~{h * rate:.0f}%", 15, INK, 600, "end"))
    body.append(f'<circle cx="{px(8):.1f}" cy="{py(16):.1f}" r="7" fill="{INK}"/>')
    body.append(text(px(8) - 12, py(16) - 16, "Observed: 16% in 8 hr", 15, INK, 600, "end"))
    body.append(f'<circle cx="{px(1.5):.1f}" cy="{py(3):.1f}" r="7" fill="{INK}"/>')
    body.append(text(px(1.5) + 14, py(3) + 26, "Observed: 3% in 1.5 hr", 14, INK, 600))
    lx, ly = x0 + 6, 138
    body.append(f'<rect x="{lx-6}" y="{ly-20}" width="330" height="62" rx="8" fill="{BG}" opacity="0.9"/>')
    body.append(f'<line x1="{lx}" y1="{ly-6}" x2="{lx+28}" y2="{ly-6}" stroke="{ACCENT}" stroke-width="2.5" stroke-dasharray="8 6"/>')
    body.append(text(lx + 38, ly - 1, "~2% an hour", 14, INK2))
    body.append(f'<rect x="{lx}" y="{ly+8}" width="28" height="12" fill="{ACCENT}" opacity="0.3"/>')
    body.append(text(lx + 38, ly + 19, f"Range from 1% readings ({lo:.2f} to {hi:.2f}%/hr)", 14, INK2))
    body.append(notes(512, [
        "The range is the gauge's 1% resolution over the 8-hour sleep. Real use varies with",
        "battery age, temperature and charge level. The RG SP (2026) only sleeps this long if",
        "Settings > Sleep is set long enough; then it shuts down cleanly.",
    ]))
    return card(590, "\n".join(body),
                "Battery used while asleep on 0.4.0: about 2% an hour, 16% over 8 hours, as measured overnight",
                "A dashed line rises from 0% at 0 hours to about 16% at 8 hours, at about 2% an hour, with a shaded range of 1.88 to 2.12% an hour from the gauge's "
                "1% resolution. Two observed points: 16% (98% to 82%) over an 8-hour overnight sleep on 0.4.0-beta.3, and 3% over 1.5 hours in an earlier experiment.")


(OUT / "battery-drain-hourly.svg").write_text(hourly())
(OUT / "battery-current-draw.svg").write_text(current())
(OUT / "battery-drain-projection.svg").write_text(projection())
print("wrote", OUT)
