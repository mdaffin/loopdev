use gpt::partition::Partition;
use std::collections::BTreeMap;
use std::fs;
use std::{io, os::unix::prelude::AsRawFd, path, process, thread, time};

use loopdev::{LoopControl, LoopDevice};
use serde::Deserialize;
use tempfile::NamedTempFile;

/// Attached loop device that is detached on drop.
pub struct AttachedLoopDevice {
    /// Backing file
    pub file: NamedTempFile,
    /// Loop device
    pub loop_device: LoopDevice,
}

impl Drop for AttachedLoopDevice {
    fn drop(&mut self) {
        self.loop_device.detach().expect("failed to detach");
    }
}

/// Retry helper with backoff delay.
pub struct Retries {
    retries: u32,
    delay: time::Duration,
}

impl Default for Retries {
    fn default() -> Self {
        Self {
            retries: 20,
            delay: time::Duration::from_millis(1),
        }
    }
}

impl Retries {
    /// Construct a new `Retries` entity with `retries` number of retries
    /// and a intial backoff duration of `initial_backoff`.
    pub fn new(retries: u32, initial_backoff: time::Duration) -> Retries {
        Retries {
            retries,
            delay: initial_backoff,
        }
    }
}

impl Iterator for Retries {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.retries == 0 {
            None
        } else {
            self.retries -= 1;
            thread::sleep(self.delay);
            self.delay = self.delay * 2;
            Some(self.retries)
        }
    }
}

/// Loop device meta info from `losetup`.
#[derive(Deserialize, Debug)]
pub struct LoopDeviceOutput {
    /// Device path.
    pub name: path::PathBuf,
    /// Size limit
    #[serde(rename = "sizelimit")]
    pub size_limit: Option<u64>,
    /// Offset.
    pub offset: Option<u64>,
    /// Backing file path.
    #[serde(rename = "back-file")]
    pub backing_file: Option<path::PathBuf>,
}

/// Query loopback device states with `losetup` and try to find
/// a device that matches `path`. Returns `None` if the device is not found.
///
/// # Panic
/// Panics if the `losetup` invocation fails or the output is not parseable
pub fn losetup_find_device(path: impl AsRef<path::Path>) -> Option<LoopDeviceOutput> {
    #[derive(Deserialize, Debug)]
    struct LoopDeviceList {
        loopdevices: Vec<LoopDeviceOutput>,
    }

    let losetup_stdout = process::Command::new("losetup")
        .args(["-J", "-l"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run losetup")
        .wait_with_output()
        .expect("failed to get losetup output")
        .stdout;
    let list = serde_json::from_reader::<_, LoopDeviceList>(io::Cursor::new(losetup_stdout))
        .expect("failed to parse losetup output");
    list.loopdevices
        .into_iter()
        .find(|d| &d.name == path.as_ref())
}

/// Create a temporary backing file with size `file_size`.
pub fn create_backing_file(file_size: u64) -> NamedTempFile {
    let file = NamedTempFile::new().expect("should be able to create a temp file");
    assert!(
        !(unsafe { libc::fallocate(file.as_raw_fd(), 0, 0, file_size as i64) } < 0),
        "should be able to allocate the temp file: {}",
        io::Error::last_os_error()
    );
    file
}

/// Write a GPT table to `file` with one partion of size `size`.
pub fn partition_backing_file(file: impl AsRef<path::Path>, size: u64) {
    let mut device = fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .expect("file should be writeable");
    gpt::mbr::ProtectiveMBR::new()
        .overwrite_lba0(&mut device)
        .expect("failed to write MBR");

    let mut disk = gpt::GptConfig::new()
        .initialized(false)
        .writable(true)
        .logical_block_size(gpt::disk::LogicalBlockSize::Lb512)
        .open(file)
        .expect("could not open backing file");

    disk.update_partitions(BTreeMap::<u32, Partition>::new())
        .expect("coult not initialize blank partition table");

    disk.add_partition(
        "Linux filesystem",
        size,
        gpt::partition_types::LINUX_FS,
        0,
        None,
    )
    .expect("could not create partition");

    disk.write()
        .expect("could not write partition table to backing file");
}

/// Create a backing file with `file_size` and attach.
pub fn create_attach_backing_file(
    offset: u64,
    sizelimit: u64,
    file_size: u64,
    part_scan: bool,
) -> AttachedLoopDevice {
    let backing_file = create_backing_file(file_size);
    attach_backing_file(backing_file, offset, sizelimit, part_scan)
}

/// Attach backing file `backing_file
pub fn attach_backing_file(
    backing_file: NamedTempFile,
    offset: u64,
    sizelimit: u64,
    part_scan: bool,
) -> AttachedLoopDevice {
    let loop_device = next_attach_retried(backing_file.as_ref(), offset, sizelimit, part_scan)
        .expect("failed to attach loop device");
    let loop_device_state =
        losetup_find_device(&loop_device.path().unwrap()).expect("failed to get device state");

    assert_eq!(
        loop_device_state
            .backing_file
            .expect("missing backing file"),
        backing_file.as_ref(),
        "the backing file should match the given file"
    );
    assert_eq!(
        loop_device_state.offset,
        Some(offset),
        "the offset should match the requested offset"
    );
    assert_eq!(
        loop_device_state.size_limit,
        Some(sizelimit),
        "the sizelimit should match the requested sizelimit"
    );

    AttachedLoopDevice {
        file: backing_file,
        loop_device,
    }
}

fn next_attach_retried(
    file: &path::Path,
    offset: u64,
    sizelimit: u64,
    part_scan: bool,
) -> Result<LoopDevice, &'static str> {
    let loop_control = LoopControl::open().expect("should be able to open the LoopControl device");

    for _ in Retries::new(10, time::Duration::from_millis(1)) {
        match loop_control.next_free().and_then(|loop_device| {
            loop_device
                .with()
                .offset(offset)
                .size_limit(sizelimit)
                .part_scan(part_scan)
                .attach(&file)
                .map(|_| loop_device)
        }) {
            Ok(loop_device) => {
                // Wait for the device file to pop up.
                for _ in Retries::default() {
                    if loop_device.path().expect("failed to get path").exists() {
                        return Ok(loop_device);
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(e) if e.raw_os_error() == Some(libc::EBUSY) => (),
            Err(e) => panic!("failed to attach file: {:?}", e),
        };
    }

    Err("failed to attach. Out of retries")
}

#[test]
fn retries() {
    assert_eq!(Retries::new(10, time::Duration::from_nanos(1)).count(), 10);
}
