# Register map

Observed on the QKZ TC MAX, firmware `CDSV001.003`. All registers are 32-bit. **Confirmed** means a write was tested and had the expected audible effect.

## Identity (read-only ASCII)

Strings are stored little-endian, 4 characters per register, NUL-terminated.

| Address | Content (example) |
|---|---|
| `0x00` | `REG:` (magic) |
| `0x04–0x06` | Firmware version, `CDSV001.003` |
| `0x08–0x0A` | Build date, `Nov 14 2025` |
| `0x0C–0x0D` | Build time, `17:13:12` |
| `0x10–0x11` | Build hash, `17c0c7e` |
| `0x36–0x37` | `PerfCfg:` (section marker) |
| `0x40–0x41` | Manufacturer, `KTMicro` |
| `0x48–0x4B` | Product, `CDS.KT USB Audio` |
| `0x50–0x56` | Serial / ID string |
| `0x5B` | `PID << 16 \| VID` (`0x011131B2`) |

## EQ

Two identical 5-band blocks exist: ADC (recording path) and DAC (playback path).

| Address | Name | Status |
|---|---|---|
| `0x18` | ADC EQ enable | observed `3` |
| `0x1A–0x23` | ADC EQ bands 1–5 | observed |
| `0x24` | DAC EQ enable (`3` on, `0` off) | observed `3` |
| `0x26–0x2F` | DAC EQ bands 1–5 | confirmed |

Each band is two registers, band *n* at `0x26 + 2n` (DAC):

| Register | Bits 31–16 | Bits 15–0 |
|---|---|---|
| A | frequency in Hz (u16) | gain × 10 dB (i16) |
| B | filter type (u16) | Q × 1000 (u16) |

Example: `0x13880000` / `0x000002C3` = 5000 Hz, 0.0 dB, type 0, Q 0.707.

Filter type `0` behaves as a peaking filter. Other values are untested.

## Gain

| Address | Name | Default | Status |
|---|---|---|---|
| `0x3A` | PGA L | `3` | unknown scale |
| `0x3B` | PGA R | `13` | unknown scale |
| `0x65` | Digital ADC gain | `0x100` | unknown scale |
| `0x66` | Digital DAC gain | `0x200` | experimental (used as preamp) |

## Unknown

`0x64`, `0x67–0x69`, `0x71–0x73`, `0x78–0x7A` hold non-zero values whose purpose is unknown (limiter/AGC parameters are a guess). `0xE1 = 0x12345678` looks like a test pattern.
