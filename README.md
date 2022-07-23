# ZinoEngine (Rust version)

An attempt to make a 3D game engine in Rust.

| Platform | CI Status                                                                                |
|----------|------------------------------------------------------------------------------------------|
| Windows  | ![WinCI](https://github.com/Zino2201/zinoengine-rs/actions/workflows/rust.yml/badge.svg) |

## How to compile (Windows)
- Clone the project and its submodules
- Install clang and set your `LIBCLANG_PATH` env var to the directory containing `clang.exe`
- `cargo build`
- Run! Note: You may need to download/copy some DLLs:
  - `dxcompiler.dll`, `dxil.dll` DirectXShaderCompiler
  - `WinPixEventRuntime.dll`, `WinPixEventRuntime_UAP.dll` PIX debugging
  - `D3D12/D3D12Core.dll`, `D3D12/d3d12SDKLayers.dll` D3D12 Agility SDK (must be placed at the subdirectory D3D12, see the macro `ze_d3d12_agility_sdk_statics!`)