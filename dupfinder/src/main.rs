use clap::{App, Arg};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path;

struct FileInfo {
    path: String,
    size: u64,
}

fn collect_files(dir: &path::Path, files: &mut Vec<FileInfo>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(FileInfo {
                    path: String::from(path.to_string_lossy()),
                    size: entry.metadata()?.len(),
                });
            } else if path.is_dir() {
                collect_files(&path, files)?;
            }
        }
    }
    return Ok(());
}

fn group_duplicates(file_infos: &[FileInfo]) -> Vec<Vec<&FileInfo>> {
    let mut groups: HashMap<u64, Vec<&FileInfo>> = HashMap::new();
    for file_info in file_infos {
        if let Some(group) = groups.get_mut(&file_info.size) {
            group.push(file_info);
        } else {
            groups.insert(file_info.size, vec![file_info]);
        }
    }
    let mut result = Vec::new();
    for (_, group) in groups.drain() {
        if group.len() > 1 {
            result.push(group)
        }
    }
    return result;
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
    let mut file_infos: Vec<FileInfo> = Vec::new();
    println!("Received {} args.", paths.len());
    for path in paths {
        match fs::metadata(path) {
            Ok(attr) => {
                if attr.is_file() {
                    file_infos.push(FileInfo {
                        path: String::from(path),
                        size: attr.len(),
                    });
                } else if attr.is_dir() {
                    collect_files(path::Path::new(path), &mut file_infos)?;
                } else {
                    eprintln!("Ignoring {}: neither file nor directory", path);
                }
            }
            Err(e) => eprintln!("Ignoring {}: {}", path, e),
        }
    }
    println!("Found {} files.", file_infos.len());
    let dup_groups = group_duplicates(&file_infos);
    println!("Found {} duplicate groups.", dup_groups.len());
    for group in dup_groups {
        for file_info in group {
            println!("{}", file_info.path);
        }
        println!();
    }
    return io::Result::Ok(());
}
