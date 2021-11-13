use clap::{value_t, App, Arg};
use itertools::Itertools;
use regex::Regex;
use ring::digest::{Context, SHA256};
use std::collections::{HashMap, HashSet};
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
    // Print statistics about duplicates found.
    verbose: bool,
    // Minimum file size. Ignore all smaller files.
    min_size: u64,
    // Compare files only based on fingerprint.
    // May yield false positives, but is *much* faster.
    quick_scan: bool,
    // Number of bytes to read as file fingerprint.
    fp_bytes: usize,
    // Include only files with the given extensions.
    included_extensions: HashSet<String>,
    // Only include files matching path_regex.
    path_regex: Option<Regex>,
}

const IMAGE_EXTS: &[&str] = &[
    // The common image format (exluding RAW):
    "jpg", "jpeg", "png", "gif", "bmp", "tif", "tiff", //
    // RAW formats
    "dng", "raw", "rw2", "raf", "nef", "nrw", "arw", "sr2", "cr2", "orf", //
    // Photo editors
    "psd", "xcf", //
    // Video files
    "mov", "mp4", ".avi",
];

fn accept_file(path: &path::Path, md: &fs::Metadata, run_opts: &RunOptions) -> bool {
    if md.len() < run_opts.min_size {
        return false;
    }
    if !run_opts.included_extensions.is_empty() {
        if let Some(ext) = path.extension() {
            if !run_opts
                .included_extensions
                .contains(&ext.to_str().unwrap_or("").to_lowercase())
            {
                return false;
            }
        } else {
            return false;
        }
    }
    if let Some(re) = &run_opts.path_regex {
        if !re.is_match(&path.to_string_lossy()) {
            return false;
        }
    }
    return true;
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
            if accept_file(&path, &md, run_opts) {
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

fn non_singleton_groups_by<'a, KeyFn, T: Eq + Hash>(
    file_infos: &[&'a FileInfo],
    key_fn: KeyFn,
) -> io::Result<Vec<Vec<&'a FileInfo>>>
where
    KeyFn: Fn(&FileInfo) -> io::Result<T>,
{
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

fn file_fp(file_info: &FileInfo, fp_size: usize) -> io::Result<Vec<u8>> {
    let mut f_in = File::open(&file_info.path)?;
    if file_info.size > 2 * fp_size as u64 {
        f_in.seek(io::SeekFrom::Start(file_info.size / 2))?;
    }
    let mut buf = vec![0; fp_size];
    f_in.read(&mut buf)?;
    return Ok(buf);
}

type ShaChecksum = [u8; 32];

fn file_sha256(file_info: &FileInfo) -> io::Result<ShaChecksum> {
    const CHUNK_SIZE: usize = 100 * 1024;
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
    if run_opts.verbose {
        let size: usize = by_size.iter().map(|g| g.len()).sum();
        println!("Duplicates by size: {}", size);
    }
    let mut by_fp = Vec::new();
    for group in by_size {
        by_fp.extend(non_singleton_groups_by(&group, |g| {
            file_fp(g, run_opts.fp_bytes)
        })?);
    }
    if run_opts.verbose {
        let size: usize = by_fp.iter().map(|g| g.len()).sum();
        println!("Duplicates by fingerprint: {}", size);
    }
    if run_opts.quick_scan {
        // Skip SHA256 checksums on quick scan.
        return Ok(by_fp);
    }
    let mut by_hash = Vec::new();
    let mut fp_misses = 0;
    for group in by_fp {
        let hash_groups = non_singleton_groups_by(&group, file_sha256)?;
        if hash_groups.len() != 1 || group.len() != hash_groups[0].len() {
            fp_misses += 1;
            if run_opts.verbose {
                println!(
                    "SHA256 differs from fingerprint result: {}",
                    group.iter().map(|f| f.path.to_string_lossy()).format(", ")
                );
            }
        }
        by_hash.extend(hash_groups);
    }
    if run_opts.verbose {
        let size: usize = by_hash.iter().map(|g| g.len()).sum();
        println!(
            "Duplicates by sha256: {} ({} corrections to fingerprint)",
            size, fp_misses
        );
    }
    return Ok(by_hash);
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
            Arg::with_name("fp-bytes")
                .short("f")
                .long("fp-bytes")
                .help("Number of bytes to read for file fingerprint")
                .default_value("4096")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Print verbose (debug) output"),
        )
        .arg(
            Arg::with_name("quick-scan")
                .short("q")
                .long("quick-scan")
                .help("Only compare file fingerprints, skip checksums"),
        )
        .arg(
            Arg::with_name("images")
                .short("i")
                .long("images")
                .help("Only compare files having an image file type extension (e.g. JPG)"),
        )
        .arg(
            Arg::with_name("path-regex")
                .short("p")
                .long("path-regex")
                .help("Only compare files matching regular expression")
                .takes_value(true),
        )
        .get_matches();
    let paths = matches
        .values_of("paths")
        .expect("paths are a required argument");
    let path_regex = if matches.is_present("path-regex") {
        match Regex::new(matches.value_of("path-regex").unwrap()) {
            Ok(re) => Some(re),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    } else {
        None
    };
    let run_opts = RunOptions {
        verbose: matches.is_present("verbose"),
        min_size: value_t!(matches.value_of("min-size"), u64).unwrap_or_else(|e| e.exit()),
        quick_scan: matches.is_present("quick-scan"),
        fp_bytes: value_t!(matches.value_of("fp-bytes"), usize).unwrap_or_else(|e| e.exit()),
        included_extensions: IMAGE_EXTS.iter().map(|s| String::from(*s)).collect(),
        path_regex: path_regex,
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
        for (i, file_info) in group.iter().enumerate() {
            println!("{}", file_info.path.to_string_lossy());
            if i > 0 {
                dup_size += file_info.size;
            }
        }
        println!();
    }
    if run_opts.verbose {
        println!(
            "Found {} files ({} bytes) and {} duplicate groups ({} redundant bytes).",
            file_infos.len(),
            total_size,
            dup_groups.len(),
            dup_size,
        );
    }
    return io::Result::Ok(());
}
