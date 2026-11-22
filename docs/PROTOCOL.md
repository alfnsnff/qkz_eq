# HID protocol

Reverse-engineered from the QKZ TC MAX (`31B2:0111`, firmware `CDSV001.003`). Other KT Micro firmwares may differ.

## Transport

- **Interface:** USB HID interface number **3** (vendor control). The other interfaces are UAC audio and consumer-control keys.
- **Report ID:** `0x4B`
- **Length:** 11 bytes out, response read from the same interface (first 11 bytes are meaningful).
- **Byte order:** little-endian for addresses and values.

## Frame layout

| Byte | 0 | 1–4 | 5 | 6 | 7–10 |
|---|---|---|---|---|---|
| Field | report `0x4B` | address (LE32) | command | `0x00` | value / status (LE32) |

### Read — command `0x52` (`'R'`)

```
TX: 4B  aa aa aa aa  52  00  00 00 00 00
RX: 4B  aa aa aa aa  52  ??  vv vv vv vv
```

The register value is bytes 7–10 of the response. Match the response on report ID, command and echoed address; stale reports can arrive.

### Write — command `0x57` (`'W'`)

```
TX: 4B  aa aa aa aa  57  00  vv vv vv vv
RX: 4B  aa aa aa aa  57  ??  03 .. .. ..
```

The write succeeded if byte 7 of the response is `0x03`. Writes take effect immediately and are **not persisted** across a power cycle.

## Timing

Responses arrive within a few milliseconds. `qkz_eq` waits up to 500 ms per request before reporting a timeout.

## Unknowns

- The command that commits settings to flash (not needed, not used).
- Meaning of response byte 6.
