This library crate hold the protobuf definitions for the gRPC services used by the project, and generate and export the gRPC client and server stubs for the services defined in the protobuf files.

## Add new gRPC

1. Add new protobuf file in `proto/` directory.
2. Modify `build.rs` to add the new protobuf file to the build script.
3. Modify `lib.rs` to export the new gRPC client and server stubs.

## How to use this in other crates (in this workspace)

Add `gengrpc = {path = "../gengrpc"}` into that crate's `Cargo.toml` file. The generate code should be available to use in that crate as `use gengrpc::<something>;`
