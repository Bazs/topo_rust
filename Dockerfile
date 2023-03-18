# tester stage containing all build dependencies and the source code.
FROM osgeo/gdal:ubuntu-small-3.5.0 AS tester

WORKDIR /usr/local/build

# Install Rust, cc, and dependencies for PROJ. The crate "proj-sys" builds PROJ from source,
# and this needs some additional dependencies.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh && \
  chmod +x rustup.sh && \
  ./rustup.sh -y && \
  rm ./rustup.sh && \
  apt-get update && apt-get install -y build-essential pkg-config \
  libssl-dev libtiff-dev libsqlite3-dev libcurl4-openssl-dev libclang-dev libproj-dev

# Add code
ADD . .

# builder stage which builds the executable in release mode.
FROM tester as builder

# Build executable.
RUN ~/.cargo/bin/cargo build --release && \
  mv target/release/topo_rust ./

# release stage which contains the executable only.
FROM osgeo/gdal:ubuntu-small-3.5.0

WORKDIR /usr/local/app

COPY --from=builder /usr/local/build/topo_rust .