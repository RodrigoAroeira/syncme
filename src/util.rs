use std::path::{Path, PathBuf};

pub fn with_trailing_slash<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut os = path.as_ref().to_path_buf().into_os_string();
    os.push("/");
    PathBuf::from(os)
}
