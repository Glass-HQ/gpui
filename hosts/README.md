# Host Shells

GPUI keeps platform runtime behavior in Rust crates and treats native app hosts
as thin packaging shells.

## Layers

1. `crates/*`
   Rust runtime, rendering, platform adapters, and shared example code.
2. `hosts/*`
   Native or platform-specific boot wrappers required for packaging, signing,
   HTML bootstrapping, or launch metadata.
3. `cargo gpui`
   The canonical orchestration entrypoint for syncing hosts, building, running,
   and future platform workflows.

## iOS

The iOS host lives in `hosts/ios/`.

Checked-in source:

- `project.yml`
- `Entitlements.plist`
- `runner/main.m`

Generated or disposable artifacts:

- `GPUIiOS.xcodeproj/`
- `build/`

`xcodegen` materializes the Xcode project from `project.yml`. The Xcode pre-build
phase invokes `cargo gpui host build-rust ios` directly, so the Rust static
library is produced through the same tooling path used by local development.
`cargo gpui host sync ios` replaces generated Xcode output instead of treating it
as durable source.

## Web

The web host lives in `hosts/web/hello_web/`.

Checked-in source:

- `Cargo.toml`
- `main.rs`
- `index.html`
- `trunk.toml`
- `.cargo/config.toml`
- `rust-toolchain.toml`

Generated or disposable artifacts:

- `dist/`
- `target/`
- `Cargo.lock`

The web host is a thin wasm/bootstrap wrapper around shared example code in
`crates/gpui_examples`.

## Command Surface

Use `cargo gpui` instead of platform-local scripts:

```sh
cargo gpui host sync ios
cargo gpui devices ios
cargo gpui run ios
cargo gpui run ios hello_world --sim
cargo gpui build ios --release
```

## Design Rule

Platform crates are durable product code. Host shells are replaceable adapters.
