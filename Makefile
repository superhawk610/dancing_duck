.PHONY: compile upload monitor deploy imagetoh

board_fqbn = arduino:avr:uno
serial_port = COM4

imagetoh:
	cargo run --manifest-path imagetoh/Cargo.toml -- image_bytes.h

compile:
	arduino-cli compile -b $(board_fqbn) .

upload: compile
	arduino-cli upload -p $(serial_port) -b $(board_fqbn) .

monitor:
	arduino-cli monitor -p $(serial_port) -b $(board_fqbn)

deploy: upload monitor
