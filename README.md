# resha

Keep your generated and versioned files in sync

## How it Works

`resha` reads a manifest file and for each entry it will hash the files listed in
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

Usage: resha [OPTIONS] [MANIFESTS]...

Arguments:
  [MANIFESTS]...  Explicit manifest files to reify [env: RESHA_MANIFEST=]

Options:
      --match <MATCH>       Manifest file name to match [env: RESHA_MATCH=] [default: ^.resha.ya?ml$]
  -r, --recursive           Recursively search for manifest files [env: RESHA_RECURSIVE=]
  -f, --fail-fast           Skip entries after failed check [env: RESHA_FAIL_FAST=]
  -d, --dry-run             Dry run [env: RESHA_DRY_RUN=]
  -i, --print-inputs        Print input files
  -m, --print-manifests     Print manifest files
  -o, --only-print-reified  Only print files from reified entries
  -q, --quiet               Hide execution output [env: RESHA_QUIET=]
  -h, --help                Print help
  -V, --version             Print version

```
<!--END[]-->

## Example Manifest

Regenerates rust files from protobuf when either the `.proto` or the rust
output has changed from the last `resha` run.

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
