# Probe tools

Python scripts used to reverse-engineer the device. Both are **read-only**.

| Script | What it does |
|---|---|
| `qkz_probe.py` | Lists the device's HID interfaces and prints their report descriptors |
| `qkz_read.py` | Dumps registers `0x00–0xFF` over interface 3 to `qkz_dump.txt` |

## Setup

```bash
python -m venv .venv
.venv/Scripts/activate   # Windows; use .venv/bin/activate elsewhere
pip install -r tools/requirements.txt
python tools/qkz_probe.py
python tools/qkz_read.py
```

For a different dongle, edit `VID, PID` at the top of each script.

Dumps are git-ignored because registers `0x50–0x56` may hold a device serial.
