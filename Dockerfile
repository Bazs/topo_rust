# base stage containing all build dependencies and the source code.
FROM osgeo/gdal:ubuntu-small-3.5.0 AS base

WORKDIR /usr/local/build

# Install Rust.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh && \
  chmod +x rustup.sh && \
  ./rustup.sh -y && \
  rm ./rustup.sh 
# Install cc, pkg-config, and dependencies for linking against PROJ.
RUN apt-get update && apt-get install -y build-essential pkg-config \
  libssl-dev libtiff-dev libsqlite3-dev libcurl4-openssl-dev libclang-dev
# Create a symlink to the PROJ library file installed in osgeo/gdal for pkg-config to pick up.
# The crate "proj-sys" uses pkg-config to detect if proj is installed, it won't find it otherwise.
# Use dpkg-architecture -s to set the DEB_TARGET_MULTIARCH variable, which identifies the platform we're in (osgeo/gdal is multiplatform).
RUN eval $(dpkg-architecture -s) && \
  ln -s /lib/$DEB_TARGET_MULTIARCH/libproj.so.15 /usr/local/lib/libproj.so

ADD . .

FROM base as tester
# Run unit tests
RUN ~/.cargo/bin/cargo test --release

# builder stage which builds the executable in release mode.
FROM tester as builder

# Build executable.
RUN ~/.cargo/bin/cargo build --release && \
  mv target/release/topo_rust ./

# release stage which contains the executable only.
FROM osgeo/gdal:ubuntu-small-3.5.0

WORKDIR /usr/local/app

COPY --from=builder /usr/local/build/topo_rust .