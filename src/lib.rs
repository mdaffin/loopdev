//! Setup and control loop devices.
//!
//! Provides rust interface with similar functionality to the Linux utility `losetup`.
//!
//! # Examples
//!
//! ```rust
//! use loopdev::LoopControl;
//! let lc = LoopControl::open().unwrap();
//! let ld = lc.next_free().unwrap();
//!
//! println!("{}", ld.path().unwrap().display());
//!
//! ld.attach_file("test.img").unwrap();
//! // ...
//! ld.detach().unwrap();
//! ```

extern crate libc;

use std::fs::File;
use std::fs::OpenOptions;

use libc::{c_int, ioctl};
use std::default::Default;
use std::io;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};

// TODO support missing operations
const LOOP_SET_FD: u16 = 0x4C00;
const LOOP_CLR_FD: u16 = 0x4C01;
const LOOP_SET_STATUS64: u16 = 0x4C04;
//const LOOP_GET_STATUS64: u16 = 0x4C05;
const LOOP_SET_CAPACITY: u16 = 0x4C07;
//const LOOP_SET_DIRECT_IO: u16 = 0x4C08;
//const LOOP_SET_BLOCK_SIZE: u16 = 0x4C09;

//const LOOP_CTL_ADD: u16 = 0x4C80;
//const LOOP_CTL_REMOVE: u16 = 0x4C81;
const LOOP_CTL_GET_FREE: u16 = 0x4C82;

const LOOP_CONTROL: &str = "/dev/loop-control";
const LOOP_PREFIX: &str = "/dev/loop";

/// Interface to the loop control device: `/dev/loop-control`.
#[derive(Debug)]
pub struct LoopControl {
    dev_file: File,
}

impl LoopControl {
    /// Opens the loop control device.
    pub fn open() -> io::Result<Self> {
        Ok(Self {
            dev_file: OpenOptions::new()
                .read(true)
                .write(true)
                .open(LOOP_CONTROL)?,
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
    /// println!("{}", ld.path().unwrap().display());
    /// ```
    pub fn next_free(&self) -> io::Result<LoopDevice> {
        let dev_num = ioctl_to_error(unsafe {
            ioctl(self.dev_file.as_raw_fd() as c_int, LOOP_CTL_GET_FREE.into())
        })?;
        LoopDevice::open(&format!("{}{}", LOOP_PREFIX, dev_num))
    }
}

/// Interface to a loop device ie `/dev/loop0`.
#[derive(Debug)]
pub struct LoopDevice {
    device: File,
}

impl AsRawFd for LoopDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.device.as_raw_fd()
    }
}

impl LoopDevice {
    /// Opens a loop device.
    pub fn open<P: AsRef<Path>>(dev: P) -> io::Result<Self> {
        // TODO create dev if it does not exist and begins with LOOP_PREFIX
        Ok(Self {
            device: OpenOptions::new().read(true).write(true).open(dev)?,
        })
    }

