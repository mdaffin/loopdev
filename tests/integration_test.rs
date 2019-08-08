extern crate libc;
extern crate loopdev;
extern crate tempfile;
#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate serde_json;

use libc::fallocate;
use serde::{Deserialize, Deserializer};
use std::fmt::Display;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use tempfile::{NamedTempFile, TempPath};

use loopdev::LoopControl;

// All tests use the same loopback device interface and so can tread on each others toes leading to
// racy tests. So we need to lock all tests to ensure only one runs at a time.
lazy_static! {
    static ref LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

#[test]
fn get_next_free_device() {
    let _lock = setup();

    let lc = LoopControl::open().expect("should be able to open the LoopControl device");
    let ld0 = lc
        .next_free()
        .expect("should not error finding the next free loopback device");

    assert_eq!(
        ld0.path(),
        Some(PathBuf::from("/dev/loop0")),
        "should find the first loopback device"
    );
}

#[test]
fn attach_a_backing_file_default() {
    attach_a_backing_file(0, 0, 128 * 1024 * 1024);
}

#[test]
fn attach_a_backing_file_with_offset() {
    attach_a_backing_file(128 * 1024, 0, 128 * 1024 * 1024);
}

#[test]
fn attach_a_backing_file_with_sizelimit() {
    attach_a_backing_file(0, 128 * 1024, 128 * 1024 * 1024);
}

#[test]
fn attach_a_backing_file_with_offset_sizelimit() {
    attach_a_backing_file(128 * 1024, 128 * 1024, 128 * 1024 * 1024);
}

// This is also allowed by losetup, not sure what happens if you try to write to the file though.
#[test]
fn attach_a_backing_file_with_offset_overflow() {
    attach_a_backing_file(128 * 1024 * 1024 * 2, 0, 128 * 1024 * 1024);
}

// This is also allowed by losetup, not sure what happens if you try to write to the file though.
#[test]
fn attach_a_backing_file_with_sizelimit_overflow() {
    attach_a_backing_file(0, 128 * 1024 * 1024 * 2, 128 * 1024 * 1024);
}

fn attach_a_backing_file(offset: u64, sizelimit: u64, file_size: i64) {
    let _lock = setup();

    let (devices, ld0_path, file_path) = {
        let lc = LoopControl::open().expect("should be able to open the LoopControl device");

        let file = create_backing_file(file_size);
        let file_path = file.to_path_buf();
        let ld0 = lc
            .next_free()
            .expect("should not error finding the next free loopback device");

        ld0.attach_with_sizelimit(&file, offset, sizelimit)
            .expect("should not error attaching the backing file to the loopdev");

        let devices = list_device(Some(ld0.path().unwrap().to_str().unwrap()));
        file.close().expect("should delete the temp backing file");

        (devices, ld0.path().unwrap(), file_path)
    };

    assert_eq!(
        devices.len(),
        1,
        "there should be only one loopback mounted device"
    );
    assert_eq!(
        devices[0].name.as_str(),
        ld0_path.to_str().unwrap(),
        "the attached devices name should match the input name"
    );
    assert_eq!(
        devices[0].back_file.as_str(),
        file_path.to_str().unwrap(),
        "the backing file should match the given file"
    );
    assert_eq!(
        devices[0].offset, offset,
        "the offset should match the requested offset"
    );
    assert_eq!(
        devices[0].size_limit, sizelimit,
        "the sizelimit should match the requested sizelimit"
    );

    detach_all();
}

fn create_backing_file(size: i64) -> TempPath {
    let file = NamedTempFile::new().expect("should be able to create a temp file");
    if unsafe { fallocate(file.as_raw_fd(), 0, 0, size) } < 0 {
        panic!(
            "should be able to allocate the tenp file: {}",
            io::Error::last_os_error()
        );
    }
    file.into_temp_path()
}

fn setup() -> MutexGuard<'static, ()> {
    let lock = LOCK.lock().unwrap();
    detach_all();
    lock
}

fn detach_all() {
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

fn list_device(dev_file: Option<&str>) -> Vec<LoopDeviceOutput> {
    let mut output = Command::new("losetup");
    output.args(&["-J", "-l"]);
    if let Some(dev_file) = dev_file {
        output.arg(dev_file);
    }
    let output = output
        .output()
        .expect("failed to cleanup existing loop devices");
    serde_json::from_slice::<ListOutput>(&output.stdout)
        .unwrap()
        .loopdevices
}

#[derive(Deserialize, Debug)]
struct LoopDeviceOutput {
    pub name: String,
    #[serde(rename = "sizelimit")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub size_limit: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub offset: u64,
    #[serde(rename = "back-file")]
    pub back_file: String,
}

#[derive(Deserialize, Debug)]
struct ListOutput {
    pub loopdevices: Vec<LoopDeviceOutput>,
}

pub fn deserialize_number_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    String::deserialize(deserializer)?
        .parse::<T>()
        .map_err(serde::de::Error::custom)
}
