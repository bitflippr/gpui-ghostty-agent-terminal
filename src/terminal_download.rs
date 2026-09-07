//! Owned downloads emitted by the VT engine, without interpreting PTY bytes.
use std::ffi::c_void;

#[derive(Debug)]
pub struct Download {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadStatus {
    pub name: String,
    pub result: Option<Result<std::path::PathBuf, String>>,
}

/// One bounded disk worker shared by all sessions. Saving never stalls PTY reads.
#[cfg(feature = "gui")]
#[derive(Default)]
pub(crate) struct Downloads {
    statuses: Vec<DownloadStatus>,
    pending: Vec<(
        usize,
        std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>,
    )>,
}

#[cfg(feature = "gui")]
impl Downloads {
    pub fn submit(&mut self, download: Download) {
        type Job = (
            Download,
            std::sync::mpsc::SyncSender<Result<std::path::PathBuf, String>>,
        );
        static WORKER: std::sync::OnceLock<Result<std::sync::mpsc::SyncSender<Job>, String>> =
            std::sync::OnceLock::new();
        let worker = WORKER.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::sync_channel::<Job>(1);
            std::thread::Builder::new()
                .name("terminal-downloads".into())
                .spawn(move || {
                    while let Ok((download, completed)) = rx.recv() {
                        let result = dirs::download_dir()
                            .ok_or_else(|| "Downloads folder is unavailable".to_string())
                            .and_then(|directory| {
                                download.save_in(&directory).map_err(|e| e.to_string())
                            });
                        let _ = completed.send(result);
                    }
                })
                .map_err(|e| e.to_string())?;
            Ok(tx)
        });
        // Keep completed history bounded, without changing pending indices.
        if self.statuses.len() >= 32 {
            self.statuses.remove(0);
            self.pending.retain_mut(|(index, _)| {
                if *index == 0 {
                    false
                } else {
                    *index -= 1;
                    true
                }
            });
        }
        let index = self.statuses.len();
        self.statuses.push(DownloadStatus {
            name: safe_name(&download.name),
            result: None,
        });
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let error = match worker {
            Ok(worker) => worker
                .try_send((download, tx))
                .err()
                .map(|_| "Download queue is full".to_string()),
            Err(error) => Some(error.clone()),
        };
        if let Some(error) = error {
            self.statuses[index].result = Some(Err(error));
        } else {
            self.pending.push((index, rx));
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        self.pending.retain(|(index, rx)| match rx.try_recv() {
            Ok(result) => {
                self.statuses[*index].result = Some(result);
                changed = true;
                false
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.statuses[*index].result = Some(Err("Download worker stopped".into()));
                changed = true;
                false
            }
        });
        changed
    }

    pub fn snapshot(&self) -> Vec<DownloadStatus> {
        self.statuses.clone()
    }
}

fn safe_name(name: &str) -> String {
    let name = name.rsplit(['/', '\\']).next().unwrap_or("");
    let mut clean: String = name
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if "<>:\"|?*".contains(c) { '_' } else { c })
        .take(160)
        .collect();
    while clean.len() > 200 {
        clean.pop();
    }
    clean = clean.trim_matches([' ', '.']).to_owned();
    let stem = clean.split('.').next().unwrap_or("").to_ascii_uppercase();
    if clean.is_empty() {
        return "download".into();
    }
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    {
        clean.insert(0, '_');
    }
    clean
}