    /// Attach the loop device to a file starting at offset into the file.
    #[deprecated(
        since = "0.2.0",
        note = "use `attach_file` or `attach_with_offset` instead"
    )]
    pub fn attach<P: AsRef<Path>>(&self, backing_file: P, offset: u64) -> io::Result<()> {
        self.attach_with_sizelimit(backing_file, offset, 0)
    }

    /// Attach the loop device to a file that maps to the whole file.
    ///
    /// # Examples
    ///
    /// Attach the device to a file.
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop4").unwrap();
    /// ld.attach_file("test.img").unwrap();
    /// # ld.detach().unwrap();
    /// ```
    pub fn attach_file<P: AsRef<Path>>(&self, backing_file: P) -> io::Result<()> {
        self.attach_with_sizelimit(backing_file, 0, 0)
    }

    /// Attach the loop device to a file starting at offset into the file.
    ///
    /// # Examples
    ///
    /// Attach the device to the start of a file.
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop5").unwrap();
    /// ld.attach_with_offset("test.img", 0).unwrap();
    /// # ld.detach().unwrap();
    /// ```
    pub fn attach_with_offset<P: AsRef<Path>>(
        &self,
        backing_file: P,
        offset: u64,
    ) -> io::Result<()> {
        self.attach_with_sizelimit(backing_file, offset, 0)
    }

    /// Attach the loop device to a file starting at offset into the file and a the given sizelimit.
    ///
    /// # Examples
    ///
    /// Attach the device to the start of a file with a maximum size of 1024 bytes.
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop6").unwrap();
    /// ld.attach_with_sizelimit("test.img", 0, 1024).unwrap();
    /// # ld.detach().unwrap();
    /// ```
    pub fn attach_with_sizelimit<P: AsRef<Path>>(
        &self,
        backing_file: P,
        offset: u64,
        sizelimit: u64,
    ) -> io::Result<()> {
        // Set offset for backing_file
        let mut info = loop_info64::default();
        info.lo_offset = offset;
        info.lo_sizelimit = sizelimit;

        Self::attach_with_loop_info(self, backing_file, info)
    }

    /// Attach the loop device with automatic partition scanning.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop5").unwrap();
    /// ld.attach_with_partscan("test.img").unwrap();
    /// # ld.detach().unwrap();
    /// ```
    pub fn attach_with_partscan<P: AsRef<Path>>(&self, backing_file: P) -> io::Result<()> {
        // Set lo_flags to LO_FLAGS_PARTSCAN value
        let mut info = loop_info64::default();
        info.lo_flags = 8;

        Self::attach_with_loop_info(self, backing_file, info)
    }

    /// Attach the loop device to a file with loop_info.
    fn attach_with_loop_info<P: AsRef<Path>>(
        &self,
        backing_file: P,
        loop_info: loop_info64,
    ) -> io::Result<()> {
        let bf = OpenOptions::new()
            .read(true)
            .write(true)
            .open(backing_file)?;

        // Attach the file
        ioctl_to_error(unsafe {
            ioctl(
                self.device.as_raw_fd() as c_int,
                LOOP_SET_FD.into(),
                bf.as_raw_fd() as c_int,
            )
        })?;

        if let Err(err) = ioctl_to_error(unsafe {
            ioctl(
                self.device.as_raw_fd() as c_int,
                LOOP_SET_STATUS64.into(),
                &loop_info,
            )
        }) {
            // Ignore the error to preserve the original error
            let _ = self.detach();
            return Err(err);
        }
        Ok(())
    }

    /// Get the path of the loop device.
    #[deprecated(since = "0.2.0", note = "use `path` instead")]
    pub fn get_path(&self) -> Option<PathBuf> {
        self.path()
    }

    /// Get the path of the loop device.
    pub fn path(&self) -> Option<PathBuf> {
        let mut p = PathBuf::from("/proc/self/fd");
        p.push(self.device.as_raw_fd().to_string());
        std::fs::read_link(&p).ok()
    }

    /// Detach a loop device from its backing file.
    ///
    /// Note that the device won't fully detach until a short delay after the underling device file
    /// gets closed. This happens when LoopDev goes out of scope so you should ensure the LoopDev
    /// lives for a short a time as possible.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let ld = LoopDevice::open("/dev/loop7").unwrap();
    /// # ld.attach_file("test.img").unwrap();
    /// ld.detach().unwrap();
    /// ```
    pub fn detach(&self) -> io::Result<()> {
        ioctl_to_error(unsafe { ioctl(self.device.as_raw_fd() as c_int, LOOP_CLR_FD.into(), 0) })?;
        Ok(())
    }

    /// Resize a live loop device. If the size of the backing file changes this can be called to
    /// inform the loop driver about the new size.
    pub fn set_capacity(&self) -> io::Result<()> {
        ioctl_to_error(unsafe {
            ioctl(
                self.device.as_raw_fd() as c_int,
                LOOP_SET_CAPACITY.into(),
                0,
            )
        })?;
        Ok(())
    }
}

#[repr(C)]
struct loop_info64 {
    pub lo_device: u64,
    pub lo_inode: u64,
    pub lo_rdevice: u64,
    pub lo_offset: u64,
    pub lo_sizelimit: u64,
    pub lo_number: u32,
    pub lo_encrypt_type: u32,
    pub lo_encrypt_key_size: u32,
    pub lo_flags: u32,
    pub lo_file_name: [u8; 64],
    pub lo_crypt_name: [u8; 64],
    pub lo_encrypt_key: [u8; 32],
    pub lo_init: [u64; 2],
}

impl Default for loop_info64 {
    fn default() -> Self {
        Self {
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

fn ioctl_to_error(ret: i32) -> io::Result<i32> {
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}
