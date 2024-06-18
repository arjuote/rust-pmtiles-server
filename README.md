# rust-pmtiles-server

Lightweight and simple [PMTiles](https://github.com/protomaps/PMTiles) server implementation intended to be used as an AWS Lambda.

It is meant to be a lambda-friendly drop-in replacement for [tileserver-gl](https://github.com/maptiler/tileserver-gl) (limited to pmtiles format, however).

Being written in Rust it compiles into smallish, statically linked standalone binary that has very quick lambda cold starts, minimal memory footprint and good performance.

## Development

Say `cargo watch -x 'run -- --serve'` to run a development server with hot reloading at port `5000`.

NOTE: If you are testing S3 sources, keep in mind that SSO-login is not supported due to a feature flag in `aws-sdk` not being enabled by default in the `pmtiles-core` crate.
As a workaround one can expose the legacy credentials, e.g. with utilities like [yawsso](https://github.com/victorskl/yawsso).

## Build

Say `cargo build --release`.

## Run

Simplest way to run the server is to build the release binary and then say `pmtiles-server --serve`.

See `pmtiles-server --help` for additional options.

The server will default to reading the configuration file from `./config.json`. If you want to load configuration file from a different path, set the environment variable `CONFIG_PATH`. This variable can also point to an S3 location.

## Configuration

Configuration file structure mostly follows the logic of [tileserver-gl configuration](https://tileserver.readthedocs.io/en/latest/config.html) with non-pmtiles relevant stuff stripped off.

Notable differences:

- S3 paths are supported with the `s3://<bucket>/<prefix>/<key>` syntax
- `home` attribute can be set to support serving at non-root domain path, e.g. `https://example.com/tileserver`.

Example:

```json
{
  "options": {
    "paths": {
      "home": "tileserver",
      "root": "s3://example-bucket/tiledata",
      "fonts": "fonts",
      "sprites": null,
      "icons": null,
      "styles": "styles",
      "pmtiles": "pmtiles"
    },
    "domains": ["https://example.com"]
  },
  "styles": {
    "mydata": {
      "style": "mydata.json"
    }
  },
  "data": {
    "mydata": {
      "pmtiles": "mydata.pmtiles"
    }
  }
}
```

The above configuration would:

- read pmtiles data from `s3://example-bucket/tiledata/pmtiles/mydata.pmtiles`
- read style files from `s3://example-bucket/tiledata/styles/mydata.json`
- exposes endpoint `/pmtiles/{z}/{x}/{y}.pbf` for fetching raw data
- exposes endpoint `/pmtiles` for fetching TileJSON description of the data
- exposes endpoint `/styles/mydata/style.json` for fetching Mapbox compatible style JSON
- advertises urls pointing at `https://example.com/tileserver/` in the rendered JSON files.

NOTE: the domain can also be overridden by `API_DOMAIN` environment variable, which is likely more convenient for real world production deployments.

## Deploy

### Containerized or bare server deployment

Deploy simply by building the release binary and dropping it alongside with the `config.json` to a container or a server.

### Lambda deployment

1. Install [cargo lambda](https://www.cargo-lambda.info/guide/installation.html) and Zig with your preferred method. Simple and portable: activate a Python venv with relatively new Python version and say `pip install cargo-lambda`
2. Say `cargo lambda build --release` to build the lambda bundle.
3. Say `cargo lambda deploy --role arn:aws:iam::123456789101:role/my-lambda-role --env-var CONFIG_PATH=s3://example-bucket/config.json`

Check out `cargo lambda --help` for more advanced settings.

Make sure that `my-lambda-role`:

- can be assumed by the lambda service
- has read privileges to the required S3 paths.

## Creating PMTiles archives

PMTiles datasets can be created from MBTiles datasets using the tools provided by https://github.com/protomaps/PMTiles

MBTiles datasets or Mapbox Vector Tile (MVT) data in general can be created from various geospatial formats using tools such as [tippecanoe](https://github.com/mapbox/tippecanoe), [GDAL](https://gdal.org/index.html) or PostGIS.

## Customizing and extending

This repository has two crates:

- `pmtiles-core` library crate that provides PMTiles parsing and the default Cache and Fetcher implementations
- `pmtiles-server` that provides a binary crate based on Axum server.

One can use the pmtile-server as an example and just import the `pmtiles-core` crate to gain full control on the server implementation.

The default cache is a simple in-memory cache that is used for caching _just_ the PMTiles archive headers. More advanced caching backends can be added by implementing the trait `pmtiles_core::cache::Cache`.

The default Fetcher implementation supports s3 and local paths. Support for more backends (e.g. azure) can be added by implementing the `pmtiles_core::fetcher::Fetcher` trait.
