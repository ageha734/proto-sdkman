# proto-sdkman

A [proto](https://moonrepo.dev/proto) WASM plugin for managing [SDKMAN](https://sdkman.io/) installations.

SDKMAN is a tool for managing parallel versions of multiple Software Development Kits on Unix-like systems (Java, Kotlin, Groovy, Scala, etc.).

## Installation

Add to your `.prototools`:

```toml
[plugins]
sdkman = "github://ageha734/proto-sdkman"
```

Or add directly from a release:

```toml
[plugins]
sdkman = "https://github.com/ageha734/proto-sdkman/releases/latest/download/proto_sdkman.wasm"
```

## Usage

```bash
# Install SDKMAN
proto install sdkman

# Use the sdk command via proto shim
proto run sdkman -- list java
proto run sdkman -- install java 21.0.2-tem
```

## Platform Support

| Platform      | Supported |
|---------------|-----------|
| Linux (x86_64, aarch64) | Yes |
| macOS (x86_64, aarch64) | Yes |
| Windows       | No (use WSL) |

## How It Works

This plugin uses proto's `native_install` mechanism to:

1. Download the SDKMAN install script from `https://get.sdkman.io`
2. Execute it via `bash`, setting `SDKMAN_DIR` to the proto-managed install directory
3. Configure shell profile to export `SDKMAN_DIR`

The `sdk` command is a shell function, so the plugin configures it to be invoked via `bash -c "source sdkman-init.sh && sdk ..."`.

## Development

### Prerequisites

- Rust toolchain with `wasm32-wasip1` target
- proto installed locally

### Build

```bash
cargo build --target wasm32-wasip1 --release
```

### Test

```bash
cargo test
```

### Local Testing

```bash
cargo build --target wasm32-wasip1 --release
proto plugin add sdkman source:./target/wasm32-wasip1/release/proto_sdkman.wasm
proto install sdkman latest
```

## License

MIT
