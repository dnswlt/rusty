use clap::{value_t, App, Arg};
use ring::digest::{Context, SHA256};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::io::prelude::*;
use std::path;

struct FileInfo {
    path: path::PathBuf,
    size: u64,
}

struct RunOptions {
    verbose: bool,
    min_size: u64,
}

fn collect_files(
    dir: &path::Path,
    files: &mut Vec<FileInfo>,
    run_opts: &RunOptions,
) -> io::Result<()> {
    if !dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Not a directory: {}", dir.to_string_lossy()),
        ));
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let md = entry.metadata()?;
            if md.len() >= run_opts.min_size {
                files.push(FileInfo {
                    path: path,
                    size: md.len(),
                });
            }
        } else if path.is_dir() {
            collect_files(&path, files, run_opts)?;
        }
    }
    return Ok(());
}

type FileInfoKeyFn<T> = fn(&FileInfo) -> io::Result<T>;

fn non_singleton_groups_by<'a, T: Eq + Hash>(
    file_infos: &[&'a FileInfo],
    key_fn: FileInfoKeyFn<T>,
) -> io::Result<Vec<Vec<&'a FileInfo>>> {
    let mut groups: HashMap<T, Vec<&FileInfo>> = HashMap::new();
    for file_info in file_infos {
        let key = key_fn(file_info)?;
        if let Some(group) = groups.get_mut(&key) {
            group.push(file_info);
        } else {
            groups.insert(key, vec![file_info]);
        }
    }
    let mut result = Vec::new();
    for (_, group) in groups {
        if group.len() > 1 {
            result.push(group);
        }
    }
    return Ok(result);
}

fn file_fp(file_info: &FileInfo) -> io::Result<Vec<u8>> {
    const FP_SIZE: u64 = 1024;
    let mut f_in = File::open(&file_info.path)?;
    if file_info.size > 2 * FP_SIZE {
        f_in.seek(io::SeekFrom::Start(file_info.size / 2))?;
    }
    let mut buf = vec![0; FP_SIZE as usize];
    f_in.read(&mut buf)?;
    return Ok(buf);
}

fn file_sha256(file_info: &FileInfo) -> io::Result<[u8; 32]> {
    const CHUNK_SIZE: usize = 1024;
    let mut context = Context::new(&SHA256);
    let mut f_in = File::open(&file_info.path)?;
    let mut buf: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];
    loop {
        let num_read = f_in.read(&mut buf)?;
        if num_read == 0 {
            break;
        } else {
            context.update(&buf[..num_read]);
        }
    }
    return Ok(context
        .finish()
        .as_ref()
        .try_into()
        .expect("Unexpected digest size"));
}

fn group_duplicates<'a>(
    file_infos: &'a [FileInfo],
    run_opts: &RunOptions,
) -> io::Result<Vec<Vec<&'a FileInfo>>> {
    let fi: Vec<&'a FileInfo> = file_infos.iter().collect();
    let by_size = non_singleton_groups_by(&fi, |f| Ok(f.size))?;
    let mut num_by_size = 0;
    let mut by_fp = Vec::new();
    for group in by_size {
        num_by_size += group.len();
        by_fp.extend(non_singleton_groups_by(&group, file_fp)?);
    }
    let mut num_by_fp = 0;
    let mut by_hash = Vec::new();
    for group in by_fp {
        num_by_fp += group.len();
        by_hash.extend(non_singleton_groups_by(&group, file_sha256)?);
    }
    let mut num_by_hash = 0;
    for group in &by_hash {
        num_by_hash += group.len();
    }
    if run_opts.verbose {
        println!(
            "Duplicates\n\tby_size: {}\n\tby_fp:{}\n\tby_sha256:{}",
            num_by_size, num_by_fp, num_by_hash
        );
    }
    return Ok(by_hash);
}

