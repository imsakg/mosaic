use std::env;
use std::time::SystemTime;
extern crate rustc_version_runtime;
use chrono::DateTime;
use rustc_version_runtime::version;

pub fn prepare_build_info() -> String {
    // dd/MM/yyyy HH:mm:ss
    let now = SystemTime::now();
    let date = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let date = DateTime::from_timestamp(date as i64, 0).unwrap();
    let date = date.format("%d/%m/%Y %H:%M:%S").to_string();
    let rustc = version();
    let rustc = format!("{}.{}.{}", rustc.major, rustc.minor, rustc.patch);
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    format!("Built on {} with rustc {} for {}-{}", date, rustc, os, arch)
}
