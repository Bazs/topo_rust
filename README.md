
[![CI](https://github.com/Bazs/topo_rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Bazs/topo_rust/actions/workflows/rust.yml)

---

# Rust implementation of the TOPO metric

## Usage

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
