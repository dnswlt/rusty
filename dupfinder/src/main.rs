use clap::{App, Arg};
use std::fs;
use std::io;
use std::path;

fn collect_files(dir: &path::Path, files: &mut Vec<String>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(String::from(path.to_string_lossy()));
            } else if path.is_dir() {
                collect_files(&path, files)?;
            }
        }
    }
    return Ok(());
}

fn main() -> io::Result<()> {
    let matches = App::new("dupfinder")
        .version("0.1")
        .author("Dennis Walter <dennis.walter@gmail.com>")
        .about("Find duplicate files.")
        .arg(Arg::with_name("paths").multiple(true).required(true))
        .get_matches();
    let paths = matches
        .values_of("paths")
        .expect("paths are a required argument");
    let mut files: Vec<String> = Vec::new();
    println!("Received {} args.", paths.len());
    for path in paths {
        match fs::metadata(path) {
            Ok(attr) if attr.is_file() => {
                println!("{} is a file", path);
                files.push(String::from(path));
            }
            Ok(attr) if attr.is_dir() => {
                println!("{} is a directory", path);
                // let dir_contents = ;
                collect_files(path::Path::new(path), &mut files)?;
            }
            Ok(_) => println!("Ignoring {}: neither file nor directory", path),
            Err(e) => println!("Ignoring {}: {}", path, e),
        }
    }
    println!("Found {} files.", files.len());
    return io::Result::Ok(());
}
