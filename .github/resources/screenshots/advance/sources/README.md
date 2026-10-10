# README screenshot sources

`pokemon-unbound-label.png` and `pokemon-unbound-hall-of-fame.png` were supplied by the repository owner for the README. The label is fan artwork for Pokémon Unbound; it is separate from the original commercial cartridge-label starter pack. No ROM or save-state data is included.

Regenerate all five README screenshots with:

```sh
cargo run -p slot --no-default-features --example readme_screenshots
```

This uses Slot’s own cartridge and save-state drawing code at 720 × 480, including the normal and delete-confirmation overlays. A working graphics context is required.
