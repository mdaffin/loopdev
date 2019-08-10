use libc::fallocate;
use serde::{Deserialize, Deserializer};
use std::io;
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};

use tempfile::{NamedTempFile, TempPath};

// All tests use the same loopback device interface and so can tread on each others toes leading to
// racy tests. So we need to lock all tests to ensure only one runs at a time.
lazy_static! {
    static ref LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

pub fn create_backing_file(size: i64) -> TempPath {
    let file = NamedTempFile::new().expect("should be able to create a temp file");
    if unsafe { fallocate(file.as_raw_fd(), 0, 0, size) } < 0 {
        panic!(
            "should be able to allocate the tenp file: {}",
            io::Error::last_os_error()
        );
    }
    file.into_temp_path()
}

pub fn setup() -> MutexGuard<'static, ()> {
    let lock = LOCK.lock().unwrap();
    detach_all();
    lock
}

pub fn attach_file(loop_dev: &str, backing_file: &str, offset: u64, sizelimit: u64) {
    if !Command::new("losetup")
        .args(&[
            loop_dev,
            backing_file,
            "--offset",
            &offset.to_string(),
            "--sizelimit",
            &sizelimit.to_string(),
        ])
        .status()
        .expect("failed to attach backing file to loop device")
        .success()
    {
        panic!("failed to cleanup existing loop devices")
    }
}

pub fn detach_all() {
    std::thread::sleep(std::time::Duration::from_millis(10));
    if !Command::new("losetup")
        .args(&["-D"])
        .status()
        .expect("failed to cleanup existing loop devices")
        .success()
    {
        panic!("failed to cleanup existing loop devices")
    }
    std::thread::sleep(std::time::Duration::from_millis(10));
}

pub fn list_device(dev_file: Option<&str>) -> Vec<LoopDeviceOutput> {
    let mut output = Command::new("losetup");
    output.args(&["-J", "-l"]);
    if let Some(dev_file) = dev_file {
        output.arg(dev_file);
    }
    let output = output
        .output()
        .expect("failed to cleanup existing loop devices");

    if output.stdout.len() == 0 {
        Vec::new()
    } else {
        serde_json::from_slice::<ListOutput>(&output.stdout)
            .unwrap()
            .loopdevices
    }
}

#[derive(Deserialize, Debug)]
pub struct LoopDeviceOutput {
    pub name: String,
    #[serde(rename = "sizelimit")]
    #[serde(deserialize_with = "deserialize_optional_number_from_string")]
    pub size_limit: Option<u64>,
    #[serde(deserialize_with = "deserialize_optional_number_from_string")]
    pub offset: Option<u64>,
    #[serde(rename = "back-file")]
    //#[serde(deserialize_with = "deserialize_nullable_string")]
    pub back_file: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ListOutput {
    pub loopdevices: Vec<LoopDeviceOutput>,
}

pub fn deserialize_optional_number_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        String(Option<String>),
        Number(Option<u64>),
    }

    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::String(None) | StringOrInt::Number(None) => Ok(None),
        StringOrInt::String(Some(s)) => Ok(Some(s.parse().map_err(serde::de::Error::custom)?)),
        StringOrInt::Number(Some(i)) => Ok(Some(i)),
    }
}