impl Download {
    pub fn save_in(&self, directory: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        use std::io::Write;
        std::fs::create_dir_all(directory)?;
        let name = safe_name(&self.name);
        let path = std::path::Path::new(&name);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path
            .extension()
            .map(|s| format!(".{}", s.to_string_lossy()))
            .unwrap_or_default();
        for suffix in 0..10_000 {
            let candidate = directory.join(if suffix == 0 {
                name.clone()
            } else {
                format!("{stem} ({suffix}){ext}")
            });
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&candidate) {
                Ok(mut file) => {
                    if let Err(error) = file.write_all(&self.data).and_then(|()| file.sync_all()) {
                        drop(file);
                        let _ = std::fs::remove_file(&candidate);
                        return Err(error);
                    }
                    return Ok(candidate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Too many downloads with this name",
        ))
    }
}

#[derive(Default)]
pub(crate) struct DownloadQueue {
    pending: Vec<Download>,
    bytes: usize,
}

unsafe extern "C" {
    fn spike_terminal_download_handler(
        terminal: *mut c_void,
        context: *mut c_void,
        callback: unsafe extern "C" fn(*mut c_void, *const u8, usize, *const u8, usize),
    ) -> i32;
}

impl DownloadQueue {
    pub(crate) fn install(&mut self, terminal: *mut c_void) -> Result<(), String> {
        let result = unsafe {
            spike_terminal_download_handler(terminal, std::ptr::from_mut(self).cast(), receive)
        };
        if result == 0 {
            Ok(())
        } else {
            Err(format!("install download handler: {result}"))
        }
    }

    pub(crate) fn take(&mut self) -> Vec<Download> {
        self.bytes = 0;
        std::mem::take(&mut self.pending)
    }
}

unsafe extern "C" fn receive(
    context: *mut c_void,
    name: *const u8,
    name_len: usize,
    data: *const u8,
    data_len: usize,
) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let queue = unsafe { &mut *context.cast::<DownloadQueue>() };
        if name_len > 4096
            || data_len > (64 * 1024 * 1024usize).saturating_sub(queue.bytes)
            || queue.pending.len() >= 128
        {
            return;
        }
        let name = unsafe { std::slice::from_raw_parts(name, name_len) };
        let data = unsafe { std::slice::from_raw_parts(data, data_len) };
        queue.pending.push(Download {
            name: String::from_utf8_lossy(name).into_owned(),
            data: data.to_vec(),
        });
        queue.bytes += data_len;
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn iterm_downloads_preserve_binary_data_and_do_not_render() {
        let mut terminal = crate::ghostty::Terminal::new(80, 24).unwrap();
        let body = [0, 255, 27, 7, 128, 13, 10];
        let data = base64::engine::general_purpose::STANDARD.encode(body);
        let name = base64::engine::general_purpose::STANDARD.encode("../binary.dat");
        for sequence in [
            format!("\x1b]1337;File=name={name};size=7:{data}\x07"),
            format!(
                "\x1b]1337;MultipartFile=name={name};size=7\x1b\\\x1b]1337;FilePart={}\x07\x1b]1337;FilePart={}\x1b\\\x1b]1337;FileEnd\x07",
                &data[..4],
                &data[4..]
            ),
        ] {
            for byte in sequence.bytes() {
                terminal.feed(&[byte]).unwrap();
            }
            let downloads = terminal.take_downloads();
            assert_eq!(downloads.len(), 1);
            assert_eq!(downloads[0].name, "../binary.dat");
            assert_eq!(downloads[0].data, body);
            assert!(terminal.render_update(true).unwrap().images.is_empty());
        }
    }

    #[test]
    fn saves_use_only_a_basename_and_never_overwrite() {
        let directory = std::env::temp_dir().join(format!(
            "terminal-download-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let download = Download {
            name: "../../sub\\report.txt".into(),
            data: b"first".to_vec(),
        };
        let first = download.save_in(&directory).unwrap();
        let second = Download {
            data: b"second".to_vec(),
            ..download
        }
        .save_in(&directory)
        .unwrap();
        assert_eq!(first, directory.join("report.txt"));
        assert_eq!(second, directory.join("report (1).txt"));
        assert_eq!(std::fs::read(&first).unwrap(), b"first");
        assert_eq!(std::fs::read(&second).unwrap(), b"second");
        assert_eq!(safe_name("C:\\a\\NUL.txt"), "_NUL.txt");
        assert_eq!(safe_name("../.."), "download");
        std::fs::remove_file(first).unwrap();
        std::fs::remove_file(second).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
