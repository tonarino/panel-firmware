# This finds the first USB-to-serial converter connected to the machine.
serial-port := $(shell serial-monitor -f --index 0)

flash:
	(cargo build --release && cargo objcopy --release -- -O binary stm32-test.bin && stm32flash -b 230400 -w stm32-test.bin -v $(serial-port))

monitor:
	serial-monitor -b 115200 -p $(serial-port)
