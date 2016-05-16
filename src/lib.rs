extern crate libc;

use std::fs::OpenOptions;
use std::fs::File;

use std::os::unix::prelude::*;
use std::io;
use std::path::PathBuf;
use libc::{c_int, ioctl, uint8_t, uint32_t, uint64_t};

static LOOP_SET_FD: u64 = 0x4C00;
static LOOP_SET_STATUS64: u64 = 0x4C04;
static LOOP_CTL_GET_FREE: u64 = 0x4C82;

const LOOP_PREFIX: &'static str = "/dev/loop";

#[derive(Debug)]
pub struct LoopControl {
    dev_file: File,
}

impl LoopControl {
    pub fn open(dev_file: &str) -> io::Result<LoopControl> {
        Ok(LoopControl { dev_file: try!(OpenOptions::new().read(true).write(true).open(dev_file)) })
    }

    // Finds and returns the next availble loop device
    pub fn next_free(&self) -> io::Result<LoopDevice> {
        let result: i32;
        unsafe {
            result = ioctl(self.dev_file.as_raw_fd() as c_int, LOOP_CTL_GET_FREE);
        }
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            let path = LOOP_PREFIX.to_string() + &result.to_string();
            Ok(LoopDevice { device: try!(OpenOptions::new().read(true).write(true).open(path)) })
        }
    }
}

#[derive(Debug)]
pub struct LoopDevice {
    device: File,
}

#[repr(C)]
pub struct loop_info64 {
    pub lo_device: uint64_t,
    pub lo_inode: uint64_t,
    pub lo_rdevice: uint64_t,
    pub lo_offset: uint64_t,
    pub lo_sizelimit: uint64_t,
    pub lo_number: uint32_t,
    pub lo_encrypt_type: uint32_t,
    pub lo_encrypt_key_size: uint32_t,
    pub lo_flags: uint32_t,
    pub lo_file_name: [uint8_t; 64],
    pub lo_crypt_name: [uint8_t; 64],
    pub lo_encrypt_key: [uint8_t; 32],
    pub lo_init: [uint64_t; 2],
}

use std::default::Default;

impl Default for loop_info64 {
    fn default() -> loop_info64 {
        loop_info64 {
            lo_device: 0,
            lo_inode: 0,
            lo_rdevice: 0,
            lo_offset: 0,
            lo_sizelimit: 0,
            lo_number: 0,
            lo_encrypt_type: 0,
            lo_encrypt_key_size: 0,
            lo_flags: 0,
            lo_file_name: [0; 64],
            lo_crypt_name: [0; 64],
            lo_encrypt_key: [0; 32],
            lo_init: [0; 2],
        }
    }
}

impl LoopDevice {
    // Attach a loop device to a file.
    pub fn attach(&mut self, backing_file: &str, offset: u64) -> io::Result<()> {
        let bf = try!(OpenOptions::new().read(true).write(true).open(backing_file));

        // Attach backing_file to device
        unsafe {
            if ioctl(self.device.as_raw_fd() as c_int,
                     LOOP_SET_FD,
                     bf.as_raw_fd() as c_int) < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        // Set offset for backing_file
        let mut info: loop_info64 = Default::default();
        info.lo_offset = offset;
        unsafe {
            if ioctl(self.device.as_raw_fd() as c_int,
                     LOOP_SET_STATUS64,
                     &mut info) < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        let mut p = PathBuf::from("/proc/self/fd");
        p.push(self.device.as_raw_fd().to_string());
        std::fs::read_link(&p).ok()
    }

    // Detach a loop device from its backing file.
    pub fn detach(&self) -> io::Result<()> {
        Ok(())
    }
}
