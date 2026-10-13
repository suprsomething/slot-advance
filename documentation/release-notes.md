# slot.advance release notes

These are the releases of **slot.advance**, the fork for the Anbernic RG SP (2026). Version numbers here are slot.advance's own. They are separate from upstream slot's version numbers, which are in [`CHANGELOG.md`](../CHANGELOG.md) (inherited from upstream and left unchanged).

Releases are tagged with an `advance-` prefix, such as `advance-0.4.0`, so they don't clash with upstream slot's tags in this repository.

The release previously named v3 is now displayed as 0.3.0; its contents are unchanged.

---

## 0.4.0 (current release)

Tag `advance-0.4.0`. Download: `slot.advance-0.4.0-rg-sp.zip` from the [release page](https://github.com/suprsomething/slot.advance/releases/tag/advance-0.4.0). [Report problems](https://github.com/suprsomething/slot.advance/issues/new/choose), attaching `slot.log` and `slot.log.1` if you can.

0.4.0 is 0.4.0-beta.3's code with a new About screen. Everything new since 0.3.0 is listed under the betas below.

**New since 0.4.0-beta.3**
- **About screen:** it now shows the slot.advance logo, the version, the number of games, free space on the games card, the battery, slotadvance.com and the credits. It replaces upstream slot's "back of the console" sticker.
- Downloads are now named `slot.advance-<version>-rg-sp.zip`.

**Checked on the device** with 0.4.0-beta.3 and this About screen: sleep and resume from inside a game, quick lid flicks, `POWER` and the charger while asleep, the timed shutdown with the game restored at the next power-on, and the first sleep after a cold boot. See [Validation status](sleep-and-battery.md#validation-status).

**Known limitation**
- Flicking the lid very quickly can occasionally leave the screen dark with the lid open. Closing and opening the lid again brings the game straight back, and nothing is lost.

**Battery:** an 8-hour overnight sleep on 0.4.0-beta.3 went from 98% to 82%, about 2% an hour, matching the earlier experiment.

---

## 0.4.0-beta.3 (beta, superseded by 0.4.0)

Tag `advance-0.4.0-beta.3`. Download: `slot-advance-0.4.0-beta.3-rg-sp.zip` from the [release page](https://github.com/suprsomething/slot.advance/releases/tag/advance-0.4.0-beta.3). It has been superseded by 0.4.0, which is the one to download.

**Fixed since 0.4.0-beta.2**
- After rapid lid flicks, opening the lid could leave the screen dark for 4 to 5 seconds before the game came back. The lid driver sometimes records "shut" after a wake even though the lid is open, so slot.advance put the device back to sleep. It now checks the lid sensor's raw level whenever the driver says "shut", and the raw level wins. It learns which level means "open" at start-up. `slot.log` notes each time it overrules the driver.

**Shown on the device**
- From 0.4.0-beta.1's logs: sleep and resume from inside a game, and the timed shutdown with the game restored at the next boot.

---

## 0.4.0-beta.2 (beta, superseded by 0.4.0-beta.3)

Tag `advance-0.4.0-beta.2`. Download: `slot-advance-0.4.0-beta.2-rg-sp.zip` from the [release page](https://github.com/suprsomething/slot.advance/releases/tag/advance-0.4.0-beta.2). It was the first published beta and has been superseded by 0.4.0-beta.3, which is the one to download.

**Fixed since 0.4.0-beta.1**
- A running game whose emulator core failed to provide its state when the lid closed was counted as saved, so the sleep timer could shut down and lose progress since the last autosave. It is now treated as unsaved: the device keeps sleeping and retries the save instead of shutting down (commit `dbbadae`).

**Known issue** (fixed in beta.3)
- After rapid lid flicks, opening the lid could leave the screen dark for a few seconds.

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
- In development testing, the lid-closed current fell from about 250 mA (0.3.0 software standby, held awake by a test build) to about 70 mA, around 72% less. An 8-hour overnight sleep on 0.4.0-beta.3 later confirmed the rate: 98% to 82%, about 2% an hour. See [Sleep and battery](sleep-and-battery.md).

**Carousel**
- Optional game titles: **Settings > Screen > Show Game Titles** (off by default) shows the selected game's title above its cartridge.

**Other**
- Striped slot.advance boot logo and README artwork.
- About screen text for slot.advance, with full credits in the README.
- Semantic versioning. The build's version and Git commit are written as the first line of `slot.log`.

---

## 0.3.0 (previous release, previously named v3)

Tag `advance-v3`, published 10 October 2026. Download: `slot-advance-v3-rg-sp.zip` (the filename is unchanged).

- Each game remembers its own shader, colour correction, fast-forward speed and fast-forward sound (`Config/game_profiles.txt`).
- `SELECT` + `L2` / `R2` sets the game's fast-forward speed; `SELECT` + `X` turns its fast-forward sound on or off.
- After a reboot, the carousel opens on the game played last, in the category it was started from.
- Quiet low-battery banners at 15% and 10%; the 5% shutdown is unchanged.
- Software standby: the screen stops being redrawn while the lid is shut or after a `POWER` tap. It still powers off after 3 minutes.
- slot. ADVANCE boot logo, shown from the second power-on.
- Also included: Recents, Favourites, five save states per game with an Up / Down switcher, and delete that asks first.

Emulation by mGBA (MPL-2.0) and gpSP (GPL-2.0), unchanged from upstream slot v1.5.0.
