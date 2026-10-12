**Pre-release for testers.** slot advance 0.4.0-beta.2 for the Anbernic RG SP (2026) (720 x 480). It is not for the RG35XX SP. The current stable release is still [slot advance 0.3.0](https://github.com/suprsomething/slot-advance/releases/tag/advance-v3); this beta becomes 0.4.0 once it has been validated on the device.

Download `slot-advance-0.4.0-beta.2-rg-sp.zip`, unzip it, and copy the contents of its `System` folder into `System` on your games card (SD2), as in the README's [upgrade steps](https://github.com/suprsomething/slot-advance#installing-and-updating). Your games, saves, states and settings are not touched. To go back, copy `System/slot` from the 0.3.0 download over it.

### New in 0.4.0
- **Real hardware sleep.** Closing the lid saves the game, then suspends the RG SP (2026) to RAM. Opening the lid carries on from the same moment, with nothing reloaded.
- **Settings > Sleep:** `3 min`, `15 min`, `30 min` (default), `1 hr`, `2 hr`, `8 hr`. When the time runs out with the lid shut, it shuts down cleanly and the next power-on restores the game.
- **Battery:** six unused USB host controllers and Wi-Fi are switched off. Link play needs `wifi=on` in `Config/slot.state`. In preliminary development testing, lid-closed standby current fell by about 72% (about 250 mA to about 70 mA). That figure comes from an experiment reproducing this configuration, not a battery test of this build. [How it was measured](https://github.com/suprsomething/slot-advance/blob/main/documentation/sleep-and-battery.md).
- **Optional game titles:** Settings > Screen > Show Game Titles (off by default).
- Striped slot advance boot logo, and the build's version and commit written as the first line of `slot.log`.

### Fixed since 0.4.0-beta.1 (not published)
- A running game whose emulator core failed to provide its state when the lid closed was counted as saved, so the sleep timer could shut down and lose progress. It is now treated as unsaved, and the device keeps sleeping and retries the save.

### Still being validated
Repeated sleep and resume from inside a game (with sound and controls afterwards), plugging in or unplugging the charger while asleep, and a timed shutdown followed by the game restoring at boot. A possible glitch when opening and closing the lid rapidly was reported on beta.1 and is being investigated. If anything goes wrong, copy `slot.log` and `slot.log.1` from the games card before powering on again.

Emulation by mGBA (MPL-2.0) and gpSP (GPL-2.0). Their licences, patches and gpSP's source are in `System/licenses/`. slot advance is GPL-3.0, based on [slot](https://github.com/BrandonKowalski/slot) by Brandon T. Kowalski. [Full release notes](https://github.com/suprsomething/slot-advance/blob/main/documentation/release-notes.md).
