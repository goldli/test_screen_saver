use core_foundation::string::CFString;
use core_foundation_sys::dictionary::CFDictionaryRef;
use core_foundation_sys::notification_center::{
    CFNotificationCallback, CFNotificationCenterAddObserver,
    CFNotificationCenterGetDistributedCenter, CFNotificationCenterRef, CFNotificationName,
};
use core_foundation_sys::runloop::CFRunLoopRun;
use core_foundation_sys::string::{CFStringGetCStringPtr, CFStringRef, kCFStringEncodingUTF8};
use std::ffi::{c_char, c_void, CStr};
use std::fs;
use core_foundation::base::TCFType;
use std::path::{Path, PathBuf};
use rand::seq::IndexedRandom;
use std::os::unix::fs::symlink;
use std::sync::Mutex;
use std::sync::OnceLock;


static SCREEN_SAVER_DIR: &str = "Library/Application Support/com.apple.wallpaper/aerials/videos";

static SCREEN_SAVER_FILE_NAME: &str = "4207734D-74FE-4F92-B5E1-6EC8DEE24A15.mov";

/*
fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn random_filename_with_ts(ext: &str) -> String {
    let mut rng = rand::rng();
    let rand_num: u32 = rng.random_range(100000..999999);
    format!("{}_{}.{}", timestamp_ms(), rand_num, ext)
}

fn rename_to_dest_file(file: &PathBuf) {
    let mut clone = file.clone();
    clone.set_file_name(SCREEN_SAVER_FILE_NAME);

    fs::rename(file, clone).unwrap();
}

fn rename_random_file(file: &PathBuf) {
    let mut clone = file.clone();
    clone.set_file_name(random_filename_with_ts("mov"));

    fs::rename(file, clone).unwrap();
}


fn get_file_name(path: &PathBuf) -> String {
    path.file_name().unwrap().to_str().unwrap().to_string()
}

*/

// ---------- 全局持久状态 ----------
static LAST_SAVER_FILE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn last_saver_file_lock() -> &'static Mutex<Option<PathBuf>> {
    LAST_SAVER_FILE.get_or_init(|| Mutex::new(None))
}

// ---------- LastSaverFile ----------
struct LastSaverFile { }

impl LastSaverFile {
    fn new() -> LastSaverFile {
        Self { }
    }

    /// 只读，`&self`；打印 last_saver 本身；规范化比较
    pub fn is_same(&self, current: &Path) -> bool {
        let guard = last_saver_file_lock().lock().unwrap();

        match guard.as_ref() {
            Some(last) => {
                // println!("last_saver = {:?}, current = {:?}", last, current);
                let a = last.canonicalize().unwrap_or_else(|_| last.clone());
                let b = current.canonicalize().unwrap_or_else(|_| current.to_path_buf());
                a == b
            }
            None => {
                false
            }
        }
    }

    pub fn set_last(&self, new_last: &Path) {
        let mut guard = last_saver_file_lock().lock().unwrap();
        *guard = Some(new_last.to_path_buf());
    }

    pub fn get_last(&self) -> Option<PathBuf> {
        last_saver_file_lock().lock().unwrap().clone()
    }
}


// ---------- VideoSaver ----------
struct VideoSaver {
    last_saver_file: LastSaverFile
}

impl VideoSaver {
    fn new() -> VideoSaver {
        VideoSaver {
            last_saver_file: LastSaverFile::new()
        }
    }

