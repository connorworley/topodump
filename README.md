# topodump
Convert tpq files to GeoTIFF format

## Installing
```
cargo install --git https://github.com/connorworley/topodump
```

## Usage
```
topodump 1.0.0
Connor Worley <connorbworley@gmail.com>
Convert tpq files to GeoTIFF format

USAGE:
    topodump <input> <output>

ARGS:
    <input>
    <output>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

Example:
```
topodump AZ_D02/D34113/N34113G3.tpq N34113G3.tif
```

## Credits
Significantly informed by Thomas J. Trebisky's [gtopo](https://github.com/trebisky/gtopo).
