# Common LUT Format (CLF) parser

[![Actions Status](https://github.com/Traverse-Research/clf-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/Traverse-Research/clf-parser/actions)
[![Latest version](https://img.shields.io/crates/v/clf-parser.svg?logo=rust)](https://crates.io/crates/clf-parser)
[![Documentation](https://docs.rs/clf-parser/badge.svg)](https://docs.rs/clf-parser)
[![Lines of code](https://tokei.rs/b1/github/Traverse-Research/clf-parser)](https://github.com/Traverse-Research/clf-parser)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4%20adopted-ff69b4.svg)](./CODE_OF_CONDUCT.md)
[![MSRV](https://img.shields.io/badge/rustc-1.74.0+-ab6000.svg)](https://blog.rust-lang.org/2023/11/16/Rust-1.74.0.html)

[![Banner](banner.png)](https://traverseresearch.nl)

https://docs.acescentral.com/specifications/clf

## Supported features

### Operators

- LUT1D
- LUT3D
- Range

### Bit depths

- `32F`

## Usage

Add this to your Cargo.toml:

```toml
[dependencies]
clf-parser = "0.0.0"
```

```rust
fn main() -> anyhow::Result<()> {
    let reader = std::fs::OpenOptions::new().read(true).open("my_file.clf")?;
    let clf = clf_parser::load_clf(reader)?;
    Ok(())
}
```
