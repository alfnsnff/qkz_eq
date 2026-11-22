# Contributing

Thanks for helping out. The project is small, so the process is light.

## Device reports

The most valuable contribution is data from dongles that aren't supported yet.

1. Set up the Python tools (see [tools/README.md](tools/README.md)).
2. Run `qkz_probe.py` to list the HID interfaces. Change the PID at the top of the scripts to your device's.
3. Run `qkz_read.py` for a **read-only** register dump.
4. Open a [New device report](../../issues/new?template=device_report.yml) issue with both outputs.

Please don't send write commands to a device you haven't dumped first.

## Code

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo run --release
```

- Keep the HID protocol in `src/device.rs` and the UI in `src/main.rs`.
- Document any newly understood register in `docs/REGISTERS.md`, and mark whether it is *observed* or *confirmed*.
- Add a line under `[Unreleased]` in `CHANGELOG.md`.

## Commit messages

Short imperative subject (`Add shelf filter types`), with a body explaining why when it isn't obvious.
