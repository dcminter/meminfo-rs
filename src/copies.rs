use procfs::process::FDTarget::Path;
use procfs::process::{FDInfo, FDPermissions, Process};
use procfs::ProcResult;
use regex::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

static INTERESTING_FILENAMES: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    vec![
        "cp",
        "mv",
        "dd",
        "tar",
        "bsdtar",
        "cat",
        "rsync",
        "scp",
        "grep",
        "fgrep",
        "egrep",
        "cut",
        "sort",
        "cksum",
        "md5sum",
        "sha1sum",
        "sha224sum",
        "sha256sum",
        "sha384sum",
        "sha512sum",
        "adb",
        "gzip",
        "gunzip",
        "bzip2",
        "bunzip2",
        "xz",
        "unxz",
        "lzma",
        "unlzma",
        "7z",
        "7za",
        "zip",
        "unzip",
        "zcat",
        "bzcat",
        "lzcat",
        "coreutils",
        "split",
        "gpg",
    ]
    .into_iter()
    .collect()
});

pub struct CopyFile {
    pub path: PathBuf,
    pub length: u64,
    pub offset: u64,
}

pub struct CopyStatus {
    pub pid: i32,
    pub source: Option<CopyFile>,
    pub target: Option<CopyFile>,
}

pub fn file_copy_info() -> Result<Vec<CopyStatus>, Box<dyn Error>> {
    let mut copy_statuses: Vec<CopyStatus> = Vec::new();
    for process_result in procfs::process::all_processes()? {
        process_to_copy_status(&mut copy_statuses, process_result)?;
    }
    Ok(copy_statuses)
}

fn process_to_copy_status(
    copy_statuses: &mut Vec<CopyStatus>,
    process_result: ProcResult<Process>,
) -> Result<(), Box<dyn Error>> {
    let process = process_result?;
    match process.exe() {
        Ok(exe_path) => match executable_process_to_copy_status(process, exe_path)? {
            Some(copy_status) => {
                copy_statuses.push(copy_status);
                Ok(())
            }
            None => Ok(()),
        },
        Err(_) => {
            // Generally this is a completely expected path; most of the time we're not the
            // process owner so we don't have permission to access it!
            Ok(())
        }
    }
}

fn executable_process_to_copy_status(
    process: Process,
    exe_path: PathBuf,
) -> Result<Option<CopyStatus>, Box<dyn Error>> {
    if interesting_exe_path(exe_path.file_name()) {
        let mut copy = CopyStatus {
            pid: process.pid,
            source: None,
            target: None,
        };

        let fd_iter = process.fd()?;
        for fd_result in fd_iter {
            let fd = fd_result?;
            let metadata = FDMetadata::get_fd_metadata(process.pid, &fd)?;
            if fd.fd > 2 {
                // stdin, stdout, and stderr aren't of interest.
                match &fd.target {
                    Path(path) => {
                        let permissions = &fd.mode();
                        if permissions.contains(FDPermissions::READ) {
                            // Ok, this is the source file!
                            let meta = fs::metadata(&path)?;
                            copy.source = Some(CopyFile {
                                path: path.clone(),
                                length: meta.len(),
                                offset: metadata.pos,
                            });
                        } else if permissions.contains(FDPermissions::WRITE) {
                            // Ok, this is the target file!
                            let meta = fs::metadata(&path)?;
                            copy.target = Some(CopyFile {
                                path: path.clone(),
                                length: meta.len(),
                                offset: metadata.pos,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(Some(copy))
    } else {
        Ok(None)
    }
}

fn interesting_exe_path(exe_path: Option<&OsStr>) -> bool {
    match exe_path {
        Some(path) => match path.to_str() {
            Some(path) => INTERESTING_FILENAMES.contains(path),
            None => false,
        },
        None => false,
    }
}

pub fn get_filename_lossy(file: &Option<CopyFile>) -> String {
    match file {
        Some(source) => match source.path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => "???".to_string(),
        },
        None => "-".to_string(),
    }
}

pub fn calculate_percentage_of_completion(status: &&CopyStatus) -> f64 {
    let source = match &status.source {
        Some(source) => {
            match source.length {
                0 => 1, // Avoid divide-by-zero by lying
                length => length,
            }
        }
        None => {
            1 // Avoid divide-by-zero by lying
        }
    };

    let target = match &status.target {
        Some(target) => target.length,
        None => 0,
    };

    let percentage = (target as f64 / source as f64) * 100.0;
    percentage
}

pub struct FDMetadata {
    pub pos: u64,
    // In theory this also contains (at least) flags and mnt_info but they're not
    // useful to me here!
}

static LAZY_OFFSET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"pos:[[:space:]]([[:digit:]]+)")
        .expect("Failed to parse the compiled-in regular expression! Heavens above! :)")
});

impl FDMetadata {
    /// Given a pid and an [[FDInfo]] obtains metadata containing the current FD offset into the file - this is
    /// implemented by direct file reads of /proc because the [[procfs]] crate that I'm using doesn't have it implemented yet.
    fn get_fd_metadata(pid: i32, fdinfo: &FDInfo) -> Result<Self, Box<dyn Error>> {
        let path = PathBuf::from("/proc")
            .join(pid.to_string())
            .join("fdinfo")
            .join(fdinfo.fd.to_string());
        match LAZY_OFFSET_REGEX.captures(&fs::read_to_string(&path)?) {
            Some(foo) => {
                let pos_text = foo.get(1);
                match pos_text {
                    Some(pos_text) => {
                        let pos = pos_text.as_str().parse::<u64>()?;
                        Ok(Self { pos })
                    }
                    None => {
                        eprintln!("No matching group in the opened fdinfo metadata");
                        Ok(Self { pos: 0 })
                    }
                }
            }
            None => {
                eprintln!("No regex match in the opened fdinfo metadata");
                Ok(Self { pos: 0 })
            }
        }
    }
}
