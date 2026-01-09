# Embers
> Stop the hours counting down

[![Issues](https://img.shields.io/github/issues/TransparentWhit/Embers)](https://github.com/TransparentWhit/Embers/issues)
[![Pull Requests](https://img.shields.io/github/issues-pr/TransparentWhit/Embers)](https://github.com/TransparentWhit/Embers/pulls)
[![Discussions](https://img.shields.io/github/discussions/TransparentWhit/Embers)](https://github.com/TransparentWhit/Embers/discussions)
[![CI](https://github.com/TransparentWhit/Embers/actions/workflows/ci.yml/badge.svg)](https://github.com/TransparentWhit/Embers/actions/workflows/ci.yml)

## Development

[![Developer QQ](https://img.shields.io/badge/Developer_QQ-459451456-1EBAFC?logo=qq)](https://qm.qq.com/q/mOIwdrkad2)

### Prerequisites

To compile Embers, you need the latest stable Rust toolchain.

On `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`, you also need the `lld` linker.

<details>
<summary>LLD Installation</summary>

- Ubuntu: `sudo apt-get install lld clang`
- Fedora: `sudo dnf install lld clang`
- Arch: `sudo pacman -S lld clang`
- Windows:
  ```
  cargo install -f cargo-binutils
  rustup component add llvm-tools-preview
  ```
</details>

> [!TIP]
> Use `--features dev` to reduce build times and enable more development features
