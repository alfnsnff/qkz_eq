# Register map

Observed on the QKZ TC MAX, firmware `CDSV001.003`. The same chip (KT Micro KT02H20) and register map ship as the JCally JM12 and Kiwi Ears Allegro Mini; facts marked *(Contour)* were measured on those units by the [Contour](https://github.com/Chronosauros/contour) project. All registers are 32-bit. **Confirmed** means a write was tested on the QKZ and had the expected effect.

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
| `0x24` | DAC EQ switch, byte 0: `3` on, `2` off *(Contour)*; bytes 1–3 are device status | observed `3` |
| `0x26–0x2F` | DAC EQ bands 1–5 | confirmed |

Each band is two registers, band *n* at `0x26 + 2n` (DAC):

| Register | Bytes 3–2 | Bytes 1–0 |
|---|---|---|
| A | frequency register (u16) | gain × 10 dB (i16) |
| B | byte 3 reserved, byte 2 filter type | Q × 1000 (u16) |

**Frequency is stored at half its audible value.** The DSP designs its biquads for 24 kHz but runs them at the 48 kHz stream rate, so a band plays at `register × 2` *(Contour, measured)*. At other stream rates the factor is `rate / 24000`.

Filter types *(Contour)*: `0` peak, `3` low shelf, `4` high shelf. Shelf Q is the register Q.

Example: `0x13880000` / `0x000002C3` = register 5000 → **10 kHz**, 0.0 dB, peak, Q 0.707. The factory bands are 2, 4, 10, 16 and 20 kHz.

## Gain

| Address | Name | Default | Status |
|---|---|---|---|
| `0x3A` | unknown | `3` | — |
| `0x3B` | Analog DAC gain ("Global Gain"): steps 1–15 = −18…+3 dB in 1.5 dB, 13 = 0 dB *(Contour)* | `13` | not written by qkz_eq |
| `0x65` | Digital ADC gain | `0x100` | unknown scale |
| `0x66` | Digital DAC gain | `0x200` | confirmed as output volume on the QKZ |

Notes on `0x66`: on the QKZ, lowering it to `0x100` / `0x080` audibly reduces volume with no other change. Contour measured **no** level change from it on the JM12, so behavior may differ by firmware. With a +12 dB band at full-scale input no clipping was heard, so the EQ appears to have internal headroom.

## Unknown

`0x64`, `0x67–0x69`, `0x71–0x73`, `0x78–0x7A` hold non-zero values whose purpose is unknown. `0xE1 = 0x12345678` looks like a test pattern.
