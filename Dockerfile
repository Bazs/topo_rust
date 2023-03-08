FROM osgeo/gdal:ubuntu-small-3.5.0 AS builder

WORKDIR /usr/local/build

# Install Rust, cc, and OpenSSL
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh && \
  chmod +x rustup.sh && \
  ./rustup.sh -y && \
  rm ./rustup.sh && \
  apt-get update && apt-get install -y build-essential pkg-config libssl-dev

# Add code and build
ADD . .
RUN ~/.cargo/bin/cargo build --release && \
  mv target/release/topo_rust ./

FROM osgeo/gdal:ubuntu-small-3.5.0

WORKDIR /usr/local/app

COPY --from=builder /usr/local/build/topo_rust .