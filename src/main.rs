use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use strict_yaml_rust::{StrictYaml as Yaml, StrictYamlLoader};
use walkdir::WalkDir;

mod entry;
mod error;
mod manifest;

use crate::entry::{Entry, FromYaml, ReifySuccess};
use crate::error::{Error, Result};

/// Keep your generated and versioned files in sync
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, trailing_var_arg = true)]
struct Args {
    /// Explicit manifest files to reify
    #[arg(
        allow_hyphen_values = true,
        conflicts_with("recursive"),
        conflicts_with("match")
    )]
    manifests: Vec<PathBuf>,

    /// Manifest file name to match
    #[arg(short, long, default_value(".rsha.yml"))]
    r#match: String,

    /// Recursively search for manifest files
    #[arg(short, long, default_value_t = false)]
    recursive: bool,

    /// Skip entries after failed check
    #[arg(short, long, default_value_t = false)]
    fail_fast: bool,

    /// Dry run
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
}

fn parse_entries(yaml: &Yaml) -> Result<Vec<Entry>> {
    yaml.as_vec()
        .ok_or(Error::ManifestMalformed)
        .and_then(|ys| ys.iter().map(Entry::from_yaml).collect::<Result<Vec<_>>>())
}

fn parse_manifest(path: &Path) -> Result<Vec<Entry>> {
    let yaml_str = fs::read_to_string(&path)?;
    let docs = StrictYamlLoader::load_from_str(&yaml_str)?;
    let yaml = docs.get(0).ok_or(Error::ManifestMalformed)?;
    parse_entries(yaml)
}

fn reify_manifest(args: &Args, path: &Path) -> Result<manifest::ReifyStatus> {
    let wd = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;

    env::set_current_dir(wd)?;

    let entries = parse_manifest(&path)?;

    let mut output = String::new();
    let mut success = true;
    let fail_fast = args.fail_fast;
    let dry_run = args.dry_run;

    println!("0..{}  # manifest {}", entries.len() - 1, path.display());

    for (i, e) in entries.iter().enumerate() {
        let name = e.name().clone().unwrap_or("<unnamed>".into());
        if fail_fast && !success {
            e.dump(&mut output, None)?;
            println!("ok {i} - {name}  # SKIP (fail fast)");
            continue;
        }

        if dry_run {
            e.dump(&mut output, None)?;
            match e.dry_run()? {
                Ok(_) => {
                    println!("ok {i} - {name}  # dry run");
                }
                Err(fail) => {
                    success = false;
                    println!("not ok {i} - {name}  # {fail}");
                }
            }
            continue;
        }

        match e.reify()? {
            Ok(ReifySuccess::ExecSuccess(sha)) => {
                e.dump(&mut output, Some(sha))?;
                println!("ok {i} - {name}");
            }
            Ok(ReifySuccess::Noop) => {
                e.dump(&mut output, None)?;
                println!("ok {i} - {name}  # noop");
            }
            Err(fail) => {
                success = false;
                e.dump(&mut output, None)?;
                println!("not ok {i} - {name}  # {fail}");
            }
        }
    }

    Ok(manifest::ReifyStatus { output, success })
}

fn find_manifests(root: &Path, name: &String, recursive: bool) -> Vec<PathBuf> {
    let mut res = Vec::new();

    let walk = WalkDir::new(root);
    let walk = if recursive { walk } else { walk.max_depth(1) };

    for file in walk.into_iter().filter_map(|f| f.ok()) {
        if file.metadata().map(|m| m.is_file()).unwrap_or(false)
            && file.file_name() == name.as_str()
        {
            res.push(file.path().to_path_buf());
        }
    }
    res
}

fn reify_and_update_manifest(args: &Args, path: &Path) -> Result<bool> {
    let status = reify_manifest(&args, &path)?;
    fs::write(&path, status.output())?;
    Ok(status.success())
}

fn start(args: &Args) -> Result<bool> {
    let files = if args.manifests.len() > 0 {
        args.manifests.clone()
    } else {
        find_manifests(Path::new("."), &args.r#match, args.recursive)
    };

    let files = files
        .iter()
        .map(|p| {
            p.canonicalize()
                .map_err(|_| Error::ManifestFileDoesntExist(p.display().to_string()))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut success = true;

    for path in files {
        if args.fail_fast && !success {
            println!("# skipping manifest {} (fail fast)", path.display());
            continue;
        }
        if !reify_and_update_manifest(&args, &path)? {
            success = false;
        }
    }

    Ok(success)
}

fn main() {
    let args = Args::parse();

    let success = match start(&args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            false
        }
    };

    if !success {
        std::process::exit(1);
    }
}
