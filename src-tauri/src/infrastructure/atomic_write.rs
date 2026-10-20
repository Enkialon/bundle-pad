use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// 在目标文件所在目录写入临时文件，完成后再替换原文件。
/// 替换失败时保留原文件，并删除临时文件。
pub fn atomic_write(target: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = target.parent().filter(|path| !path.as_os_str().is_empty()).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "目标路径没有父目录")
    })?;
    fs::create_dir_all(parent)?;

    let mut temp = tempfile::Builder::new().prefix(".bundlepad-").suffix(".tmp").tempfile_in(parent)?;
    temp.write_all(bytes)?;
    temp.flush()?;
    temp.as_file().sync_all()?;

    let temp_path = temp.into_temp_path();
    replace_file(temp_path.as_ref(), target)?;
    let _ = temp_path.keep();
    Ok(())
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }

    let from_wide = wide(from);
    let to_wide = wide(to);
    let ok = unsafe { MoveFileExW(from_wide.as_ptr(), to_wide.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn replaces_existing_file_without_deleting_first() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        fs::write(&target, "原始内容").unwrap();

        atomic_write(&target, "新的内容".as_bytes()).unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "新的内容");
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".bundlepad-"))
            .collect();
        assert!(leftovers.is_empty(), "临时文件应在替换后消失: {leftovers:?}");
    }

    #[test]
    fn failed_write_keeps_the_original_file() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-directory");
        fs::write(&blocker, "occupied").unwrap();
        let target = blocker.join("note.md");

        let error = atomic_write(&target, b"new").unwrap_err();
        assert_eq!(fs::read_to_string(&blocker).unwrap(), "occupied");
        assert!(!target.exists());
        let _ = error;
    }
}
