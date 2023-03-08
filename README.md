
[![CI - CD](https://github.com/Bazs/topo_rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Bazs/topo_rust/actions/workflows/rust.yml)

---

# Rust implementation of the TOPO metric

To use, either build from source or use the Docker image.

## Build from source

Besides the rust native dependencies expressed in the Cargo.lock file, the project depends on GDAL version 2.4-3.6 
being installed locally. See https://gdal.org/download.html for download details for your platform.

Please make sure to build the binary using `cargo build --release`, otherwise the runtime performance will be slow.

## Use the docker image

Check the https://hub.docker.com/repository/docker/balazsopra/topo_rust/general repository for the Docker latest image.
The executable is in the default work dir inside the image.

## Running the executable

The executable is configured via a YAML configuration file.

See the Config struct in [main.rs](./src/main.rs) for the options.

Example config file where the ground truth and proposal maps are given as GeoJSON files with LineString features:
```yaml
proposal_geojson_path: # Put your file here.
ground_truth:
  !Geojson
    filepath: # Put your file here.
data_dir: # Intermediate files will be written here.
```

Example config where the ground truth is fetched from the OSM Overpass API:

```yaml
proposal_geojson_path: # Put your own file here.
ground_truth:
  !Osm
    bounding_box:
      left_lon: 139.788745
      right_lon: 139.792244
      bottom_lat: 35.683695
      top_lat: 35.685717
data_dir: ./data
```

## Algorithm description

See [[1]](#references), section 5.2.1 for a detailed description of the algorithm.

## References

1. [RoadRunner: improving the precision of road network inference from GPS trajectories, Songtao et al.](https://dspace.mit.edu/handle/1721.1/137390)
