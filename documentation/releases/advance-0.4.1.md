slot.advance 0.4.1 for the Anbernic RG SP (2026) (720 x 480). It is not for the RG35XX SP. The previous release is [slot.advance 0.4.0](https://github.com/suprsomething/slot.advance/releases/tag/advance-0.4.0).

**Upgrading:** download `slot.advance-0.4.1-rg-sp.zip`, unzip it, and copy the contents of its `System` folder into `System` on your games card (SD2), as in the README's [upgrade steps](https://github.com/suprsomething/slot.advance/blob/advance-0.4.1/README.md#installing-and-updating). Your games, saves, states and settings are not touched. **New install:** follow the README's [new install steps](https://github.com/suprsomething/slot.advance/blob/advance-0.4.1/README.md#new-install).

### 35% less resume time, with Wi-Fi off
With Wi-Fi off (the default), the game's first frame now appears a median of **0.81 s** after the kernel's first logged resume marker, against **1.25 s** in 0.4.0: 443 ms sooner, 35.4% less measured time. The time runs from that marker to the first rendered frame (11 wakes on 0.4.1-beta.1, which has the same runtime source as 0.4.1 apart from the version; 10 on 0.4.0). It leaves out the unlogged moment the lid starts to open, and it isn't the kernel's total device-resume time.

- **Why it was slower:** switching Wi-Fi off unloads its driver and cuts power to the radio, but the kernel kept the radio's SDIO card registered, and every wake waited about a second for it to time out.
- **What changed:** once, at start-up, straight after unloading the Wi-Fi driver, slot.advance removes that stale card, as BaseOS does before loading the driver.
- **What didn't:** the suspend itself, the lid and timer wakes, the radio's power state, and `wifi=on` (wakes there take about 0.82 s on both versions; no improvement is claimed).

### Validation
Checked on the device with 0.4.1-beta.1, which has the same code: first-frame timing passed; lid closes during the Wi-Fi shutdown completed, with the start-up race below accepted; the 0.4.0 sleep checklist completed; and a 7 hour 37 minute sleep passed for reliability, with no unexpected wakes and the radio unpowered throughout.

**Battery:** whether 0.4.1 uses the same standby battery as 0.4.0 is inconclusive. Its extended sleep went from 99% to 75% in 7 hours 37 minutes, against 94% to 74% in 7 hours 46 minutes on 0.4.0, but the two started at different charge levels and the gauge behaves differently near full. Across three extended sleeps on 0.4.x (0.4.0-beta.3, 0.4.0 and 0.4.1-beta.1), the gauge showed about 2 to 3.2% an hour, against about 7% for 0.3.0 (semi-stock) software standby. These are observed gauge readings, not a guaranteed range. [How it was measured](https://github.com/suprsomething/slot.advance/blob/advance-0.4.1/documentation/sleep-and-battery.md).

### Known limitations
- **Start-up race:** if the lid is closed at almost exactly the moment Wi-Fi switches off, about 20 seconds after power-on, the stale card may not be removed. Sleep and wake still work normally; that session's wakes simply take as long as in 0.4.0, and the next power-on removes it as usual.
- Rapidly flicking the lid closed and open can occasionally leave the screen dark with the lid open. Deliberate stress testing reproduced this in both 0.4.0 and 0.4.1. Normal wake behaviour is unaffected, and closing and reopening the lid restores the game without losing progress.

If anything goes wrong, copy `slot.log` and `slot.log.1` from the games card before powering on again, and [open a bug report](https://github.com/suprsomething/slot.advance/issues/new/choose) with both files attached.

Emulation by mGBA (MPL-2.0) and gpSP (GPL-2.0). Their licences, patches and gpSP's source are in `System/licenses/`. slot.advance is GPL-3.0, based on [slot](https://github.com/BrandonKowalski/slot) by Brandon T. Kowalski. [Full release notes](https://github.com/suprsomething/slot.advance/blob/advance-0.4.1/documentation/release-notes.md).
