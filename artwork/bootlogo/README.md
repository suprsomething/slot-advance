# Boot logo

slot shows its boot logo through the device's own bootloader, not through slot's drawing code. Each time slot starts, it reads `System/bootlogo.bmp` from the games card (or `Config/bootlogo.bmp`, if you put your own there), centres it on a screen-sized canvas filled with the image's top-left colour, and writes the result to the `boot-resource` partition. The bootloader shows that picture at the next power-on. The first time, slot keeps the stock picture as `bootlogo.baseos.bmp` on that partition. This is all in `crates/slot/src/bootlogo.rs`, which slot.advance leaves unchanged.

| File | What it is |
|---|---|
| `slot-advance-logo-source.png` | The striped slot.advance design as supplied (1536 x 1024, eight flat colours). |
| `slot-advance-bootlogo-720x480.png` / `.bmp` | The full RG SP (2026) screen as the bootloader will show it: 720 x 480, black, logo centred, at the design's own scale. |
| `slot-original-bootlogo.bmp` | Backup of upstream slot's `card/System/bootlogo.bmp`. |

`card/System/bootlogo.bmp` is the striped slot.advance logo cut to its own size (560 x 280, 24-bit BMP on black) so that slot's existing code places it. On a 720 x 480 screen the result is identical, pixel for pixel, to `slot-advance-bootlogo-720x480.png`. Cut to size, it also still fits every other panel BaseOS supports.

To go back to slot's logo, copy `slot-original-bootlogo.bmp` over `System/bootlogo.bmp` on the card and start slot once.
