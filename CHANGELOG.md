# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-11-22

### Added

- Save EQ to the dongle's flash (command `0x53`), confirmed by a read after it reconnects
- Low and high shelf filter types
- Custom title bar, edge resize and rounded corners on Windows 11
- App logo and icon
- Disconnected screen with automatic reconnect

### Changed

- Compact layout; presets moved into the toolbar
- Preamp replaced by an output gain trim under *Lanjutan*

### Fixed

- Band frequencies were written at their audible value but the chip plays them at twice the register value; they are now halved on the wire
- EQ off now writes `2` (the chip's off value) instead of `0`
- Gain slider centered in its card

## [0.1.0] - 2026-11-22

### Added

- 5-band DAC EQ editor with live response curve
- Read / Apply with change tracking and optional auto-apply
- Built-in and user presets (JSON on the PC)
- Experimental preamp control (register `0x66`)
- Protocol and register documentation, Python probe tools
