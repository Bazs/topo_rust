.PHONY: docker-build-dev-image
docker-build-dev-image:
	docker build --target dependencies -t topo-rust-dev:latest .
