# Build applications
build:
	cd init && cargo build --release --target=riscv64gc-unknown-none-elf
	cd echo && cargo build --release --target=riscv64gc-unknown-none-elf
	cd template && cargo build --release --target=riscv64gc-unknown-none-elf

# Clean the intermediate build files
clean:
	cd init && cargo clean
	cd echo && cargo clean
	cd template && cargo clean
