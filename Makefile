IMAGE_NAME=balazsopra/topo-rust
IMAGE_TAG=$(shell git rev-parse --short HEAD)-$(shell date +'%Y-%m-%d')

.PHONY: docker-build
docker-build:
	cargo build --release
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .

.PHONY: docker-build-push
docker-build-push: docker-build
	docker push $(IMAGE_NAME):$(IMAGE_TAG)