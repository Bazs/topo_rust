# base stage containing all build dependencies and the source code.
FROM ubuntu:22.10 AS dependencies

WORKDIR /usr/local/build

# Install cc, pkg-config, and dependencies for linking against PROJ.
RUN apt-get update && apt-get install -y build-essential pkg-config \
  libproj-dev libgdal-dev curl libclang-dev
# Install Rust.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh && \
  chmod +x rustup.sh && \
  ./rustup.sh -y && \
  rm ./rustup.sh 

# Add source code.
FROM dependencies as base
ADD . .

# Run unit tests in a new build stage.
FROM base as tester
RUN ~/.cargo/bin/cargo test --release

# builder stage which builds the executable in release mode.
FROM tester as builder

# Build executable.
RUN ~/.cargo/bin/cargo build --release && \
  mv target/release/topo_rust ./

# Release build stage whith dependencies and the executable.
FROM dependencies

WORKDIR /usr/local/app

COPY --from=builder /usr/local/build/topo_rust .