# This finds the first USB-to-serial converter connected to the machine.
serial-port := $(shell serial-monitor -f --index 0)

flash:
	(cargo build --release && cargo objcopy --release --bin panel-firmware -- -O binary panel-brain-firmware.bin && dfu-util -D panel-brain-firmware.bin -d "0483:df11" -a 0 -s 0x08000000:leave)

flash-serial:
	(cargo build --release && cargo objcopy --release --bin panel-firmware -- -O binary panel-brain-firmware.bin && stm32flash -R -b 230400 -w panel-brain-firmware.bin -v $(serial-port))

monitor:
	serial-monitor -b 115200 -p $(serial-port)
