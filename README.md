## Dependencies

* [cargo, rustc](https://rustup.rs)
* `stm32flash` (`brew install stm32flash`, `apt install stm32flash`, etc.)
* `serial-monitor` (`cargo install serial-monitor`)


## Target STM32 Models

(the following README steps target this chip)
```
STM32F
411CEU6
```
[Board Info](https://stm32-base.org/boards/STM32F411CEU6-WeAct-Black-Pill-V2.0)

(I have some of these laying around, not tested yet)
```
STM32F
103C8T6
```

[Board Info](https://stm32-base.org/boards/STM32F103C8T6-Black-Pill)

## Steps

```
rustup target add thumbv7em-none-eabi
```

## Workflow

You can use the makefile for easy development.

```bash
# Build and flash the firmware
make flash

# Monitor the serial output
make monitor
```

## Board Connection

Using a CP2102 (3.3v logic) or another USB-Serial converter, connect its `TX` to pin `A10` and its `RX` to pin `A9`.
Also connect 3.3v from the CP2102 to the 3.3v pin on the STM32, and do the same for ground.
If you try to power the STM32 from its USB C port without this power connection, it won't work.

## Convert to BIN File

`cargo build` will create an ARM ELF file, but we need it in a binary `.bin` format.

### Install the Tools

```
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

### Create the BIN File

```
cargo objcopy --release -- -O binary stm32-test.bin
```

## Flash the BIN File

On the "black pill" board, hold down the `BOOT0` button, press and release `NRST` (reset button), then let get of `BOOT0` to get into flashing mode.

Find your USB-UART converter path via a tool like `serial-monitor` or however you prefer. On MacOS it turned up as `/dev/cu.SLAB_USBtoUART` but results will vary.

```
stm32flash -b 230400 -w stm32-test.bin -v /dev/cu.SLAB_USBtoUART
```

## Monitor Serial Output

In the spirit of doing everything in Rust, you can install a straightforward serial monitor via Cargo:

```
cargo install serial-monitor
```

Simply invoke it with `serial-monitor` and it will begin monitoring the first serial port it finds.

You can also pass it a specific device with

```
serial-monitor -p /dev/cu.SLAB_USBtoUART
```

Set the baud rate with the `-b` flag:

```
serial-monitor -b 9600 -p /dev/cu.SLAB_USBtoUART
```
