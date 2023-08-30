
## Andoid bindings


### Prepare

Rust

install `rustup`

    rustup target add aarch64-linux-android
    rustup target add armv7-linux-androideabi
    rustup target add i686-linux-android
    rustup target add x86_64-linux-android

Install Android NDK


Add /Users/dubadub/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/ to PATH

### Build

Build library:

    cargo build --features=bindings --lib --target=x86_64-linux-android --release

Biuld bindings

    cargo run --features="bindings uniffi/cli"  \
      --bin uniffi-bindgen generate \
      --library target/x86_64-linux-android/release/libcooklang.so \
      --language kotlin \
      --out-dir out