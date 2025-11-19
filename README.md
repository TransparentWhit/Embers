# Embers
> Stop the hours counting down

## Development

### Prerequisites

The `lld` linker is required to compile Embers.

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
