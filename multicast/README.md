# Cross-compile for Raspberry 2

```shell
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf
cargo build --release --target=armv7-unknown-linux-gnueabihf
```
