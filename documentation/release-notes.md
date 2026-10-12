# slot advance release notes

These are the releases of **slot advance**, the fork for the Anbernic RG SP (2026). Version numbers here are slot advance's own. They are separate from upstream slot's version numbers, which are in [`CHANGELOG.md`](../CHANGELOG.md) (inherited from upstream and left unchanged).

Releases are tagged with an `advance-` prefix, such as `advance-0.4.0`, so they don't clash with upstream slot's tags in this repository.

The release previously named v3 is now displayed as 0.3.0; its contents are unchanged.

---

## 0.4.0-beta.2 (beta, the latest release)

Tag `advance-0.4.0-beta.2`. Download: `slot-advance-0.4.0-beta.2-rg-sp.zip` from the [release page](https://github.com/suprsomething/slot-advance/releases/tag/advance-0.4.0-beta.2). It is published as the latest release, labelled beta; it becomes `0.4.0` once the checks under [Validation status](sleep-and-battery.md#validation-status) pass. [Report problems](https://github.com/suprsomething/slot-advance/issues/new/choose) with `slot.log` and `slot.log.1` attached.

**Fixed since 0.4.0-beta.1**
- A running game whose emulator core failed to provide its state when the lid closed was counted as saved, so the sleep timer could shut down and lose progress since the last autosave. It is now treated as unsaved: the device keeps sleeping and retries the save instead of shutting down (commit `dbbadae`).

**Known issue**
- A possible glitch when opening and closing the lid rapidly was reported on 0.4.0-beta.1 and is being investigated.

Everything below was introduced in 0.4.0-beta.1, which was only installed for on-device testing and never published.

**Sleep**
- Closing the lid (or tapping `POWER` with it open) saves the game, then suspends the RG SP (2026) to RAM. Opening the lid carries on from memory, with nothing reloaded.
- New setting **Settings > Sleep**: `3 min`, `15 min`, `30 min` (default), `1 hr`, `2 hr`, `8 hr`. When the time runs out, it shuts down cleanly, and the next power-on restores the game from its suspend slot.
- `POWER` with the lid shut no longer lights the screen. Chargers and other stray wakes go straight back to sleep.
- On a low battery, sleep is planned to aim to preserve approximately 10%. At 10% or less it sleeps for 3 minutes, as 0.3.0 did.
- A game that could not be written to the card when the lid closed is never shut down by the timer.
- If hardware sleep isn't available or fails, that sleep falls back to 0.3.0's software standby.

**Battery**
- Six USB host controllers the RG SP (2026) has no ports for are switched off at startup. The USB-C port is unaffected.
- Wi-Fi is switched off about 20 seconds after starting. Link play needs `wifi=on` in `Config/slot.state`, and the link shortcut shows **Link Off** without it.
- In preliminary development testing, the lid-closed current fell from about 250 mA (0.3.0 software standby, held awake by a test build) to about 70 mA, around 72% less. That figure comes from an experiment reproducing this configuration, not a battery test of this build. See [Sleep and battery](sleep-and-battery.md).

**Carousel**
- Optional game titles: **Settings > Screen > Show Game Titles** (off by default) shows the selected game's title above its cartridge.

**Other**
- Striped slot. advance boot logo and README artwork.
- About screen text for slot advance, with full credits in the README.
- Semantic versioning. The build's version and Git commit are written as the first line of `slot.log`.

---

## 0.3.0 (previously named v3)

Tag `advance-v3`, published 10 October 2026. Download: `slot-advance-v3-rg-sp.zip` (the filename is unchanged).

- Each game remembers its own shader, colour correction, fast-forward speed and fast-forward sound (`Config/game_profiles.txt`).
- `SELECT` + `L2` / `R2` sets the game's fast-forward speed; `SELECT` + `X` turns its fast-forward sound on or off.
- After a reboot, the carousel opens on the game played last, in the category it was started from.
- Quiet low-battery banners at 15% and 10%; the 5% shutdown is unchanged.
- Software standby: the screen stops being redrawn while the lid is shut or after a `POWER` tap. It still powers off after 3 minutes.
- slot. ADVANCE boot logo, shown from the second power-on.
- Also included: Recents, Favourites, five save states per game with an Up / Down switcher, and delete that asks first.

Emulation by mGBA (MPL-2.0) and gpSP (GPL-2.0), unchanged from upstream slot v1.5.0.
