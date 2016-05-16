#[cfg(not(test))]
extern crate libc;

#[cfg(not(test))]
use std::fs::OpenOptions;
#[cfg(not(test))]
use std::fs::File;

#[cfg(test)]
use mocks::{File, OpenOptions};

use std::os::unix::prelude::*;
use std::io;
use std::path::PathBuf;
use libc::{c_int, ioctl};

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
            result = ioctl(self.dev_file.as_raw_fd() as c_int, 0x4C82);
        }
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            let path = LOOP_PREFIX.to_string() + &result.to_string();
            Ok(LoopDevice {
                device: try!(OpenOptions::new().read(true).write(true).open(path)),
                backing_file: None,
            })
        }
    }
}

#[derive(Debug)]
pub struct LoopDevice {
    device: File,
    backing_file: Option<File>,
}

impl LoopDevice {
    // Attach a loop device to a file.
    #[allow(unused_variables)]
    pub fn attach(&self, backing_file: File, offset: u64) -> io::Result<()> {
        // open backing_file (rdwr)
        // open device file (rdwr)
        // ioctl(device_fd, LOOP_SET_FD, file_fd)
        // info.lo_offset = offset;
        // ioctl(device_fd, LOOP_SET_STATUS64, &info)
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

#[cfg(test)]
mod mocks {
    use std::io;
    use std::path::Path;
    use libc::c_int;
    use std::os::unix::prelude::AsRawFd;

    pub type RawFd = c_int;

    #[derive(Debug)]
    pub struct File {
        fd: RawFd,
        read: bool,
        write: bool,
    }

    impl File {
        pub fn new(fd: RawFd, read: bool, write: bool) -> File {
            File {
                fd: fd,
                read: read,
                write: write,
            }
        }
    }

    impl AsRawFd for File {
        fn as_raw_fd(&self) -> RawFd {
            self.fd
        }
    }

    pub struct OpenOptions {
        read: bool,
        write: bool,
    }

    impl OpenOptions {
        pub fn new() -> OpenOptions {
            OpenOptions {
                read: false,
                write: false,
            }
        }

        pub fn read(&mut self, en: bool) -> &mut OpenOptions {
            self.read = en;
            self
        }

        pub fn write(&mut self, en: bool) -> &mut OpenOptions {
            self.write = en;
            self
        }

        #[allow(unused_variables)]
        pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
            if path.as_ref().to_str().unwrap() == "/dev/null" {
                Err(io::Error::new(io::ErrorKind::PermissionDenied,
                                   "/dev/null: Operation not permitted"))
            } else {
                Ok(File {
                    fd: 3,
                    read: self.read,
                    write: self.write,
                })
            }
        }
    }

}

#[cfg(test)]
mod libc {
    extern crate libc as real_libc;
    extern crate errno;

    use std::cell::RefCell;
    pub use self::real_libc::{c_int, c_ulong};

    thread_local!(static RETURN_VALUE: RefCell<c_int> = RefCell::new(0));

    pub fn set_return_value(value: c_int) {
        RETURN_VALUE.with(|v| {
            *v.borrow_mut() = value;
        })
    }

    fn get_return_value() -> c_int {
        RETURN_VALUE.with(|v| {
            if *v.borrow() < 0 {
                errno::set_errno(errno::Errno(-*v.borrow()));
            }
            *v.borrow()
        })
    }

    #[allow(unused_variables)]
    pub unsafe extern "C" fn ioctl(fd: c_int, request: c_ulong) -> c_int {
        get_return_value()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use mocks::File;
    use std::io;
    use libc;

    macro_rules! lc_open {
        ( $name:ident, $i:expr, $r:expr ) => {
            #[test]
            fn $name() {
                let r: io::Result<LoopControl> = $r;
                assert_eq!(format!("{:?}", LoopControl::open($i)), format!("{:?}", r));
            }
        };
    }
    lc_open!(lc_open_ok,
             "/dev/loop-control",
             Ok(LoopControl { dev_file: File::new(3, true, true) }));
    lc_open!(lc_open_err,
             "/dev/null",
             Err(io::Error::new(io::ErrorKind::PermissionDenied,
                                "/dev/null: Operation not permitted")));

    macro_rules! lc_next_free {
        ( $name:ident, $out:expr, $exp:expr ) => {
            #[test]
            fn $name() {
                libc::set_return_value($out);
                let lc = LoopControl { dev_file: File::new(3, true, true) };
                let exp: io::Result<LoopDevice> = $exp;
                assert_eq!(format!("{:?}", lc.next_free()), format!("{:?}", exp));
            }
        };
    }
    lc_next_free!(lc_next_free_0,
                  0,
                  Ok(LoopDevice {
                      device: File::new(3, true, true),
                      backing_file: None,
                  }));
    lc_next_free!(lc_next_free_1,
                  1,
                  Ok(LoopDevice {
                      device: File::new(3, true, true),
                      backing_file: None,
                  }));
    lc_next_free!(lc_next_free_2,
                  2,
                  Ok(LoopDevice {
                      device: File::new(3, true, true),
                      backing_file: None,
                  }));
    lc_next_free!(lc_next_free_10,
                  10,
                  Ok(LoopDevice {
                      device: File::new(3, true, true),
                      backing_file: None,
                  }));
    lc_next_free!(lc_next_free_99,
                  99,
                  Ok(LoopDevice {
                      device: File::new(3, true, true),
                      backing_file: None,
                  }));
    lc_next_free!(lc_next_free_100,
                  100,
                  Ok(LoopDevice {
                      device: File::new(3, true, true),
                      backing_file: None,
                  }));
    lc_next_free!(lc_next_free_err_1, -1, Err(io::Error::from_raw_os_error(1)));
    lc_next_free!(lc_next_free_err_2, -2, Err(io::Error::from_raw_os_error(2)));
    lc_next_free!(lc_next_free_err_3, -3, Err(io::Error::from_raw_os_error(3)));
}
