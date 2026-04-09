# gpu-overlay

Transparent, always-on-top GPU metrics overlay for macOS and Linux.

Displays real-time GPU temperature, utilization, VRAM, and clock speeds in a compact floating panel — similar to MSI Afterburner's on-screen display.

## Supported GPUs

| Vendor | Platform | Data Source |
|---|---|---|
| Apple Silicon (M-series) | macOS | IORegistry |
| NVIDIA | macOS + Linux | nvidia-smi |
| AMD | Linux | sysfs/hwmon |
| Intel (Arc/integrated) | Linux | sysfs/i915 |

## Build

```sh
cargo build --release
```

## Run

```sh
cargo run --release
```

No configuration needed. The overlay appears in the top-right corner.

## Features

- Transparent background, click-through (mouse events pass through)
- Visible on all Spaces and in fullscreen apps
- Auto-detects all available GPUs
- Multi-GPU support (displays all detected GPUs)
- 1-second refresh rate
- Zero config

## License

[MIT](LICENSE)
