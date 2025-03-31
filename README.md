[![Workflow Status](https://github.com/enarx/rcrt1/workflows/test/badge.svg)](https://github.com/enarx/rcrt1/actions?query=workflow%3A%22test%22)
[![Average time to resolve an issue](https://isitmaintained.com/badge/resolution/enarx/rcrt1.svg)](https://isitmaintained.com/project/enarx/rcrt1 "Average time to resolve an issue")
[![Percentage of issues still open](https://isitmaintained.com/badge/open/enarx/rcrt1.svg)](https://isitmaintained.com/project/enarx/rcrt1 "Percentage of issues still open")
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# rcrt1

## Overview
rcrt1 is a Rust library developed by the Enarx project for relocating ELF dynamic symbols. It serves as a replacement for the traditional C runtime (crt1) and handles the crucial task of properly loading and relocating ELF binaries at runtime, specifically focusing on relocating dynamic symbols.

## Core Functionality
- **Dynamic Relocation**: Provides functionality to relocate ELF dynamic symbols in a static Position Independent Executable (PIE)
- **No Standard Library**: Operates as a `no_std` crate, making it suitable for system-level programming without relying on the Rust standard library
- **ELF Support**: Specifically handles ELF64 binaries using the Goblin library for parsing ELF structures
- **x86_64 Focus**: Primarily targets x86_64 architecture, with specific handling for x86_64 relative relocations

## Key Components
1. **Dynamic Symbol Relocation**: The `dyn_reloc` function handles the relocation of dynamic symbols by applying base address offsets
2. **Runtime Initialization**: The `rcrt` function serves as a replacement for the traditional `crt1.o`, handling initial program setup from the stack pointer
3. **Startup Macro**: Provides a convenient `x86_64_linux_startup!` macro for defining the program entry point

## Technical Details
- Handles both REL and RELA relocation types for ELF binaries
- Works with Position Independent Executables (PIEs)
- Requires specific compiler optimizations in the binary's Cargo.toml for correct operation
- Manages program headers and the `_DYNAMIC` section to calculate load offsets
- Uses naked functions and assembly for low-level operations

## Usage Requirements
When using this crate, specific compiler optimizations must be set in the binary's Cargo.toml:
```toml
[profile.dev.package.rcrt1]
opt-level = 3
debug-assertions = false
overflow-checks = false
```
