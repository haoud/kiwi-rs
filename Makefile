all: build

# Build the userspace and kernel
build: build-kernel

# Build the kernel
build-kernel:
	cd kernel && cargo build --release --target riscv64gc-unknown-none-elf

# Run the kernel
run: build
	cd kernel && cargo run --release --target riscv64gc-unknown-none-elf

# Clean the intermediate build files
clean:
	cd kernel && cargo clean

# Print help message
help:
	@echo "                                                                    "
	@echo "                       Welcome to Kiwi !                            "
	@echo "                                                                    "
	@echo "This command is work-in-progress but in the future, you will be able"
	@echo "to get some help and information about kiwi and how to build and run"
	@echo "the project                                                         "
	@echo "                                                                    "
