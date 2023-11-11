all: bin
bin:
	cargo build --release

clean:
	rm -rf ./target
