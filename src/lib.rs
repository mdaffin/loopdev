#[cfg(not(test))]
extern crate libc;
#[cfg(test)]
extern crate libc as real_libc;
extern crate errno;

use libc::{c_int, O_RDWR, open, close, ioctl};
use std::path::PathBuf;

const LOOP_PREFIX: &'static str = "/dev/loop";

#[derive(Debug,PartialEq)]
pub struct LoopControl {
    fd: c_int,
}

// A wrapper around libc::open
fn open_wrapper(f: &str) -> Result<c_int, String> {
    let fd: c_int;
    let loctl = std::ffi::CString::new(f).unwrap();
    unsafe {
        fd = open(loctl.as_ptr(), O_RDWR);
    }

    if fd < 0 {
        Err(format!("{}: {}", f, errno::errno()))
    } else {
        Ok(fd)
    }
}

// Opens /dev/loop-control
pub fn open_loop_control(dev: &str) -> Result<LoopControl, String> {
    open_wrapper(dev).map(|fd| LoopControl { fd: fd })
}

impl LoopControl {
    // Finds and returns the next availble loop device
    pub fn next_free(&self) -> Result<LoopDevice, String> {
        assert!(self.fd >= 0);
        let result: i32;
        unsafe {
            result = ioctl(self.fd, 0x4C82);
        }
        if result < 0 {
            Err(String::from(format!("{}", errno::errno())))
        } else {
            Ok(LoopDevice {
                device: PathBuf::from(LOOP_PREFIX.to_string() + &result.to_string()),
                backing_file: None,
                device_fd: None,
                backing_file_fd: None,
            })
        }
    }
}

impl Drop for LoopControl {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
        };
    }
}

#[derive(Debug,PartialEq)]
pub struct LoopDevice {
    pub device: PathBuf,
    backing_file: Option<PathBuf>,
    device_fd: Option<c_int>,
    backing_file_fd: Option<c_int>,
}

impl LoopDevice {

    // Attach a loop device to a file.
    pub fn attach(&self, backing_file: PathBuf) -> Result<(), String> {
        // self.device_fd = try!(open_wrapper(self.device.to_str().unwrap()));
        // self.backing_file_fd = try!(open_wrapper(try!(backing_file.to_str().ok_or(String::from("backing file not valid utf8")))));



        Ok(())
    }

    // Detach a loop device from its backing file.
    pub fn detach(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod libc {
    use errno;
    use std::cell::RefCell;
    pub use real_libc::{c_int, c_ulong, c_char, O_RDWR};
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
    pub unsafe extern "C" fn open(path: *const c_char, oflag: c_int) -> c_int {
        get_return_value()
    }

    #[allow(unused_variables)]
    pub unsafe extern "C" fn close(fd: c_int) -> c_int {
        get_return_value()
    }

    #[allow(unused_variables)]
    pub unsafe extern "C" fn ioctl(fd: c_int, request: c_ulong) -> c_int {
        get_return_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc;
    use std::path::PathBuf;

    macro_rules! lc_open {
        ( $name:ident, $x:expr, $e:expr ) => {
            #[test]
            fn $name() {
                libc::set_return_value($x);
                assert_eq!(open_loop_control("/dev/loop-control"), $e);
            }
        };
    }

    lc_open!(lc_open_0, 0, Ok(LoopControl { fd: 0 }));
    lc_open!(lc_open_1, 1, Ok(LoopControl { fd: 1 }));
    lc_open!(lc_open_2, 2, Ok(LoopControl { fd: 2 }));
    lc_open!(lc_open_100, 100, Ok(LoopControl { fd: 100 }));
    lc_open!(lc_open_large,
             1024 * 1024,
             Ok(LoopControl { fd: 1024 * 1024 }));
    lc_open!(lc_open_err,
             -1,
             Err(String::from("/dev/loop-control: Operation not permitted")));

    macro_rules! lc_next_free {
        ( $name:ident, $inp:expr, $out:expr, $exp:expr ) => {
            #[test]
            fn $name() {
                libc::set_return_value($out);
                let lc = LoopControl { fd: $inp };
                assert_eq!(lc.next_free(), $exp);
            }
        };
    }

    lc_next_free!(lc_next_free_0, 0, 0, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop0")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_1, 1, 1, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop1")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_2, 2, 2, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop2")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_3, 5, 5, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop5")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_4, 10, 10, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop10")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_5, 54, 54, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop54")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_7, 128, 128, Ok(LoopDevice { device: PathBuf::from(String::from("/dev/loop128")), backing_file: None, device_fd: None, backing_file_fd: None }));
    lc_next_free!(lc_next_free_err_1,
                  123,
                  -1,
                  Err(String::from("Operation not permitted")));
    lc_next_free!(lc_next_free_err_2,
                  123,
                  -2,
                  Err(String::from("No such file or directory")));
    lc_next_free!(lc_next_free_err_3,
                  123,
                  -3,
                  Err(String::from("No such process")));
    #[test]
    #[should_panic(expected = "assertion failed")]
    #[allow(unused_must_use)]
    fn ln_next_free_panic() {
        libc::set_return_value(-1);
        let lc = LoopControl { fd: -1 };
        lc.next_free();
    }
}
