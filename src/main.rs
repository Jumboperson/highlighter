use std::ffi::CString;
use std::fs;
use std::io;
use std::path::PathBuf;
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::fileapi::{FindFirstChangeNotificationA, FindNextChangeNotification};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::{INFINITE, WAIT_FAILED, WAIT_OBJECT_0};
use winapi::um::winnt::{FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, HANDLE};

struct DirWatcher {
    src: PathBuf,
    dst: PathBuf,
    watcher: HANDLE,
}

impl DirWatcher {
    pub fn new(src: PathBuf, dst: PathBuf) -> Option<Self> {
        let cstring = CString::new(&*src.clone().into_os_string().into_string().unwrap())
            .expect("Failed to make CString");
        let handle = unsafe {
            FindFirstChangeNotificationA(
                cstring.as_c_str().as_ptr(),
                0,
                FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE,
            )
        };

        if !src.is_dir() {
            panic!("Source needs to be a directory");
        }
        if !dst.is_dir() {
            panic!("Destination needs to be a directory");
        }

        if handle != INVALID_HANDLE_VALUE {
            println!("Creating DirWatcher from {:?} to {:?}", src, dst);
            Some(Self {
                src,
                dst,
                watcher: handle,
            })
        } else {
            None
        }
    }

    // Callback when we have a notification
    fn run_copy(&self) -> io::Result<()> {
        for entry in fs::read_dir(self.src.clone())? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let fname = path.file_name().unwrap();
            let mut fpath = self.dst.clone();
            fpath.push(fname);
            if fpath.exists() && path.exists() {
                let dst_meta = fpath.metadata()?;
                let src_meta = path.metadata()?;
                let dst_time = dst_meta.modified().unwrap();
                let src_time = src_meta.modified().unwrap();
                // If the file has actually been modified since, update it
                if !src_time.duration_since(dst_time).unwrap().is_zero() {
                    fs::copy(path, fpath)?;
                }
            } else {
                fs::copy(path, fpath)?;
            }
        }
        Ok(())
    }

    pub fn execute(&self) {
        loop {
            let status = unsafe { WaitForSingleObject(self.watcher, INFINITE) };
            match status {
                WAIT_FAILED => panic!("Something went wrong!"),
                WAIT_TIMEOUT => continue,
                WAIT_OBJECT_0 => {
                    if unsafe { FindNextChangeNotification(self.watcher) } != 0 {
                        self.run_copy();
                    }
                }
                _ => panic!("What?! {}", status),
            }
        }
    }
}

fn main() {
    let appdata = std::env::var("TEMP").unwrap();
    let homepath = std::env::var("USERPROFILE").unwrap();
    let watcher = DirWatcher::new(
        PathBuf::from(appdata + "\\Highlights\\Hunt  Showdown"),
        PathBuf::from(homepath + "\\Videos\\Hunt  Showdown"),
    )
    .unwrap();
    watcher.execute();
}
