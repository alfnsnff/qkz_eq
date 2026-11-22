<p align="center">
  <img src="assets/logo.svg" width="128" alt="qkz_eq logo">
</p>

<h1 align="center">qkz_eq</h1>

<p align="center">
  <a href="https://github.com/alfnsnff/qkz_eq/actions/workflows/ci.yml"><img src="https://github.com/alfnsnff/qkz_eq/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/alfnsnff/qkz_eq/releases/latest"><img src="https://img.shields.io/github/v/release/alfnsnff/qkz_eq" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
</p>

A small native desktop equalizer for the **QKZ TC MAX** USB-C DAC dongle and, potentially, other dongles built on **KT Micro** audio chips.

The vendor ships no PC software for this dongle. `qkz_eq` talks to the chip directly over its HID control interface, so you can shape the DAC's built-in 5-band parametric EQ without any system-wide audio driver or APO.

> [!WARNING]
> This is an independent, reverse-engineered project. It is not affiliated with QKZ or KT Micro. Writing registers is done at your own risk. Changes are live until you press **Simpan ke DAC**, which stores them in the dongle's flash.

## Features

- 5-band parametric EQ (frequency, gain, Q; peak and low/high shelf) running on the DAC itself
- Save to the dongle's flash, so the EQ follows it to phones and other PCs without this app
- Live frequency-response curve
- Read / Apply with per-band change tracking, or optional auto-apply on release
- Built-in presets plus your own, saved on the PC as JSON
- Output gain trim under *Lanjutan* (digital DAC gain register)
- Single native binary (Rust + [egui](https://github.com/emilk/egui)), no runtime needed

## Supported devices

| Device | VID:PID | Firmware | Status |
|---|---|---|---|
| QKZ TC MAX | `31B2:0111` | `CDSV001.003` | Tested |

The JCally JM12 and Kiwi Ears Allegro Mini report the same `31B2:0111` and register map, so they will likely work too (untested here). Other KT Micro dongles may use different PIDs. Use the [probe tools](tools/) to check yours and open an issue with the results.

## Install

### From source

Requires a [Rust toolchain](https://rustup.rs) (1.80+). On Windows you also need the MSVC build tools.

```bash
git clone https://github.com/alfnsnff/qkz_eq.git
cd qkz_eq
cargo run --release
```

On Linux, `hidapi` needs `libudev-dev` and a udev rule to access the device without root:

```bash
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="31b2", MODE="0666"' | sudo tee /etc/udev/rules.d/70-ktmicro.rules
```

## Usage

1. Plug in the dongle and start `qkz_eq`. It connects and reads the current EQ.
2. Drag the band sliders. The curve updates immediately, and changed bands are highlighted.
3. Press **Apply** (or enable **Auto apply**) to send the changes to the DAC.
4. Press **Simpan ke DAC** to keep them after unplugging. The dongle disconnects for 1–3 s while saving, then the app reads the settings back to confirm.
5. Save a setup under **Preset**. User presets live in `%APPDATA%\qkz_eq\presets.json` on Windows.

Double-click a gain slider to reset it to 0 dB.

## How it works

The chip exposes a vendor HID interface (interface 3) that accepts 11-byte register read/write reports. The EQ, gain and identity strings are 32-bit registers. See [docs/PROTOCOL.md](docs/PROTOCOL.md) for the wire format and [docs/REGISTERS.md](docs/REGISTERS.md) for the known register map.

```
src/
├── main.rs      UI (egui)
├── device.rs    HID transport and register encoding
└── presets.rs   Built-in and user presets
tools/           Python scripts used to probe and dump the device
docs/            Protocol and register documentation
```

## Roadmap

- [ ] Measure output gain register scale
- [ ] Support more KT Micro VID/PIDs
- [ ] Analog Global Gain (register `0x3B`)

## Acknowledgements

The save command, frequency scaling and filter types were worked out by the [Contour](https://github.com/Chronosauros/contour) project, an open-source Android app for USB DAC EQs.

## Contributing

Register dumps from other devices are the most useful contribution. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
