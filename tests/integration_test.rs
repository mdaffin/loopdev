use std::path::Path;

use loopdev::LoopControl;

use crate::util::{
    create_attach_backing_file, create_backing_file, partition_backing_file, Retries,
};

mod util;

#[test]
fn get_next_free_device() {
    LoopControl::open()
        .expect("should be able to open the LoopControl device")
        .next_free()
        .expect("should not error finding the next free loopback device");
}

#[test]
fn attach_a_backing_file_default() {
    create_attach_backing_file(0, 0, 128 * 1024 * 1024, false);
}

#[test]
fn attach_a_backing_file_with_offset() {
    create_attach_backing_file(128 * 1024, 0, 128 * 1024 * 1024, false);
}

#[test]
fn attach_a_backing_file_with_sizelimit() {
    create_attach_backing_file(0, 128 * 1024, 128 * 1024 * 1024, false);
}

#[test]
fn attach_a_backing_file_with_offset_sizelimit() {
    create_attach_backing_file(128 * 1024, 128 * 1024, 128 * 1024 * 1024, false);
}

// This is also allowed by losetup, not sure what happens if you try to write to the file though.
#[test]
fn attach_a_backing_file_with_offset_overflow() {
    create_attach_backing_file(128 * 1024 * 1024 * 2, 0, 128 * 1024 * 1024, false);
}

// This is also allowed by losetup, not sure what happens if you try to write to the file though.
#[test]
fn attach_a_backing_file_with_sizelimit_overflow() {
    create_attach_backing_file(0, 128 * 1024 * 1024 * 2, 128 * 1024 * 1024, false);
}

#[test]
fn attach_a_backing_file_with_part_scan_default() {
    const SIZE: u64 = 10 * 1024 * 1024;
    let backing_file = create_backing_file(SIZE);
    partition_backing_file(backing_file.path(), 1024 * 1024);
    let attached = util::attach_backing_file(backing_file, 0, SIZE, true);

    // Assume that partion zero is <device>p0.
    let loop_device_path_partition_1 = attached
        .loop_device
        .path()
        .expect("failed to get path")
        .to_string_lossy()
        .chars()
        .chain("p1".chars())
        .collect::<String>();
    let loop_device_path_partition_0 = Path::new(&loop_device_path_partition_1);

    for _ in Retries::default() {
        if loop_device_path_partition_0.exists() {
            return;
        }
    }

    panic!(
        "failed to find partition {:?}",
        loop_device_path_partition_0
    );
}
