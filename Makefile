DOCKER_NAME ?= rcore-tutorial-v3
.PHONY: docker build_docker
	
docker:
	docker run --rm -it -v ${PWD}:/mnt -w /mnt ${DOCKER_NAME} bash

build_docker: 
	docker build -t ${DOCKER_NAME} .

fmt:
	cd easy-fs; cargo fmt; cd ../easy-fs-fuse cargo fmt; cd ../kernel ; cargo fmt; cd ../user; cargo fmt; cd ..

all:
	cd ./os && make build
	cp os/target/riscv64gc-unknown-none-elf/release/os.bin kernel-qemu
	cp bootloader/rustsbi-qemu.bin sbi-qemu

test: all
	@cp test/sdcard.img .

	@qemu-system-riscv64 -machine virt \
        -m 128M -nographic -smp 2 \
        -kernel kernel-qemu \
        -bios sbi-qemu \
        -drive file=sdcard.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
        -device virtio-net-device,netdev=net \
        -netdev user,id=net | tee output.log

# the test scripts produce 'SyntaxWarning: invalid escape sequence'
	@python3 -W ignore test/check_result/test_runner.py output.log > results.json
	@python3 test/visualize_result.py results.json
	@rm -f sdcard.img
	@rm -f *-qemu
	@rm -f *.json

