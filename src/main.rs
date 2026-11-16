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



static SCREEN_SAVER_DIR: &str = "/Users/ai2soft/Library/Application Support/com.apple.wallpaper/aerials/videos";

static SCREEN_SAVER_FILE_NAME: &str = "4207734D-74FE-4F92-B5E1-6EC8DEE24A15.mov";

static DS_STORE: &str = ".DS_Store";


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
*/

fn get_file_name(path: &PathBuf) -> String {
    path.file_name().unwrap().to_str().unwrap().to_string()
}

fn replace_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
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
    Ok(())
}

/// 从指定目录随机选一个文件改名为 4207734D-74FE-4F92-B5E1-6EC8DEE24A15.mov
fn re_specify_file_in_dir(dir: &str) -> std::io::Result<()> {
    let mut files = Vec::new();
    let mov_dir = Path::new(SCREEN_SAVER_DIR).join("mov");

    for entry in std::fs::read_dir(mov_dir)? {
        let path = entry?.path();
        // 只取文件，跳过文件夹、软链接（is_file 会跟随软链接）
        if path.is_file() {
            let filename = path.file_name().unwrap().to_str().unwrap();
            if filename != DS_STORE {
                //println!("搜索到文件: {}", filename);
                files.push(path);
            }
        }
    }

    // 0. 视频屏保文件(symlink的目标)
    let saver_file = &Path::new(SCREEN_SAVER_DIR).join(SCREEN_SAVER_FILE_NAME);

    // 1. 随机取一个文件
    let mut random = rand::rng();
    let chosen = files.choose(&mut random).unwrap();

    let _ = replace_symlink(chosen.as_path(), saver_file.as_path());

    Ok(())
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
                let _ = re_specify_file_in_dir(SCREEN_SAVER_DIR);
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
