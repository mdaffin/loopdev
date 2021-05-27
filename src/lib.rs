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

use bindings::{
    loop_info64, LOOP_CLR_FD, LOOP_CTL_GET_FREE, LOOP_SET_CAPACITY, LOOP_SET_FD, LOOP_SET_STATUS64,
};
use libc::{c_int, ioctl};
use std::fs::{File, OpenOptions};
use std::{
    default::Default,
    io,
    os::unix::prelude::*,
    path::{Path, PathBuf},
};

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[allow(non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

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

    /// Attach the loop device to a file with given options.
    ///
    /// # Examples
    ///
    /// Attach the device to a file.
    ///
    /// ```rust
    /// use loopdev::LoopDevice;
    /// let mut ld = LoopDevice::open("/dev/loop3").unwrap();
    /// ld.with().part_scan(true).attach("test.img").unwrap();
    /// # ld.detach().unwrap();
    /// ```
    pub fn with(&mut self) -> AttachOptions<'_> {
        AttachOptions {
            device: self,
            info: Default::default(),
        }
    }

    /// Attach the loop device to a file starting at offset into the file.
    #[deprecated(
        since = "0.2.0",
        note = "use `loop.with().offset(offset).attach(file)` instead"
    )]
    pub fn attach<P: AsRef<Path>>(&self, backing_file: P, offset: u64) -> io::Result<()> {
        let info = loop_info64 {
            lo_offset: offset,
            ..Default::default()
        };

        Self::attach_with_loop_info(self, backing_file, info)
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
        let info = loop_info64 {
            ..Default::default()
        };

        Self::attach_with_loop_info(self, backing_file, info)
    }

    /// Attach the loop device to a file starting at offset into the file.
    #[deprecated(
        since = "0.2.2",
        note = "use `loop.with().offset(offset).attach(file)` instead"
    )]
    pub fn attach_with_offset<P: AsRef<Path>>(
        &self,
        backing_file: P,
        offset: u64,
    ) -> io::Result<()> {
        let info = loop_info64 {
            lo_offset: offset,
            ..Default::default()
        };

        Self::attach_with_loop_info(self, backing_file, info)
    }

    /// Attach the loop device to a file starting at offset into the file and a the given sizelimit.
    #[deprecated(
        since = "0.2.2",
        note = "use `with().size_limit(size).attach(file)` instead"
    )]
    pub fn attach_with_sizelimit<P: AsRef<Path>>(
        &self,
        backing_file: P,
        offset: u64,
        size_limit: u64,
    ) -> io::Result<()> {
        let info = loop_info64 {
            lo_offset: offset,
            lo_sizelimit: size_limit,
            ..Default::default()
        };

        Self::attach_with_loop_info(self, backing_file, info)
    }

    /// Attach the loop device to a file with loop_info64.
    fn attach_with_loop_info(
        &self, // TODO should be mut? - but changing it is a breaking change
        backing_file: impl AsRef<Path>,
        info: loop_info64,
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
                &info,
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
    /// let ld = LoopDevice::open("/dev/loop5").unwrap();
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

/// Used to set options when attaching a device. Created with [LoopDevice::with()].
///
/// # Examples
///
/// Enable partition scanning on attach:
///
/// ```rust
/// use loopdev::LoopDevice;
/// let mut ld = LoopDevice::open("/dev/loop6").unwrap();
/// ld.with()
///     .part_scan(true)
///     .attach("test.img")
///     .unwrap();
/// # ld.detach().unwrap();
/// ```
///
/// A 1MiB slice of the file located at 1KiB into the file.
///
/// ```rust
/// use loopdev::LoopDevice;
/// let mut ld = LoopDevice::open("/dev/loop7").unwrap();
/// ld.with()
///     .offset(1024*1024)
///     .size_limit(1024*1024*1024)
///     .attach("test.img")
///     .unwrap();
/// # ld.detach().unwrap();
/// ```
pub struct AttachOptions<'d> {
    device: &'d mut LoopDevice,
    info: loop_info64,
}

impl AttachOptions<'_> {
    /// Offset in bytes from the start of the backing file the data will start at.
    pub fn offset(mut self, offset: u64) -> Self {
        self.info.lo_offset = offset;
        self
    }

    /// Maximum size of the data in bytes.
    pub fn size_limit(mut self, size_limit: u64) -> Self {
        self.info.lo_sizelimit = size_limit;
        self
    }

    /// Force the kernel to scan the partition table on a newly created loop device. Note that the
    /// partition table parsing depends on sector sizes. The default is sector size is 512 bytes
    pub fn part_scan(mut self, enable: bool) -> Self {
        if enable {
            self.info.lo_flags |= 1 << 4;
        } else {
            self.info.lo_flags &= u32::max_value() - (1 << 4);
        }
        self
    }

    /// Attach the loop device to a file with the set options.
    pub fn attach(self, backing_file: impl AsRef<Path>) -> io::Result<()> {
        self.device.attach_with_loop_info(backing_file, self.info)
    }
}

fn ioctl_to_error(ret: i32) -> io::Result<i32> {
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}
