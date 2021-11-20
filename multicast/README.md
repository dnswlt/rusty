# Cross-compile for Raspberry 2

```shell
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf
cargo build --release --target=armv7-unknown-linux-gnueabihf
```

Update 2021-11-20:
I received an error about GLIBC version when running the binary on the Pi.

https://github.com/japaric/rust-cross/issues/42 has the solution:

```
mkdir -p $HOME/git/github.com/raspberrypi
cd $HOME/git/github.com/raspberrypi
git clone https://github.com/raspberrypi/tools

RUSTFLAGS="-C linker=$HOME/git/github.com/raspberrypi/tools/arm-bcm2708/arm-rpi-4.9.3-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc" cargo build --target armv7-unknown-linux-gnueabihf --release
```
