slot.advance 0.4.0 for the Anbernic RG SP (2026) (720 x 480). It is not for the RG35XX SP. The previous release is [slot.advance 0.3.0](https://github.com/suprsomething/slot.advance/releases/tag/advance-v3).

**Upgrading:** download `slot.advance-0.4.0-rg-sp.zip`, unzip it, and copy the contents of its `System` folder into `System` on your games card (SD2), as in the README's [upgrade steps](https://github.com/suprsomething/slot.advance#installing-and-updating). Your games, saves, states and settings are not touched. **New install:** follow the README's [new install steps](https://github.com/suprsomething/slot.advance#new-install).

### New in 0.4.0
- **Real hardware sleep.** Closing the lid saves the game, then suspends the RG SP (2026) to RAM. Opening the lid carries on from the same moment, with nothing reloaded.
- **Settings > Sleep:** `3 min`, `15 min`, `30 min` (default), `1 hr`, `2 hr`, `8 hr`. When the time runs out with the lid shut, it shuts down cleanly and the next power-on restores the game.
- **Battery:** six unused USB host controllers and Wi-Fi are switched off. Link play needs `wifi=on` in `Config/slot.state`. Lid-closed standby current fell by about 72% (about 250 mA to about 70 mA). An 8-hour overnight sleep used 16% of the battery, about 2% an hour. [How it was measured](https://github.com/suprsomething/slot.advance/blob/main/documentation/sleep-and-battery.md).
- **Optional game titles:** Settings > Screen > Show Game Titles (off by default).
- **About screen** with the version, the number of games, free space on the games card, the battery, slotadvance.com and the credits.
- A game that could not be saved when the lid closed is never shut down by the sleep timer.
- `POWER` with the lid shut no longer lights the screen.
- Striped slot.advance boot logo, and the build's version and commit written as the first line of `slot.log`.

### Known limitation
Flicking the lid very quickly can occasionally leave the screen dark with the lid open. Closing and opening the lid again brings the game straight back, and nothing is lost.

If anything goes wrong, copy `slot.log` and `slot.log.1` from the games card before powering on again, and [open a bug report](https://github.com/suprsomething/slot.advance/issues/new/choose) with both files attached.

Emulation by mGBA (MPL-2.0) and gpSP (GPL-2.0). Their licences, patches and gpSP's source are in `System/licenses/`. slot.advance is GPL-3.0, based on [slot](https://github.com/BrandonKowalski/slot) by Brandon T. Kowalski. [Full release notes](https://github.com/suprsomething/slot.advance/blob/main/documentation/release-notes.md).
