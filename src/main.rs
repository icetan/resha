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
    #[arg(long, default_value(".rsha.yml"))]
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

    /// Print input files
    #[arg(short = 'i', long, default_value_t = false)]
    print_inputs: bool,

    /// Print manifest files
    #[arg(short = 'm', long, default_value_t = false)]
    print_manifests: bool,

    /// Only print files from reified entries
    #[arg(short, long, default_value_t = false)]
    only_print_reified: bool,

    /// Hide execution output
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
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

fn reify_manifest(
    args: &Args,
    path: &Path,
    output: &mut dyn std::fmt::Write,
    prev_success: bool,
) -> Result<manifest::ReifyStatus> {
    let print_files = |e: &Entry, success: bool| {
        if args.print_inputs && (!args.only_print_reified || success) {
            for path in e.all_files() {
                println!("{}", path.display());
            }
        }
    };

    let print_tap = !args.print_inputs && !args.print_manifests;

    let wd = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;

    env::set_current_dir(wd)?;

    let entries = parse_manifest(&path)?;

    let mut success = prev_success;
    let mut print_man = false;

    if print_tap {
        println!("1..{}  # manifest {}", entries.len(), path.display())
    }

    for (i, e) in entries.iter().enumerate() {
        let i = i + 1;
        let name = e.name().clone().unwrap_or("<unnamed>".into());

        if args.fail_fast && !success {
            if !args.dry_run {
                e.dump(output, None)?;
            }
            print_files(e, false);
            if print_tap {
                println!("ok {i} - {name}  # SKIP (fail fast)");
            }
            continue;
        }

        if args.dry_run {
            match e.dry_run()? {
                Ok(_) => {
                    print_man = true;
                    print_files(e, true);
                    if print_tap {
                        println!("ok {i} - {name}  # dry run");
                    }
                }
                Err(fail) => {
                    success = false;
                    print_files(e, false);
                    if print_tap {
                        println!("not ok {i} - {name}  # {fail}");
                    }
                }
            }
            continue;
        }

        let reify_status = if !args.quiet {
            e.reify(&mut std::io::stderr())
        } else {
            e.reify(&mut std::io::sink())
        };

        match reify_status? {
            Ok(ReifySuccess::ExecSuccess(sha)) => {
                print_man = true;
                e.dump(output, Some(sha))?;
                print_files(e, true);
                if print_tap {
                    println!("ok {i} - {name}");
                }
            }
            Ok(ReifySuccess::Noop) => {
                e.dump(output, None)?;
                print_files(e, false);
                if print_tap {
                    println!("ok {i} - {name}  # noop");
                }
            }
            Err(fail) => {
                success = false;
                e.dump(output, None)?;
                print_files(e, false);
                if print_tap {
                    println!("not ok {i} - {name}  # {fail}");
                }
            }
        }
    }

    if args.print_manifests && (!args.only_print_reified || print_man) {
        println!("{}", path.display());
    }

    Ok(manifest::ReifyStatus { success })
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
        // if args.fail_fast && !success {
        //     println!("# skipping manifest {} (fail fast)", path.display());
        //     continue;
        // }
        let mut output = String::new();

        if !reify_manifest(&args, &path, &mut output, success)?.success() {
            success = false;
        }

        // Don't write back to manifest file if dry run
        if !args.dry_run {
            fs::write(&path, &output)?;
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
