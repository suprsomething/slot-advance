# Sleep and battery: how it works and how it was measured

This page is the detail behind the battery figures in the [README](../README.md). It covers how sleep has changed across slot.advance's releases, what 0.4.0 does when the lid closes, how the battery figures were obtained, how far they can be trusted, and what was checked on the device before release.

Everything here applies to the **Anbernic RG SP (2026)** (720 x 480 screen) running BaseOS 1.3.0 (Linux 4.9). Version numbers are **slot.advance** versions, not upstream slot's. **0.4.0** (tag `advance-0.4.0`) is the current release, released after the checks under [Validation status](#validation-status). **0.3.0** (tag `advance-v3`) is the previous release.

## Contents

- [Summary](#summary)
- [The evolution of sleep](#the-evolution-of-sleep)
- [What 0.4.0 does when the lid closes](#what-040-does-when-the-lid-closes)
- [Battery results](#battery-results)
- [Methodology and uncertainty](#methodology-and-uncertainty)
- [Raw reports and calculations](#raw-reports-and-calculations)
- [Validation status](#validation-status)

---

## Summary

| Lid closed | Estimated current | Observed battery drop | Source |
|---|---|---|---|
| 0.3.0 software standby, test build held awake | ~250 mA (248 to 253) | 61% to 47% in 2 hours (~7% an hour) | Test B |
| First suspend-to-RAM, USB and Wi-Fi on | ~105 mA (87.5 to 122.5) | 68% to 62% in 2 hours (~3% an hour) | Test A |
| 0.4.0-beta.3, USB and Wi-Fi off | Not converted (above 80%; see below) | 98% to 82% in 8 hours (~2% an hour) | Overnight sleep, tester's notes |
| 0.4.0, USB and Wi-Fi off | Not converted (above 80%; see below) | 94% to 74% in 7 h 46 min (~2.6% an hour) | Overnight sleep, night A log |
| 0.4.0 configuration, reproduced | ~70 mA (70 to 75) | 77% to 74% in 1.5 hours (~2% an hour) | Experiment 5, segment 3 |

- All currents are **estimates calculated from the battery gauge**, not measured with electrical test equipment. They rely on the battery's nominal 3,500 mAh capacity and on assumptions about the gauge's calibration. The ranges shown reflect each test's method, not every possible systematic calibration error. No independent current measurement was made.
- **About 72% less** standby current: (250 - 70) / 250, using Experiment 5's 70 mA, measured below 80%, where the gauge's step sizes were estimated. Its range of 70 to 75 mA gives 70% to 72%.
- **About 2 to 2.6% an hour on the battery gauge**, from two overnight sleeps with 0.4.0's sleep configuration: 0.4.0-beta.3 went from 98% to 82% in 8 hours (~2% an hour), and 0.4.0 from 94% to 74% in 7 hours 46 minutes (~2.6% an hour). The range is reported as observed rather than averaged.
- **The overnight sleeps are not converted to a current.** Both ran above 80%, where the gauge's 1% steps are smaller than 35 mAh and, above about 85%, unmeasured. Converted with the one step size estimated up there, the two nights would give about 53 and 74 mA, which do not agree; see [the overnight sleeps](#methodology-and-uncertainty). The current figures stay with the runs where the step size was estimated.
- **Ordinary 0.3.0 shuts down 3 minutes after the lid closes.** Its hourly figure describes a test build deliberately held in standby. A switched-off device draws only a negligible amount compared with any standby, so 0.4.0's benefit is staying asleep efficiently for hours with instant lid wake, not lower consumption than switching off.
- No change to battery use **during gameplay** has been measured, and none is claimed.

---

## The evolution of sleep

| | Upstream slot | slot.advance 0.3.0 | slot.advance 0.4.0 |
|---|---|---|---|
| CPU | Running | Running, mostly idle | Suspended |
| Drawing | A black frame every display refresh (~60 a second) | Two black frames, then none | None |
| Display | Backlight off | Backlight off | Suspended by its driver |
| Memory | Active | Active | Kept powered; the game is preserved |
| Unused USB host controllers and Wi-Fi | On | On | Off from startup |
| Emulator | Paused | Paused, no longer ticked | Paused, then frozen with the whole system |
| After the lid closes | Powers off after 3 minutes | Powers off after 3 minutes | Sleeps for the chosen time (3 min to 8 hr), then shuts down cleanly |
| Estimated lid-closed current | Not measured | ~250 mA (test build held awake) | ~70 mA |

**Upstream slot** (verified in its source, `crates/slot/src/device_app.rs` at the fork point): closing the lid turns off the backlight and pauses the game, but the main loop keeps running at the display's refresh rate (about 60 Hz). Each pass draws a full-screen black rectangle and swaps buffers. After 3 minutes it powers off, and the next boot restores the game from its suspend slot. Its standby current was never measured.

**slot.advance 0.3.0** (`crates/slot/src/standby.rs`): once the screen is dark, it draws two black frames and then stops drawing and stops ticking the emulator. It wakes every 50 ms to read input, poll the lid (every 150 ms) and run its timers. The CPU, memory and display hardware stay powered. Test B logged zero GPU interrupts a minute in this state, with the CPU governor at `performance`, 1.512 GHz. The 3-minute power-off is unchanged.

**slot.advance 0.4.0**: real Linux suspend-to-RAM, described next.

---

## What 0.4.0 does when the lid closes

1. **Save first.** The game's suspend slot (`resume.state`) and battery save are written and flushed to the card before anything else (`App::doze`). If the write fails, it is retried once. A game that still can't be saved is marked unsaved (see below).
2. **Settle.** Drawing stops as in 0.3.0. After a 400 ms pause, so a quick flick of the lid never reaches the kernel, slot.advance arms a wake alarm and suspends.
3. **Suspend to RAM.** slot.advance writes `mem` to `/sys/power/state` (`crates/slot-power/src/sleep.rs`). This is suspend-to-RAM, not the lighter `freeze` (suspend-to-idle). The kernel freezes all processes, suspends device drivers (the display among them), takes the secondary CPU cores offline and hands the last one to the platform firmware. Memory stays powered. Which power rails stay on is decided by the firmware, not Linux.
4. **Wake sources.**
   - **Lid:** the power chip's driver has an `os_sleep` setting. BaseOS writes `16` ("Super Standby") at every boot, in which the lid is not a wake source. slot.advance writes `0` ("Normal Standby") at startup, which makes the hall sensor a wake source through the power-key device. It refuses to suspend if the lid isn't armed.
   - **Timer:** the SoC's real-time clock (`sunxi-rtc`, found by name, never by number) with a relative alarm. The deadline is counted on the kernel's uptime clock, which keeps counting through suspend, so it always runs from the lid closing.
   - **`POWER`**, and incidental events from the power chip such as a charger.
5. **Classify every wake** (`crates/slot/src/sleep.rs`). Lid open: carry on playing, nothing reloaded. Deadline reached: shut down cleanly; the next boot restores the game from `resume.state`. Anything else (`POWER` with the lid shut, a charger, a stray event): back to sleep with the screen dark and the same deadline. More than 20 unexpected wakes in one sleep, or a suspend that fails twice, falls back to 0.3.0's software standby for that sleep.

**Unused hardware** (`crates/slot-power/src/quiet.rs`). At startup, slot.advance switches off the six USB host controllers `ohci1-3` and `ehci1-3` through their sysfs switches. The RG SP (2026)'s USB-C port uses a separate controller (`5100000.udc`, with `ehci0`/`ohci0`) and is not touched. Once BaseOS has finished starting Wi-Fi (uptime 20 s), slot.advance unloads the `8821cs` driver, which cuts the radio's supply rail, unless `wifi=on` is set. Both are logged and optional, hardware sleep never depends on either, and both last only until the next boot. They stay off during gameplay too; no gameplay battery effect has been measured.

**Battery cap.** Sleep is planned to aim to preserve approximately 10% of the battery. It assumes a deliberately pessimistic 100 mA, so the device wakes and shuts down early if the chosen time would go below about 10%. This is a plan based on the gauge, not a guarantee. At 10% or less it sleeps for 3 minutes, as 0.3.0 did. The 5% shutdown from slot still applies.

**Save safety.** A game that could not be written when the lid closed is never shut down by the timer. Instead, the device keeps sleeping and retries the save at each deadline, and only the 5% battery shutdown can end it. This now includes the case where the emulator core fails to provide its state. Before commit `dbbadae`, that case was wrongly reported as saved. A core that is still loading, or failed to start, has no progress to lose and is treated as saved.

### The lid-wake trade-off

Experiment 5 compared the same hardware with lid wake armed (Normal Standby, segment 3, ~70 mA) and with the firmware's deeper Super Standby (segment 4, ~64 mA, lid not armed). Lid wake costs roughly 7 to 9 mA, about 10%. slot.advance keeps it, because waking by opening the lid is the point of sleeping instead of switching off.

---

## Battery results

<p align="center">
<img src="../.github/resources/battery/battery-current-draw.svg" width="100%" alt="Estimated lid-closed current draw from battery-gauge readings: about 250 mA for 0.3.0 software standby held awake by a test build, about 105 mA for the first suspend-to-RAM test, and about 70 mA for the 0.4.0 configuration, about 72% less">
</p>

<sub>Bars are estimates calculated from battery-gauge readings. Whiskers show the gauge's 1% resolution over each run (tick-timed for 0.3.0). One run per configuration.</sub>

### Battery used while asleep

<p align="center">
<img src="../.github/resources/battery/battery-drain-projection.svg" width="100%" alt="Battery used while asleep on 0.4.0, projected at 2 to 2.6% an hour: 4 to 5.2% at 2 hours, 8 to 10.4% at 4 hours and 16 to 21% at 8 hours, with three observed points: 16% over 8 hours on 0.4.0-beta.3, 20% over 7 hours 46 minutes on 0.4.0, and 3% over 1.5 hours in Experiment 5">
</p>

<sub>The dashed lines and shaded band are projections at 2 and 2.6% an hour, the rates of the two overnight sleeps. The white points were measured: 16% (98% to 82%) over 8 hours on 0.4.0-beta.3, 20% (94% to 74%) over 7 hours 46 minutes on 0.4.0, and 3% over 1.5 hours in Experiment 5. Real use varies with battery age, temperature and charge level.</sub>

| Asleep for | Battery used | Basis |
|---|---|---|
| 15 min | ~0.5 to 0.7% | Projected |
| 30 min | ~1 to 1.3% | Projected |
| 1 hr | ~2 to 2.6% | Projected |
| 1.5 hr | 3% (77% to 74%) | Measured once, Experiment 5 |
| 2 hr | ~4 to 5.2% | Projected |
| 4 hr | ~8 to 10.4% | Projected |
| 7 h 46 min | 20% (94% to 74%) | Measured once, 0.4.0 |
| 8 hr | 16% (98% to 82%) | Measured once, 0.4.0-beta.3 |

Projected rows are worked out from the two overnight rates, 2 and 2.6% an hour. Measured rows are single runs read in whole percent.

---

## Methodology and uncertainty

**Instrument.** The only measurement available on the device is the battery gauge. Its `charge_counter` is just the battery percentage multiplied by the 3,500 mAh design capacity: every reading is a multiple of 35,000 µAh. So the real resolution is one percent, **35 mAh**.

**Test B (0.3.0 software standby).** This used a test build of 0.3.0 whose only change was a 3-hour standby timeout instead of 3 minutes, so standby could be held for 2 hours. With the lid closed, a logger recorded the gauge every minute. From the start and end readings alone, the figure is 245 ± 17.5 mA. Because each 1% tick can be timed to within a minute, the 13 full steps between the first and last tick pin it down to **248 to 253 mA**. The ticks came every 7 to 9 minutes throughout, so the drain was steady.

**Test A (first suspend-to-RAM).** One `mem` suspend lasted 2 hours, with lid wake armed and USB and Wi-Fi on. The device was awake for only 2.35 s of the 7,201 s interval. The battery fell from 68% to 62%. Each end reading could be anywhere within its 1% band, so the result is **105 ± 17.5 mA**. *The raw report prints ±2.5 mA. That assumed 5 mAh steps, before Test B showed the step is 35 mAh, and it is wrong.*

**Experiment 5 (where the current goes).** This was one overnight run of four 90-minute suspends, each adding one change. Each segment ran from one 1% tick to the first tick after waking. The device then stayed awake until the next tick, to measure its awake current for the time it spent awake inside the segment.

| Segment | Configuration | Battery while asleep | Raw estimate (35 mAh steps) | Assuming 240 / 250 / 260 mA awake |
|---|---|---|---|---|
| 1 | Normal Standby, lid armed, USB and Wi-Fi on | 89% to 84% | 130.9 mA | 94 / 98 / 102 mA |
| 2 | + USB host controllers off | 82% to 79% | 77.2 mA | 66 / 69 / 71 mA |
| 3 | + Wi-Fi off (**the 0.4.0 configuration**) | 77% to 74% | 69.8 mA | 70 / 73 / 75 mA |
| 4 | Super Standby, lid not armed | 72% to 70% | 62.6 mA | 61 / 64 / 66 mA |

**The gauge isn't linear at high charge.** Above about 80%, one percent step worked out to about 26 mAh instead of 35. That is why segment 1's raw figure, 131 mA, is far above Test A's 105 mA for the same configuration. The right-hand column sidesteps step size: within a segment, the asleep and awake readings come from the same part of the gauge, so their ratio doesn't depend on the step. That ratio is then scaled by an assumed awake current (Test B measured 250 mA). Below 80%, the raw and corrected figures agree. Segment 3, at 77% to 73%, gives 69.8 mA raw and 70 to 75 mA corrected. *The raw report's ±0.5 mA figures ignore this effect and are too optimistic.*

**What the comparisons support.**
- The two runs of the baseline configuration agree within their uncertainty: Test A at 105 ± 17.5 mA, and segment 1 at about 98 mA corrected.
- USB off (segment 1 to 2) gave the largest single saving, roughly 30 mA. This rests on one run, and its exact size is mixed up with the high-charge gauge error.
- Wi-Fi off showed no saving that could be separated from noise while asleep.
- The direction of the overall improvement is confirmed by two independent comparisons (Test A against segment 3, and segment 1 against segments 2 and 3). The size, about 105 to about 70 mA, rests on single runs.

**The overnight sleeps.** Two overnight sleeps used 0.4.0's sleep configuration (0.4.0 only added the About screen to beta.3), each as a single continuous suspend:

| | 0.4.0-beta.3 | 0.4.0 (night A) |
|---|---|---|
| Battery | 98% to 82% | 94% to 74% |
| Time asleep | 8 hours | 7 h 46 min (27,987 s) |
| Observed rate | 2.0% an hour (1.88 to 2.12 within whole-percent readings) | 2.6% an hour (2.44 to 2.70) |
| End reading | Next morning, after the 8-hour timer had shut the device down | Straight after a lid wake, by the validation watcher |
| Record | Tester's notes; no log | [`overnight-0.4.0-night-a.txt`](battery-tests/overnight-0.4.0-night-a.txt) |

The range shown on this page, 2 to 2.6% an hour, is these two observations. It rests on two runs and is not a statistical interval.

**Why a percentage drop isn't a current at high charge.** The gauge reports whole percent, and its steps are not all 35 mAh. Experiment 5's awake ticks, at about 250 mA, put one step at about **26 mAh at 83%**, **31 at 78%**, **36 at 73%** and **36 at 69%**. Above about 85% no step has been measured. Both overnight sleeps started there.

**The two nights don't confirm the same current.** Applying the one estimated high-charge step (26.3 mAh above 80%, 35 below) as an illustration gives about **53 mA** for beta.3 and **74 mA** for night A. These don't agree, and the evidence can't say why. Three things differ and none can be ruled out:
- the step sizes above 85% are unknown;
- the end readings were taken differently (after a timer shutdown and a power-on, against straight after a wake);
- before night A, the gauge fell from 99% to 94% in 22 minutes awake, which at 250 to 300 mA would be only about 20 mAh a step if the gauge is right.

So neither overnight sleep is converted to a current. The current-draw chart and the 72% reduction keep Experiment 5's ~70 mA, measured below 80% where the step sizes were estimated. Both nights are reported as the percentages the gauge showed, because that is what a player sees. `calculate.py` prints the full working.

---

## Raw reports and calculations

| File | What it is |
|---|---|
| [`battery-tests/test-b-software-standby.txt`](battery-tests/test-b-software-standby.txt) | Test B, 10 October 2026 (UTC): 0.3.0 software standby held awake for 2 hours, with the minute-by-minute log |
| [`battery-tests/test-a-suspend-usb-wifi-on.txt`](battery-tests/test-a-suspend-usb-wifi-on.txt) | Test A, 10 October 2026 (UTC): one 2-hour `mem` suspend with lid wake, USB and Wi-Fi on |
| [`battery-tests/experiment-5-suspend-segments.txt`](battery-tests/experiment-5-suspend-segments.txt) | Experiment 5, 11 October 2026 (UTC): four 90-minute suspends, with USB, Wi-Fi and PE19 states logged around every sleep |
| (no file) | The 8-hour overnight sleep on 0.4.0-beta.3: battery percentages noted by the tester (98% and 82%), recorded in `calculate.py` |
| [`battery-tests/overnight-0.4.0-night-a.txt`](battery-tests/overnight-0.4.0-night-a.txt) | Night A, 15 to 16 October 2026: a 7 h 46 min overnight sleep on the 0.4.0 release, 94% to 74%, with the validation watcher's report and `slot.log` |
| [`battery-tests/calculate.py`](battery-tests/calculate.py) | Recalculates every figure on this page from the reports and the two overnight sleeps |
| [`battery-tests/charts.py`](battery-tests/charts.py) | Regenerates the three battery charts |

The reports are unedited device output. Their own summary lines contain the two errors noted above (Test A's ±2.5 mA and Experiment 5's ±0.5 mA). `calculate.py` shows the corrected working. Run it from the repository root:

```
python3 documentation/battery-tests/calculate.py
```

The earlier experiments that established suspend, lid wake, timed wake and `POWER` wake on the RG SP (2026), Experiments 1 to 4, are not included here; they took no battery measurements.

---

## Validation status

**Shown on the RG SP (2026) with development scripts, before 0.4.0 was built:**
- `mem` suspend: about 15 cycles across Experiments 2 to 5 and Test A, with no failures.
- Lid wake, with the screen back in under a second, from the carousel.
- Timed wake accurate to 1 to 2 seconds, over sleeps of 30 seconds to 2 hours, with no unexpected wakes during long sleeps.
- The USB controllers and Wi-Fi stay off through suspend and resume.
- The SD card reads and writes straight after resume.
- `POWER` wake with the lid shut (Experiment 4). The device woke early as expected, but the script could not confirm the cause from its interrupt counts.

**Shown on the device with 0.4.0-beta.1 (`slot.log`, 12 October 2026):**
- Sleep and resume from inside a running game: five sleeps of 13 seconds to 27 minutes, each resuming straight into the game, with sound reported normal.
- A timed shutdown: with Sleep at `3 min`, the device woke at 179 seconds and shut down cleanly, and the next session loaded the game at boot.
- Rapid lid flicks: the kernel's refusals to suspend mid-flick were handled, but after some flicks the screen stayed dark for 4 to 5 seconds with the lid open. The lid driver had recorded "shut" after the wake. 0.4.0-beta.3 checks the sensor's raw level in that case (see `crates/slot-power/src/lid.rs`).

**Checked on the device with 0.4.0-beta.3 (13 October 2026):**
- From `slot.log`: repeated sleeps and resumes from inside a game, including quick lid flicks, with each slow lid open resuming the game.
- From `slot.log`: a timed shutdown. With Sleep at `3 min`, the device woke at 179 seconds with the lid shut and shut down cleanly; the game was restored at the next power-on.
- From `slot.log`: the lid sensor's level was learnt at start-up after a cold boot (`lid: sensor reads hi with the lid open`), and the beta.1 problem (the lid driver recording "shut" after a wake with the lid open) did not recur: `slot.log` never had to overrule the driver.
- Reported by the tester, from sessions whose logs had been replaced: `POWER` pressed with the lid shut, and the charger plugged in and unplugged while asleep, both left the screen dark until the lid was opened.
- Rumble, headphones and all controls after several resumes were confirmed on 0.4.0-beta.1, and nothing they rely on changed afterwards.

**Known limitation.** Rapidly flicking the lid closed and open can occasionally leave the screen dark with the lid open. Deliberate stress testing reproduced this in both 0.4.0 and 0.4.1. Normal wake behaviour is unaffected, and closing and reopening the lid restores the game without losing progress. In the case examined in detail, the device woke and read the lid as shut (from both the driver and the raw sensor), so it went back to sleep while the lid came to rest open; the lid was most likely still moving when it was read. With no further lid movement to wake it, the screen stayed dark until the lid was closed and opened again. It was accepted for 0.4.0 rather than fixed, because it needs deliberate rapid flicking and recovery is simply closing and opening the lid. The 0.4.1 candidate does not change lid handling, and stress testing on it showed the same behaviour.

**Battery:** an 8-hour overnight sleep on 0.4.0-beta.3 went from 98% to 82%, about 2% an hour. A later overnight sleep on the 0.4.0 release went from 94% to 74% in 7 hours 46 minutes, about 2.6% an hour (see [Methodology and uncertainty](#methodology-and-uncertainty)).

0.4.0 was released on the strength of these checks, built from the same code as 0.4.0-beta.3 plus a new About screen, which was checked on the device before release.
