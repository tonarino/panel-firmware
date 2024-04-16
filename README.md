# panel-firmware

Microcontroller firmware for controlling the lights and volume dial for [tonari](https://tonari.no/).

More info outlined here:

[https://blog.tonari.no/rust-simple-hardware-project](https://blog.tonari.no/rust-simple-hardware-project)

## Dependencies

* [cargo, rustc](https://rustup.rs) (strict dependency on 1.60.0. see https://github.com/tonarino/panel-firmware/issues/28)
* `dfu-util` (`brew install dfu-util`, `apt install dfu-util`, etc.)
* (Optional, for UART flashing) `stm32flash` (`brew install stm32flash`, `apt install stm32flash`, etc.)
* (Optional, for UART flashing) `serial-monitor` (`cargo install serial-monitor`)


## Target STM32 Models

The current firmware uses this model:
```
STM32F411RE
```

This firmware can also be debugged on a USB-C "black pill" board, linked here:
[Board Info](https://stm32-base.org/boards/STM32F411CEU6-WeAct-Black-Pill-V2.0)

The firmware also used to run on the cheaper STM32F103-based boards. Look in the commit history for working with that. It might be beneficial in the future to support both simultaneously and enable one or the other via feature flags.

## Steps

```
rustup target add thumbv7em-none-eabihf
```

## Workflow

You can use the Makefile for easy development.

```bash
# Build and flash the firmware
make flash

# Monitor the serial output
make monitor
```

## Board Connection

### USB DFU
Simply connect the host computer to the STM32 dev board via a USB cable.

### Serial Flashing
Using a CP2102 (3.3v logic) or another USB-Serial converter, connect its `TX` to pin `A10` and its `RX` to pin `A9`.
Also connect 3.3v from the CP2102 to the 3.3v pin on the STM32, and do the same for ground.
If you try to power the STM32 from its USB port without this power connection, it won't work.

## Convert to BIN File

`cargo build` will create an ARM ELF file, but we need it in a binary `.bin` format.

### Install the Tools

```
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

### Create the BIN File

```
cargo objcopy --release -- -O binary panel-brain-firmware.bin
```

## Flash the BIN File

On the "black pill" board, hold down the `BOOT0` button, press and release `NRST` (reset button), then let get of `BOOT0` to get into flashing mode.

Find your USB-UART converter path via a tool like `serial-monitor` or however you prefer. On MacOS it turned up as `/dev/cu.SLAB_USBtoUART` but results will vary.


### USB DFU Flashing

```
dfu-util -D panel-brain-firmware.bin -d "0483:df11" -a 0 -s 0x08000000
```

### Serial UART Flashing
```
stm32flash -b 230400 -w panel-brain-firmware.bin -v /dev/cu.SLAB_USBtoUART
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

## Debugging with an STM32F103-based Board

* Download the BlackMagic `v*.tar.gz` tarball release on [github](https://github.com/blacksphere/blackmagic/releases/)
* Find `blackmagic_dfu_swlink.bin` in the tarball.
* Using a USB-serial device, put an STM32F103 dev board (typically a blue-pill or black-pill) in bootloader mode (see the steps above).
* Run `stm32flash -R -b 230400 -w blackmagic_dfu-swlink.bin -v <PATH_TO_YOUR_SERIAL_DEVICE_HERE>`
* Get `blackmagic-native.bin` from the release tarball as well.
* Run `dfu-util -d 1d50:6018,:6017 -s 0x08002000:leave -D blackmagic-native.bin`

### Debugging
* Attach the SWD wires from the blackmagic debug probe you just created to the target device. This is typically 4 wires: `3v3`, `GND`, `SWDIO`, and `SWCLK`.
* Run `arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/panel-firmware` (you will need a GNU ARM toolchain for this)
* Inside gdb, run `target extended-remote /dev/cu.usbmodem95C55F961` (or wherever your blackmagic probe shows up as a device)
* In gdb: `monitor swdp_scan`
* In gdb: `attach 1`
