# Blender_rs
This library will help download, install, and render blender files. This crate is a wrapper handler design to invoke blender and process outputs. This struct is capable of downloading blender from source and validate version integrity.

## Examples
I've composed a list of example what you can do with blender_rs. You must have at least a blender installed (min version 4.2.0) appended to the manager before running render example.

### Download
This example demonstrate downloading a copy of blender from the blender foundation organization, uncompressed the content, and return you a new struct containing blender path and version, with methods ready to render.

Run
```bash
cargo run --example manager exact-download <version>
# e.g.
cargo run --example manager exact-download 4.1.0
```

For more info, please read [here](./examples/manager/README.md).
