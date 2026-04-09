# gpu-overlay

Transparent, always-on-top GPU metrics overlay for macOS and Linux.

Displays real-time GPU temperature, utilization, VRAM, and clock speeds in a compact floating panel.

## Supported GPUs

| Vendor | Platform | Data Source |
|---|---|---|
| NVIDIA | macOS + Linux | nvidia-smi |
| AMD | Linux | sysfs/hwmon |
| Intel (Arc/integrated) | Linux | sysfs/i915 |
| Apple Silicon (M-series) | macOS | IORegistry |

Board vendor (MSI, ASUS, EVGA, etc.) is detected from PCI subsystem IDs.

## Build

```sh
cargo build --release
```

## Run

```sh
cargo run --release
```

No configuration needed. The overlay appears in the top-right corner of the primary monitor.

## Features

- Transparent, click-through background (mouse events pass through)
- Visible on all workspaces and in fullscreen apps
- Auto-detects all available GPUs
- Multi-GPU support
- Board vendor detection (MSI, ASUS, EVGA, ZOTAC, etc.)
- 1-second refresh rate
- Zero config

## License

[MIT](LICENSE)
