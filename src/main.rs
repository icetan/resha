use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use strict_yaml_rust::{StrictYaml as Yaml, StrictYamlLoader};
use walkdir::WalkDir;
use clap::Parser;

mod entry;
mod error;
mod manifest;

use crate::entry::{Entry, FromYaml, ReifySuccess};
use crate::error::{Error, Result};

/// Keep your generated and versioned files in sync
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
   /// Explicit manifest file to reify (can be given multiple times)
   #[arg(short, long)]
   manifest: Vec<PathBuf>,

   /// Manifest file name to match
   #[arg(long, default_value = ".rsha")]
   r#match: String,

   /// Recursively search for manifest files
   #[arg(short, long, default_value_t = false)]
   recurs: bool,

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
    let mut failed = false;
    let fail_fast = args.fail_fast;
    let dry_run = args.dry_run;

    println!("0..{}  # manifest {}", entries.len() - 1, path.display());

    for (i, e) in entries.iter().enumerate() {
        let name = e.name().clone().unwrap_or("<unnamed>".into());
        if fail_fast && failed {
            e.dump(&mut output, None)?;
            println!("ok {i} - {name}  # SKIP failing fast");
            continue;
        }

        if dry_run {
            e.dump(&mut output, None)?;
            match e.dry_run()? {
                Ok(_) => {
                    println!("ok {i} - {name}  # dry run");
                }
                Err(fail) => {
                    failed = true;
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
                failed = true;
                e.dump(&mut output, None)?;
                println!("not ok {i} - {name}  # {fail}");
            }
        }
    }

    Ok(manifest::ReifyStatus {
        output,
        success: !failed,
    })
}

fn find_manifests<'a>(root: &Path, name: &String) -> Vec<PathBuf> {
    let mut res = Vec::new();
    for file in WalkDir::new(root).into_iter().filter_map(|f| f.ok()) {
        if file.metadata().map(|m| m.is_file()).unwrap_or(false)
            && file.file_name() == name.as_str() {
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
    let files = if args.manifest.len() > 0 {
        args.manifest.clone()
    } else if args.recurs {
        find_manifests(Path::new("."), &args.r#match)
    } else {
        vec![Path::new(&args.r#match).canonicalize()?]
    };

    let files = files.iter()
        .map(|p| Ok(p.canonicalize()?))
        .collect::<Result<Vec<_>>>()?;

    let success = files.iter()
        .map(|p| match reify_and_update_manifest(&args, &p) {
            Ok(false) => {
                if args.fail_fast {
                    Err(Error::FailFastStop)
                } else {
                    Ok(false)
                }
            }
            rest => rest
        })
        .collect::<Result<Vec<_>>>()?
        .iter()
        .all(|x| x.clone());

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
