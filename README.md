# rsha

Keep your generated and versioned files in sync

## How it Works

`rsha` reads a manifest file and for each entry it will hash the files listed in
`files` and `required_files` together with the `cmd` attribute. If the hash
doesn't match the `sha` attribute the `cmd` shell script will be run. If the
script runs successfuly a new hash will be generated and the `sha` attribute
will be updated.

It is fast to check if files are in sync on CI and easy for developers to
re-sync files when things change, beacase the hash is saved in the manifest
file and versioned (e.g. with git) along the input files.

## Usage

<!--p[cargo run -- --help]-->
```
Keep your generated and versioned files in sync

Usage: rsha [OPTIONS] [MANIFESTS]...

Arguments:
  [MANIFESTS]...  Explicit manifest files to reify

Options:
  -m, --match <MATCH>  Manifest file name to match [default: .rsha.yml]
  -r, --recursive      Recursively search for manifest files
  -f, --fail-fast      Skip entries after failed check
  -d, --dry-run        Dry run
  -h, --help           Print help
  -V, --version        Print version

```
<!--END[]-->

## Example Manifest

Regenerates rust files from protobuf when either the `.proto` or the rust
output has changed from the last `rsha` run.

```yaml
-
  name: Update generated protobuf files
  cmd: |
    protoc --rust_out=src/protos --proto_path=protobuf protobuf/model.proto
  required_files:
  - protobuf/model.proto
  files:
  - src/protos/model.rs
```