fn _group_duplicates<'a>(
    file_infos: &'a [FileInfo],
    run_opts: &RunOptions,
) -> io::Result<Vec<Vec<&'a FileInfo>>> {
    let mut result = Vec::new();
    let mut num_groups_by_size = 0;
    let mut num_groups_by_fp = 0;
    let mut num_groups_by_hash = 0;
    let mut groups: HashMap<u64, Vec<&FileInfo>> = HashMap::new();
    // First, group by file size.
    for file_info in file_infos {
        if let Some(group) = groups.get_mut(&file_info.size) {
            group.push(file_info);
        } else {
            groups.insert(file_info.size, vec![file_info]);
        }
    }
    for (_, group) in groups {
        // Each group that is not a singleton must be further analyzed.
        if group.len() > 1 {
            num_groups_by_size += 1;
            // Group by file "fingerprint" (some bytes from the middle of the file).
            const FP_SIZE: u64 = 1024;
            let mut same_fps: HashMap<[u8; FP_SIZE as usize], Vec<&FileInfo>> = HashMap::new();
            for file_info in group {
                let mut f_in = File::open(&file_info.path)?;
                if file_info.size > 2 * FP_SIZE {
                    f_in.seek(io::SeekFrom::Start(file_info.size / 2))?;
                }
                let mut buf: [u8; 1024] = [0; 1024];
                f_in.read(&mut buf)?;
                if let Some(same_fp) = same_fps.get_mut(&buf) {
                    same_fp.push(file_info);
                } else {
                    same_fps.insert(buf, vec![file_info]);
                }
            }
            for (_, same_fp) in same_fps {
                if same_fp.len() > 1 {
                    num_groups_by_fp += 1;
                    let mut same_hashes: HashMap<[u8; 32], Vec<&FileInfo>> = HashMap::new();
                    for file_info in same_fp {
                        let mut context = Context::new(&SHA256);
                        let mut f_in = File::open(&file_info.path)?;
                        let mut buf: [u8; 1024] = [0; 1024];
                        loop {
                            let num_read = f_in.read(&mut buf)?;
                            if num_read == 0 {
                                break;
                            } else {
                                context.update(&buf[..num_read]);
                            }
                        }
                        let hash = context
                            .finish()
                            .as_ref()
                            .try_into()
                            .expect("Unexpected digest size");
                        if let Some(same_hash) = same_hashes.get_mut(&hash) {
                            same_hash.push(file_info);
                        } else {
                            same_hashes.insert(hash, vec![file_info]);
                        }
                    }
                    for (_, same_hash) in same_hashes {
                        if same_hash.len() > 1 {
                            num_groups_by_hash += 1;
                            result.push(same_hash);
                        }
                    }
                }
            }
        }
    }
    if run_opts.verbose {
        println!(
            "Duplicate groups:\n\tby size: {}\n\tby file_fp: {}\n\tby hash: {}",
            num_groups_by_size, num_groups_by_fp, num_groups_by_hash
        );
    }
    return Ok(result);
}

fn main() -> io::Result<()> {
    let matches = App::new("dupfinder")
        .version("0.1")
        .author("Dennis Walter <dennis.walter@gmail.com>")
        .about("Find duplicate files.")
        .arg(Arg::with_name("paths").multiple(true).required(true))
        .arg(
            Arg::with_name("min-size")
                .short("s")
                .long("min-size")
                .help("Minimum size of files considered")
                .default_value("0")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Print verbose (debug) output"),
        )
        .get_matches();
    let paths = matches
        .values_of("paths")
        .expect("paths are a required argument");
    let min_size = value_t!(matches.value_of("min-size"), u64).unwrap_or_else(|e| e.exit());
    let verbose = matches.is_present("verbose");
    let run_opts = RunOptions {
        verbose: verbose,
        min_size: min_size,
    };
    let mut file_infos: Vec<FileInfo> = Vec::new();
    for path in paths {
        match fs::metadata(path) {
            Ok(attr) => {
                if attr.is_file() && attr.len() >= run_opts.min_size {
                    file_infos.push(FileInfo {
                        path: path::PathBuf::from(path),
                        size: attr.len(),
                    });
                } else if attr.is_dir() {
                    collect_files(path::Path::new(path), &mut file_infos, &run_opts)?;
                } else {
                    eprintln!("Ignoring {}: neither file nor directory", path);
                }
            }
            Err(e) => eprintln!("Ignoring {}: {}", path, e),
        }
    }
    let total_size: u64 = file_infos.iter().map(|e| e.size).sum();
    let dup_groups = group_duplicates(&file_infos, &run_opts)?;
    let mut dup_size: u64 = 0;
    for group in &dup_groups {
        for file_info in group {
            println!("{}", file_info.path.to_string_lossy());
            dup_size += file_info.size;
        }
        println!();
    }
    if run_opts.verbose {
        println!(
            "Found {} files ({} bytes) and {} duplicate groups ({} bytes).",
            file_infos.len(),
            total_size,
            dup_groups.len(),
            dup_size,
        );
    }
    return io::Result::Ok(());
}
