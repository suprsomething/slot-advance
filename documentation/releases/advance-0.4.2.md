slot.advance 0.4.2 for the Anbernic RG SP (2026) (720 x 480). It is not for the RG35XX SP. The previous release is [slot.advance 0.4.1](https://github.com/suprsomething/slot.advance/releases/tag/advance-0.4.1).

**Upgrading:** download `slot.advance-0.4.2-rg-sp.zip`, unzip it, and copy the contents of its `System` folder into `System` on your games card (SD2), as in the README's [upgrade steps](https://github.com/suprsomething/slot.advance/blob/advance-0.4.2/README.md#installing-and-updating). Your games, saves, states and settings are not touched. **New install:** follow the README's [new install steps](https://github.com/suprsomething/slot.advance/blob/advance-0.4.2/README.md#new-install).

A maintenance release of fixes from upstream slot, each checked against slot.advance and adapted where needed. Sleep, wake, the lid, the sleep timer, Wi-Fi and power management are unchanged from 0.4.1.

### Save states you can rely on when the clock is wrong
If the clock had gone backwards (set back in Set Clock, or reset after the battery ran flat), a new save state could vanish: with all five slots full, it was saved, "saved" was shown, and it was deleted straight away to make room. A new state is now always placed after the newest one, so it is first in the switcher and the oldest is the one replaced.

### Rewind does about half the work
mGBA was asked to build every state twice, and rewind takes one every second frame. The size is now remembered while a game runs (0.73 ms down to 0.39 ms per state, timed on a computer). States saved to the card are still exactly the size mGBA reports.

### Steady frame rate when muted
After 3 seconds of silence slot lets go of the speaker, and its timing then ran slightly slow, which could hold the game back. It now keeps time to a deadline. On the RG SP, a minute of muted play showed no stutter and the sound came back straight away each time.

### Crash diagnostics
If slot.advance itself crashes, the RG SP starts it again within a few seconds, and the details are now kept: a `crash` folder on the games card holds a log line for each crash, the log of each of the last five crashed sessions, and the latest crash's core and memory map. `slot.log` also gets a line naming the signal, the thread and where it happened. Before, a crash left almost nothing to go on. `slot.log` now also times each step of a game launch.

### gpSP stability
gpSP could fail to start if memory near it was crowded at boot. Its code cache is now part of the core itself, so it is always there.

### Validation
Checked on the device with 0.4.2-beta.1, which has the same code apart from the version and the crash line's thread label: save states (including with the clock a day behind), rewind, fast-forward, muted play, gpSP, three lid closes, a timed shutdown and restore, and a deliberate crash with automatic restart. [Details](https://github.com/suprsomething/slot.advance/blob/advance-0.4.2/documentation/release-notes.md).

If anything goes wrong, copy `slot.log` and `slot.log.1` (and, if there is one, the `crash` folder's `cores.log` and newest `slot-….log`) from the games card before powering on again, and [open a bug report](https://github.com/suprsomething/slot.advance/issues/new/choose) with the files attached.

Emulation by mGBA (MPL-2.0) and gpSP (GPL-2.0). Their licences, patches and gpSP's source are in `System/licenses/`. slot.advance is GPL-3.0, based on [slot](https://github.com/BrandonKowalski/slot) by Brandon T. Kowalski. [Full release notes](https://github.com/suprsomething/slot.advance/blob/advance-0.4.2/documentation/release-notes.md).
