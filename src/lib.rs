//! Setup and control loop devices.
//!
//! Provides rust interface with similar functionalty to the linux utility `losetup`.
//!
//! # Examples
//!
//! ```rust
//! use loopdev::LoopControl;
//! let lc = LoopControl::open().unwrap();
//! let ld = lc.next_free().unwrap();
//!
//! println!("{}", ld.get_path().unwrap().display());
//!
//! ld.attach("test.img", 0).unwrap();
//! // ...
//! ld.detach().unwrap();
//! ```

extern crate libc;

use std::fs::OpenOptions;
use std::fs::File;

use std::os::unix::prelude::*;
use std::io;
use std::path::PathBuf;
use libc::{c_int, ioctl, uint8_t, uint32_t, uint64_t};
use std::default::Default;

static LOOP_SET_FD: u64 = 0x4C00;
static LOOP_CLR_FD: u64 = 0x4C01;
static LOOP_SET_STATUS64: u64 = 0x4C04;
static LOOP_CTL_GET_FREE: u64 = 0x4C82;

const LOOP_CONTROL: &'static str = "/dev/loop-control";
const LOOP_PREFIX: &'static str = "/dev/loop";

/// Interface to the loop control device: `/dev/loop-control`.
#[derive(Debug)]
pub struct LoopControl {
    dev_file: File,
}

impl LoopControl {
    /// Opens the loop control device.
    pub fn open() -> io::Result<LoopControl> {
        Ok(LoopControl {
            dev_file: try!(OpenOptions::new().read(true).write(true).open(LOOP_CONTROL)),
        })
    }

    /// Finds and opens the next available loop device.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loopdev::LoopControl;
    /// let lc = LoopControl::open().unwrap();
    /// let ld = lc.next_free().unwrap();
    /// println!("{}", ld.get_path().unwrap().display());
    /// ```
    pub fn next_free(&self) -> io::Result<LoopDevice> {
        let result;
        unsafe {
            result = ioctl(self.dev_file.as_raw_fd() as c_int, LOOP_CTL_GET_FREE);
        }
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(try!(LoopDevice::open(&format!("{}{}", LOOP_PREFIX, result))))
        }
    }
}

/// Interface to a loop device ie `/dev/loop0`.
#[derive(Debug)]
pub struct LoopDevice {
    device: File,
}

impl LoopDevice {
    /// Opens a loop device.
    pub fn open(dev: &str) -> io::Result<LoopDevice> {
        // TODO create dev if it does not exist and begins with LOOP_PREFIX
        let f = try!(OpenOptions::new().read(true).write(true).open(dev));
        Ok(LoopDevice { device: f })
    }

    /// Attach the loop device to a file starting at offset into the file.
    ///
    /// # Examples
    ///
    /// Attach the device to the start of a file.
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop4").unwrap();
    /// ld.attach("test.img", 0).unwrap();
    /// # ld.detach().unwrap();
    /// ```
    pub fn attach(&self, backing_file: &str, offset: u64) -> io::Result<()> {
        let bf = try!(OpenOptions::new().read(true).write(true).open(backing_file));

        // Attach the file
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
                try!(self.detach());
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Get the path of the loop device.
    pub fn get_path(&self) -> Option<PathBuf> {
        let mut p = PathBuf::from("/proc/self/fd");
        p.push(self.device.as_raw_fd().to_string());
        std::fs::read_link(&p).ok()
    }

    /// Detach a loop device from its backing file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop6").unwrap();
    /// # ld.attach("test.img", 0).unwrap();
    /// ld.detach().unwrap();
    /// ```
    pub fn detach(&self) -> io::Result<()> {
        unsafe {
            if ioctl(self.device.as_raw_fd() as c_int, LOOP_CLR_FD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

#[repr(C)]
struct loop_info64 {
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
