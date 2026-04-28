.PHONY: build-docker release-linux release-local release-all clean

IMAGE := ccli-builder

build-docker:
	docker build --target builder -t $(IMAGE) .

release-linux: build-docker
	mkdir -p dist
	docker build --target output -o dist .

release-local:
	mkdir -p dist
	cargo build --release
	cp target/release/ccli dist/ccli-$(shell uname -s | tr A-Z a-z)-$(shell uname -m | sed 's/x86_64/amd64/;s/arm64/arm64/;s/aarch64/arm64/')

release-all: release-linux release-local

clean:
	rm -rf dist
