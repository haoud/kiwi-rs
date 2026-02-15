all: build

# Build the userspace and kernel
build: build-user build-kernel

# Build the kernel
build-kernel:
	cd kernel && cargo build --release

# Build the userspace
build-user:
	cd user && make build

# Run the kernel
run: build-user
	cd kernel && cargo run --release

# Clean the intermediate build files
clean:
	cd kernel && cargo clean
	cd user && make clean

# Print help message
help:
	@echo "                                                                    "
	@echo "                       Welcome to Kiwi !                            "
	@echo "                                                                    "
	@echo "This command is work-in-progress but in the future, you will be able"
	@echo "to get some help and information about kiwi and how to build and run"
	@echo "the project                                                         "
	@echo "                                                                    "
