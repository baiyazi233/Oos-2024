DOCKER_NAME ?= rcore-tutorial-v3
.PHONY: docker build_docker
	
docker:
	docker run --rm -it -v ${PWD}:/mnt -w /mnt ${DOCKER_NAME} bash

build_docker: 
	docker build -t ${DOCKER_NAME} .

fmt:
	cd easy-fs; cargo fmt; cd ../easy-fs-fuse cargo fmt; cd ../kernel ; cargo fmt; cd ../user; cargo fmt; cd ..

all:
	@rm -rf os/.cargo
	@cp -r os/cargo-submit os/.cargo
	@rm -rf user/.cargo
	@cp -r user/cargo-submit user/.cargo
	@cd ./os && make build
	@cp os/target/riscv64gc-unknown-none-elf/release/os.bin kernel-qemu
	@cp bootloader/rustsbi-qemu.bin sbi-qemu

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

fuck: all
	@qemu-system-riscv64 -machine virt -kernel kernel-qemu -m 128M -nographic -smp 2 -bios sbi-qemu -drive file=sdcard-riscv.img,if=none,format=raw,id=x0  -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 -device virtio-net-device,netdev=net -netdev user,id=net
	@rm -f sbi-qemu
	@rm -f kernel-qemu


