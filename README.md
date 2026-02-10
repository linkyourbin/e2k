# e2k

**Fast and reliable EasyEDA/LCSC to KiCad converter written in Rust**

Convert EasyEDA and LCSC components to KiCad library formats with a single command.

## Features

- ✅ Convert symbols to KiCad format (.kicad_sym or .lib)
- ✅ Convert footprints to KiCad format (.kicad_mod)
- ✅ Convert 3D models (OBJ to VRML, STEP passthrough)
- ✅ Support for KiCad v5.x (legacy) and v6.x/v7.x formats
- ✅ Fast Rust implementation with low memory usage
- ✅ Type-safe coordinate conversions
- ✅ Standalone binary - no dependencies required

## Installation

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/e2k.exe` (Windows) or `target/release/e2k` (Linux/macOS).

## Quick Start

```bash
# Convert everything (symbol + footprint + 3D model)
e2k --full --lcsc-id C529356

# Convert only symbol
e2k --symbol --lcsc-id C529356

# Convert only footprint
e2k --footprint --lcsc-id C529356

# Convert only 3D model
e2k --3d --lcsc-id C529356
```

## Usage

```
e2k [OPTIONS] --lcsc-id <ID>

Options:
  --lcsc-id <ID>          LCSC component ID (e.g., C2040)
  --symbol                Convert symbol only
  --footprint             Convert footprint only
  --3d                    Convert 3D model only
  --full                  Convert all (symbol + footprint + 3D model)
  -o, --output <PATH>     Output directory [default: .]
  --overwrite             Overwrite existing components
  --v5                    Use KiCad v5 legacy format
  --project-relative      Use project-relative paths for 3D models
  --debug                 Enable debug logging
  -h, --help              Print help
  -V, --version           Print version
```

## Examples

```bash
# Convert with custom output directory
e2k --full --lcsc-id C529356 -o ./my_library

# Use KiCad v5 legacy format
e2k --full --lcsc-id C529356 --v5

# Overwrite existing component
e2k --symbol --lcsc-id C529356 --overwrite

# Enable debug logging
e2k --full --lcsc-id C529356 --debug
```

## Output Structure

```
output_directory/
├── e2k.kicad_sym              # Symbol library (v6 format)
├── e2k.lib                    # Symbol library (v5 format, if --v5)
├── e2k.pretty/                # Footprint library
│   └── Component_Name.kicad_mod
└── e2k.3dshapes/              # 3D model library
    ├── Component_Name.wrl     # VRML format
    └── Component_Name.step    # STEP format
```

## Performance

- **Fast**: Native compiled code with minimal overhead
- **Efficient**: Low memory usage (~20MB)
- **Reliable**: Type-safe conversions prevent runtime errors

## Comparison with Python Version

| Feature | Python | e2k (Rust) |
|---------|--------|------------|
| Symbol conversion | ✅ | ✅ |
| Footprint conversion | ✅ | ✅ |
| 3D model conversion | ✅ | ✅ |
| KiCad v5 support | ✅ | ✅ |
| KiCad v6/v7 support | ✅ | ✅ |
| Performance | Good | Excellent |
| Memory usage | ~50MB | ~20MB |
| Dependencies | Python + packages | None (standalone) |
| Type safety | Runtime | Compile-time |
| Binary size | N/A | 7.1MB |

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run with Logging

```bash
RUST_LOG=debug cargo run -- --full --lcsc-id C529356
```

## Architecture

```
e2k/
├── src/
│   ├── main.rs                      # CLI entry point
│   ├── lib.rs                       # Library root
│   ├── cli.rs                       # Command-line parsing
│   ├── error.rs                     # Error types
│   ├── converter.rs                 # Coordinate/unit conversions
│   ├── library.rs                   # Library file management
│   ├── easyeda/
│   │   ├── api.rs                   # EasyEDA API client
│   │   ├── importer.rs              # Data importers
│   │   ├── models.rs                # Data structures
│   │   └── svg_parser.rs            # SVG path parsing
│   └── kicad/
│       ├── symbol.rs                # Symbol structures
│       ├── footprint.rs             # Footprint structures
│       ├── symbol_exporter.rs       # Symbol export
│       ├── footprint_exporter.rs    # Footprint export
│       └── model_exporter.rs        # 3D model export
└── tests/
    └── integration_tests.rs
```

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Credits

Based on the original [easyeda2kicad](https://github.com/uPesy/easyeda2kicad.py) Python implementation.
