.PHONY: test

test:
	cargo test

.PHONY: benchmark

benchmark:
	cargo build --release --features runtime-benchmarks

	./.maintain/benchmark.sh magic
	./.maintain/benchmark.sh did 2 50

	./.maintain/benchmark.sh advertiser
	./.maintain/benchmark.sh tag 2 50

	./.maintain/benchmark.sh swap
	./.maintain/benchmark.sh nft 2 50

	./.maintain/benchmark.sh ad 2 1000

	./.maintain/benchmark.sh linker 2 50

.PHONY: clean

clean:
	cargo clean -p parami
	cargo clean -p parami-runtime
