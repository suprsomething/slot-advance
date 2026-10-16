#!/usr/bin/env python3
"""Generate the slot advance battery and wake-up charts as self-contained SVGs.

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
LO_RATE, HI_RATE = 2.0, 3.2   # %/h shown by the gauge over three extended sleeps, 0.4.0 and 0.4.1 (see calculate.py)


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
        # name, note, low %/h, high %/h, what was seen, colour
        ("0.3.0 software standby", "test build held awake", 7, 7, "61% to 47% in 2 hours", CONTEXT),
        ("First suspend-to-RAM", "development test", 3, 3, "68% to 62% in 2 hours", MID),
        ("0.4.0 and 0.4.1", "three extended sleeps", LO_RATE, HI_RATE, "16% in 8 h, 20% in 7.8 h, 24% in 7.6 h", ACCENT),
    ]
    body = [header("Battery used per hour, lid closed",
                   "Each block is 1% of the battery, as shown by the battery gauge")]
    y = 118
    cell, gap = 46, 8
    for name, note, lo, hi, seen, col in rows:
        body.append(labelled(32, y + 18, name, note))
        full = int(lo)
        for i in range(full):
            cx = 32 + i * (cell + gap)
            body.append(f'<rect x="{cx}" y="{y + 32}" width="{cell}" height="34" rx="4" fill="{col}"/>')
        # The range above the lowest observation: outlined blocks, filled to the highest.
        i = full
        while i < hi:
            cx = 32 + i * (cell + gap)
            part = min(1, hi - i)
            body.append(f'<rect x="{cx + 0.75}" y="{y + 32.75}" width="{cell - 1.5}" height="32.5" rx="4" fill="none" stroke="{col}" stroke-width="1.5"/>')
            body.append(f'<rect x="{cx}" y="{y + 32}" width="{cell * part:.1f}" height="34" rx="4" fill="{col}" opacity="0.45"/>')
            i += 1
        label = f"~{lo:g}% an hour" if lo == hi else f"~{lo:g} to {hi:g}% an hour"
        end = i
        body.append(text(32 + end * (cell + gap) + 6, y + 57, label, 18, INK, 650))
        body.append(text(688, y + 18, seen, 15, INK3, anchor="end"))
        y += 100
    body.append(notes(434, [
        "Observed battery-percentage drops (whole 1% steps). Ordinary 0.3.0 shuts down",
        "3 minutes after the lid closes; its rate comes from a test build held in standby.",
        "Extended sleeps: 0.4.0-beta.3 (about 2% an hour), 0.4.0 (2.6%) and 0.4.1-beta.1",
        "(3.2%, from 99%). The gauge's 1% steps differ in size, so the rate shown depends",
        "partly on the charge level. These are observations, not a guaranteed range.",
    ]))
    return card(566, "\n".join(body),
                "Battery used per hour with the lid closed: about 7% for 0.3.0 software standby, 3% for the first suspend-to-RAM test, 2 to 3.2% for 0.4.0 and 0.4.1",
                "Blocks of 1 percent, from observed battery-gauge drops. 0.3.0 software standby (a test build held awake; ordinary 0.3.0 shuts down after 3 minutes): "
                "61% to 47% in 2 hours, about 7% an hour. First suspend-to-RAM test: 68% to 62% in 2 hours, about 3% an hour. "
                "0.4.0 and 0.4.1, three extended sleeps: 98% to 82% in 8 hours on 0.4.0-beta.3 (about 2% an hour), 94% to 74% in 7 h 46 min on 0.4.0 (about 2.6% an hour) "
                "and 99% to 75% in 7 h 37 min on 0.4.1-beta.1 (about 3.2% an hour). "
                "The gauge's 1% steps differ in size, so the rate shown depends partly on the charge level.")


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
        "Whiskers: the battery gauge's resolution over each run. 0.4.0's current is from a",
        "1.5-hour run below 80% charge, where the gauge's step size was estimated. The",
        "extended sleeps started above 80%, where it is not, so they give no current.",
    ]))
    return card(566, "\n".join(body),
                "Estimated lid-closed current draw: about 250 mA for 0.3.0 software standby, 105 mA for the first suspend-to-RAM test, 70 mA for the 0.4.0 configuration",
                "Horizontal bar chart of currents calculated from battery-gauge readings. 0.3.0 software standby, a test build held awake: about 250 mA (248 to 253). "
                "First suspend-to-RAM with USB and Wi-Fi on: about 105 mA (87.5 to 122.5). 0.4.0 configuration, from a 1.5-hour run below 80% charge, where the gauge's step size was estimated: "
                "about 70 mA (70 to 75). The extended sleeps started above 80% charge, where the gauge's step size is not known, so they are not converted to a current. About 72% less than software standby.")


# ---------------------------------------------------------------- projection (doc)
def projection():
    x0, x1, y0, y1 = 84, 680, 420, 120
    top = 30
    def px(h): return x0 + (x1 - x0) * h / 8
    def py(p): return y0 - (y0 - y1) * p / top
    body = [header("Battery used while asleep",
                   "About 2 to 3.2% an hour, as three extended sleeps showed (0.4.0 and 0.4.1)")]
    for p in range(0, top + 1, 5):
        body.append(f'<line x1="{x0}" y1="{py(p):.1f}" x2="{x1}" y2="{py(p):.1f}" stroke="{GRID}"/>')
        body.append(text(x0 - 12, py(p) + 5, f"{p}%", 14, INK3, anchor="end"))
    for h in range(0, 9):
        body.append(text(px(h), y0 + 26, f"{h}", 14, INK3, anchor="middle"))
    body.append(text((x0 + x1) / 2, y0 + 52, "Hours asleep", 15, INK2, anchor="middle"))
    body.append(text(26, (y0 + y1) / 2, "Battery used", 15, INK2, anchor="middle",
                     extra=f' transform="rotate(-90 26 {(y0 + y1) / 2})"'))
    band = f"M{px(0)} {py(0)} L{px(8):.1f} {py(8 * HI_RATE):.1f} L{px(8):.1f} {py(8 * LO_RATE):.1f} Z"
    body.append(f'<path d="{band}" fill="{ACCENT}" opacity="0.18"/>')
    for r in (LO_RATE, HI_RATE):
        body.append(f'<line x1="{px(0)}" y1="{py(0)}" x2="{px(8):.1f}" y2="{py(8 * r):.1f}" '
                    f'stroke="{ACCENT}" stroke-width="2" stroke-dasharray="8 6"/>')
    for h in (2, 4):
        body.append(text(px(h) - 10, py(h * HI_RATE) - 12, f"{h} hr: {h * LO_RATE:g} to {h * HI_RATE:g}%", 15, INK, 600, "end"))

    def observed(h, pct, label, dy=5):
        cx, cy = px(h), py(pct)
        w = 7.7 * len(label)
        body.append(f'<rect x="{cx - 18 - w:.1f}" y="{cy + dy - 16:.1f}" width="{w + 8:.1f}" height="22" rx="4" fill="{BG}" opacity="0.88"/>')
        body.append(text(cx - 14, cy + dy, label, 15, INK, 600, "end"))
        body.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="7" fill="{INK}"/>')

    observed(27420 / 3600, 24, "24% in 7.6 hr (0.4.1-beta.1)")
    observed(27987 / 3600, 20, "20% in 7.8 hr (0.4.0)")
    observed(8, 16, "16% in 8 hr (0.4.0-beta.3)")
    body.append(f'<circle cx="{px(1.5):.1f}" cy="{py(3):.1f}" r="7" fill="{INK}"/>')
    body.append(text(px(1.5) + 14, py(3) + 26, "3% in 1.5 hr (Experiment 5)", 14, INK, 600))
    lx, ly = 430, 372
    body.append(f'<rect x="{lx-6}" y="{ly-20}" width="262" height="62" rx="8" fill="{BG}" opacity="0.9"/>')
    body.append(f'<line x1="{lx}" y1="{ly-6}" x2="{lx+28}" y2="{ly-6}" stroke="{ACCENT}" stroke-width="2" stroke-dasharray="8 6"/>')
    body.append(text(lx + 38, ly - 1, f"Projected at {LO_RATE:g} and {HI_RATE:g}% an hour", 14, INK2))
    body.append(f'<circle cx="{lx + 14}" cy="{ly + 14}" r="6" fill="{INK}"/>')
    body.append(text(lx + 38, ly + 19, "Observed", 14, INK2))
    body.append(notes(512, [
        "The three extended sleeps set the range; it is what the gauge showed, not a",
        "guaranteed range. The gauge's 1% steps differ in size, so the rate depends partly",
        "on the charge level. Real use also varies with battery age and temperature.",
    ]))
    return card(590, "\n".join(body),
                "Battery used while asleep: about 2 to 3.2% an hour, 16 to 26% over 8 hours, from three extended sleeps on 0.4.0 and 0.4.1",
                "Two dashed lines rise from 0% at 0 hours to 16% and about 26% at 8 hours, at 2 and 3.2% an hour, with the range between them shaded. "
                "Four observed points: 16% (98% to 82%) over 8 hours on 0.4.0-beta.3, 20% (94% to 74%) over 7 hours 46 minutes on 0.4.0, "
                "24% (99% to 75%) over 7 hours 37 minutes on 0.4.1-beta.1, and 3% over 1.5 hours in an earlier experiment.")


# ---------------------------------------------------------------- wake-up time (README)
# From documentation/wake-tests/first-frame.py: medians, ms from resume start to first frame.
WAKE_040, WAKE_041 = 1251, 808


def wake():
    rows = [
        ("0.4.0", "10 wakes, 1236 to 1270 ms", WAKE_040, CONTEXT),
        ("0.4.1", "11 wakes, 803 to 828 ms", WAKE_041, ACCENT),
    ]
    x0, x1 = 32, 560
    scale = (x1 - x0) / 1400
    body = [header("Waking from sleep, Wi-Fi off",
                   "Median time from the start of resume to the game's first frame")]
    for v in (0, 500, 1000):
        gx = x0 + v * scale
        body.append(f'<line x1="{gx:.1f}" y1="104" x2="{gx:.1f}" y2="300" stroke="{GRID}" stroke-width="1"/>')
        body.append(text(gx, 324, f"{v / 1000:g} s", 14, INK3, anchor="middle" if v else "start"))
    y = 112
    for name, note, v, col in rows:
        body.append(labelled(x0, y + 20, name, note))
        by = y + 34
        bw = v * scale
        body.append(f'<rect x="{x0}" y="{by}" width="{bw:.1f}" height="34" rx="4" fill="{col}"/>')
        body.append(text(x0 + bw + 12, by + 24, f"{v / 1000:.2f} s", 20, INK, 650))
        y += 92
    saved = WAKE_040 - WAKE_041
    body.append(f'<rect x="512" y="200" width="176" height="84" rx="10" fill="#221F33" stroke="{ACCENT}" stroke-width="1.5"/>')
    body.append(text(600, 238, f"{100 * saved / WAKE_040:.0f}% less", 28, INK, 750, "middle"))
    body.append(text(600, 264, f"{saved} ms sooner", 15, INK2, anchor="middle"))
    body.append(notes(362, [
        "Measured on the RG SP (2026) from the kernel's own log. The moment the lid starts",
        "to open isn't logged, so the time before resume starts isn't included. With",
        "wifi=on, both versions take about 0.82 s.",
    ]))
    return card(442, "\n".join(body),
                f"Waking from sleep with Wi-Fi off: median {WAKE_040 / 1000:.2f} s on 0.4.0 and {WAKE_041 / 1000:.2f} s on 0.4.1, 35% less time",
                f"Two horizontal bars. Median time from the start of the kernel's resume to the game's first frame, with Wi-Fi off: "
                f"0.4.0, {WAKE_040} ms over 10 wakes (1236 to 1270); 0.4.1, {WAKE_041} ms over 11 wakes (803 to 828). "
                f"{saved} ms sooner, 35% less time. The time before resume starts is not logged and not included. With wifi=on, both take about 0.82 s.")


WAKE_OUT = OUT.parent / "wake"
WAKE_OUT.mkdir(parents=True, exist_ok=True)
(WAKE_OUT / "wake-first-frame.svg").write_text(wake())
(OUT / "battery-drain-hourly.svg").write_text(hourly())
(OUT / "battery-current-draw.svg").write_text(current())
(OUT / "battery-drain-projection.svg").write_text(projection())
print("wrote", OUT)
