
[![CI - CD](https://github.com/Bazs/topo_rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Bazs/topo_rust/actions/workflows/rust.yml)

---

# Rust implementation of the TOPO metric

There are several ways to use and develop the tool.

## Build locally

Besides the rust native dependencies expressed in the Cargo.lock file, the project depends on GDAL version 2.4-3.6 and
PROJ version 9.0.0 being installed locally. See 
* https://gdal.org/download.html, and
* https://proj.org/install.html

for download details for your platform.

Please make sure to build the binary using `cargo build --release`, otherwise the runtime performance will be slow.

## Use the release Docker image

Check the https://hub.docker.com/repository/docker/balazsopra/topo-rust/general repository for the Docker latest image.
The executable is in the default work dir inside the image. Example execution:

```
docker run --rm -ti -v <local dir with input files and config file>:/usr/local/app balazsopra/topo-rust:<check the latest tag>
$ ./topo_rust --config-filepath <your config file>
```

## Develop inside Docker

The Dockerfile has a build stage which has all dependencies including the Rust compiler installed. It can be built with 
`make` using the recipe in the [Makefile](./Makefile). Example:

```
make docker-build-dev-image

# Mount the code into the image.
docker run --rm -ti -v <path to topo-rust repository>:/usr/local/code \
  -w /usr/local/code topo-rust-dev:latest
```

The code can be build and tested with `cargo build` and `cargo test` once inside the docker image.

## Running the executable

The executable is configured via a YAML configuration file.

See the Config struct in [main.rs](./src/main.rs) for the options.

Example config file where the ground truth and proposal maps are given as GeoJSON files with LineString features:
```yaml
proposal_geofile_path: # Put your file here.
ground_truth:
  !Geofile
    filepath: # Put your file here.
topo_params:
  resampling_distance: 11.0
  hole_radius: 6.0
data_dir: # Intermediate files will be written here.
```

Example config where the ground truth is fetched from the OSM Overpass API:

```yaml
proposal_geofile_path: # Put your own file here.
ground_truth:
  !Osm
    bounding_box:
      left_lon: 139.788745
      right_lon: 139.792244
      bottom_lat: 35.683695
      top_lat: 35.685717
topo_params:
  resampling_distance: 11.0
  hole_radius: 6.0
data_dir: ./data
```

## Algorithm description

See [[1]](#references), section 5.2.1 for a detailed description of the algorithm.

## References

1. [RoadRunner: improving the precision of road network inference from GPS trajectories, Songtao et al.](https://dspace.mit.edu/handle/1721.1/137390)
