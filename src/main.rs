use std::env;
use std::fs;
use std::fmt;
use std::io;
use std::process::Command;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;

use thiserror::Error as ThisError;
use sha2::{Sha256, Digest};
use yaml_rust::{Yaml, YamlLoader};

#[derive(ThisError, Debug)]
enum Error {
    #[error("YAML file doesn't exist")]
    YamlFileDoesntExist,
    #[error("Can't load YAML from string")]
    LoadYaml(#[from] yaml_rust::ScanError),
    #[error("YAML node is not of type hash/map")]
    NotAMap,
    #[error("Config file is malformed")]
    ConfigMalformed,
    #[error("Can't parse hash from YAML")]
    MissingCmd,
    #[error("Can't parse hash from YAML")]
    Io(#[from] io::Error),
    #[error("Can't parse hash from YAML")]
    ConvertUTF8(#[from] FromUtf8Error),
    #[error("Unknown problem")]
    Unknown,
}

// #[derive(Debug)]
// struct Error(ErrorKind, String);

// impl error::Error for Error {}

// impl fmt::Display for Error {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "HczError: {:?}", self)
//     }
// }

// impl Error {
//     fn new(kind: ErrorKind) -> Error {
//         Error(kind, String::new())
//     }
// }

trait FromYaml: Sized {
    fn from_yaml(yaml: &Yaml) -> Result<Self, Error>;
}

#[derive(Debug)]
struct HczEntry {
    cmd: String,
    required_files: Vec<String>,
    files: Vec<String>,
    sha: Option<String>,
}

impl HczEntry {
    fn calc_sha(&self) -> Result<String, Error> {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];
        let mut all_files = self.files.iter()
            .chain(self.required_files.iter())
            .collect::<Vec<_>>();
        all_files.sort();
        for file in all_files {
            let input = File::open(&file)?;

            let mut reader = BufReader::new(input);

            loop {
                let count = reader.read(&mut buffer)?;
                if count == 0 { break }
                hasher.update(&buffer[..count]);
            }
        }
        hasher.update(&self.cmd);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn exec(&self) -> Result<String, Error> {
        Ok(Command::new("/bin/bash")
            .arg("-c")
            .arg(&self.cmd)
            .output()
            .map_err(Error::Io)
            .and_then(|o| String::from_utf8(o.stdout).map_err(Error::ConvertUTF8))?)
    }

    fn run(&self) -> Result<(), Error> {
        println!("{}", self.calc_sha()?);
        println!("{}",  self.exec()?);
        Ok(())
    }
}

impl fmt::Display for HczEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromYaml for HczEntry {
    fn from_yaml(yaml: &Yaml) -> Result<Self, Error> {
        match yaml {
            Yaml::Hash(hash) => {
                let cmd = hash
                    .get(&Yaml::from_str("cmd"))
                    .and_then(Yaml::as_str)
                    .map(String::from)
                    .ok_or(Error::MissingCmd)?;

                let sha = hash
                    .get(&Yaml::from_str("sha"))
                    .and_then(Yaml::as_str)
                    .map(String::from);

                let files = hash
                    .get(&Yaml::from_str("files"))
                    .map(str_vec)
                    .unwrap_or(Vec::new());

                let required_files = hash
                    .get(&Yaml::from_str("required_files"))
                    .map(str_vec)
                    .unwrap_or(Vec::new());

                Ok(Self {
                    cmd,
                    sha,
                    files,
                    required_files,
                })
            }
            _ => Err(Error::NotAMap),
        }
    }
}

fn canonicalize(p: &String) -> Option<PathBuf> {
    Path::new(p).canonicalize().ok()
}

fn path_vec(y: &Yaml) -> Vec<PathBuf> {
    str_vec(y).iter()
        .filter_map(canonicalize)
        .collect::<Vec<_>>()
}

fn str_vec(y: &Yaml) -> Vec<String> {
    match y {
        Yaml::Array(x) => x.iter()
            .filter_map(Yaml::as_str)
            .map(String::from)
            .collect::<Vec<_>>(),
        Yaml::String(x) => vec![x.into()],
        _ => vec![],
    }
}

fn print_result(r: Result<impl fmt::Display, impl fmt::Debug>) {
    match r {
        Ok(s) => eprintln!("{}", s),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

fn read_yaml(path: &Path) -> Result<Vec<Yaml>, Error> {
    let yaml_str = fs::read_to_string(&path)?;
    let yaml = YamlLoader::load_from_str(&yaml_str)?;
    Ok(yaml)
}

fn parse_entries(yaml: &Yaml) -> Result<Vec<HczEntry>, Error> {
    match yaml {
        Yaml::Array(ys) => Ok(ys.iter()
            .flat_map(HczEntry::from_yaml)
            .collect::<Vec<_>>()
        ),
        _ => Err(Error::ConfigMalformed)
    }
}

fn start() -> Result<(), Error> {
    let file = env::args().nth(1)
        .unwrap_or(".hcz.yaml".into());

    let path = canonicalize(&file)
        .ok_or(Error::YamlFileDoesntExist)?;

    println!("Reading file {}", path.display());

    let wd = path.parent()
        .ok_or(Error::Unknown)?;

    env::set_current_dir(wd)?;

    let entries = read_yaml(&path)
        .and_then(|v| v.get(0)
            .ok_or(Error::ConfigMalformed)
            .and_then(parse_entries)
        )?;

    for e in entries {
        e.run()?;
    };

    Ok(())
}

fn main() {
    match start() {
        Err(e) => { eprintln!("Error: {}", e); }
        _ => {}
    };
}
