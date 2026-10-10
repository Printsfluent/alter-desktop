// Server stand-in for the desktop storage module: the shared ffmpeg resolver
// only needs a data root to look for bundled binaries.
use std::path::PathBuf;

pub fn data_root() -> PathBuf {
    std::env::temp_dir().join("alter-bot")
}