    fn replace_symlink(&mut self, target: &Path, link: &Path) -> std::io::Result<()> {
        // 1. 删除已存在的符号链接（如果存在）
        //    用 symlink_metadata 判断，避免跟随链接
        match fs::symlink_metadata(link) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    fs::remove_file(link)?;       // 删除链接本身
                    println!("已删除了 符号连接");
                } else {
                    // 不是符号链接，避免误删真实文件/目录
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!("{} 已存在且不是符号链接", link.display()),
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 不存在，直接创建
            }
            Err(e) => return Err(e),
        }

        // 2. 创建新的符号链接
        symlink(target, link)?;
        println!("已重建了 符号连接");

        self.last_saver_file.set_last(target);

        Ok(())
    }

    /// 从指定目录随机选一个文件改名为 4207734D-74FE-4F92-B5E1-6EC8DEE24A15.mov
    fn re_specify_file_in_dir(&mut self, dir: &str) -> std::io::Result<()> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));

        println!("------------");
        let mut files = Vec::new();
        let mov_dir = home.join(dir).join("mov");

        if !mov_dir.exists() {
            eprintln!("mov目录不存在: {}", mov_dir.display());
            return Ok(());
        }

        for entry in std::fs::read_dir(mov_dir)? {
            let path = entry?.path();
            // 只取文件，跳过文件夹、软链接（is_file 会跟随软链接）
            if path.is_file() {
                let filename = path.file_name().unwrap().to_str().unwrap();
                if !filename.starts_with(".") {
                    // println!("搜索到文件: {}", filename);
                    if !self.last_saver_file.is_same(&path) {
                        files.push(path.clone());
                    }
                }
            }
        }

        // println!("文件数量: {}", files.len());
        if files.is_empty() {
            eprintln!("无可用候选视频");
            return Ok(());
        }

        // 0. 视频屏保文件(symlink的目标)
        let saver_file = &home.join(dir).join(SCREEN_SAVER_FILE_NAME);

        // 1. 随机取一个文件
        let mut random = rand::rng();
        //let chosen = files.choose(&mut random).unwrap();
        // 3. 空集合兜底 + 不用 unwrap
        let chosen = if let Some(c) = files.choose(&mut random) {
            c.clone()
        } else {
            // 过滤后为空，退回全集（或直接返回 Ok，不切换）
            return Ok(());
        };

        if let Err(e) = self.replace_symlink(chosen.as_path(), &saver_file.as_path()) {
            eprintln!("替换软链接失败: {e}");
        }
        Ok(())
    }
}



unsafe extern "C" fn notify_callback(
    _center: CFNotificationCenterRef,
    _observer: *mut c_void,
    name: CFNotificationName,
    _object: *mut c_void,
    _user_info: CFDictionaryRef,
) {
    unsafe {
        let cf_str: CFStringRef = name;
        let ptr = CFStringGetCStringPtr(cf_str, kCFStringEncodingUTF8);
        if ptr.is_null() {
            return;
        }
        let cstr = CStr::from_ptr(ptr as *const c_char);
        let Ok(s) = cstr.to_str() else {
            return;
        };

        match s {
            "com.apple.screensaver.didstop" => {
                println!("✅ 检测到屏保退出！");

                let mut saver = VideoSaver::new();
                let _ = saver.re_specify_file_in_dir(SCREEN_SAVER_DIR);
                println!("文件名调整完成")
            },
            "com.apple.screensaver.didstart" => println!("ℹ️ 屏保启动"),
            _ => {}
        }
    }
}


fn main() {
    unsafe {
        let center = CFNotificationCenterGetDistributedCenter();

        let name_stop = CFString::from_static_string("com.apple.screensaver.didstop"); //能检测到视频屏保退出

        CFNotificationCenterAddObserver(
            center,
            std::ptr::null_mut(),
            // 强制转成 CFNotificationCallback（因为类型系统没法自动对齐）
            std::mem::transmute::<unsafe extern "C" fn(_, _, _, _, _) -> (), CFNotificationCallback>(notify_callback),
            name_stop.as_concrete_TypeRef(),
            std::ptr::null_mut(),
            0,
        );

        let name_start = CFString::from_static_string("com.apple.screensaver.didstart");
        CFNotificationCenterAddObserver(
            center,
            std::ptr::null_mut(),
            std::mem::transmute::<unsafe extern "C" fn(_, _, _, _, _) -> (), CFNotificationCallback>(notify_callback),
            name_start.as_concrete_TypeRef(),
            std::ptr::null_mut(),
            0,
        );

        println!("🔍 监听 macOS 屏保事件中...");

        CFRunLoopRun();
    }
}
