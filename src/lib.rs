#[cfg(not(test))]
extern crate libc;
#[cfg(test)]
extern crate libc as real_libc;
extern crate errno;

use libc::{c_int, O_RDWR, open, close, ioctl};

const LOOP_PREFIX: &'static str = "/dev/loop";

#[derive(Debug,PartialEq)]
pub struct LoopControl {
    fd: c_int,
}

pub fn open_loop_control(dev: &str) -> Result<LoopControl, String> {
    let fd: c_int;
    let loctl = std::ffi::CString::new(dev).unwrap();
    unsafe {
        fd = open(loctl.as_ptr(), O_RDWR);
    }

    if fd < 0 {
        Err(format!("{}: {}", dev, errno::errno()))
    } else {
        Ok(LoopControl { fd: fd })
    }
}

impl LoopControl {
    pub fn next_free(&self) -> Result<String, String> {
        assert!(self.fd >= 0);
        let result: i32;
        unsafe {
            result = ioctl(self.fd, 0x4C82);
        }
        if result < 0 {
            Err(String::from(format!("{}", errno::errno())))
        } else {
            Ok(format!("{}{}", LOOP_PREFIX, result))
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

pub struct LoopDevice {
    device_fd: c_int,
    backing_file_fd: c_int,
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
    lc_open!(lc_open_large, 1024 * 1024, Ok(LoopControl { fd: 1024 * 1024 }));
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

    lc_next_free!(lc_next_free_0, 0, 0, Ok(String::from("/dev/loop0")));
    lc_next_free!(lc_next_free_1, 1, 1, Ok(String::from("/dev/loop1")));
    lc_next_free!(lc_next_free_2, 2, 2, Ok(String::from("/dev/loop2")));
    lc_next_free!(lc_next_free_5, 5, 5, Ok(String::from("/dev/loop5")));
    lc_next_free!(lc_next_free_10, 10, 10, Ok(String::from("/dev/loop10")));
    lc_next_free!(lc_next_free_54, 54, 54, Ok(String::from("/dev/loop54")));
    lc_next_free!(lc_next_free_128, 128, 128, Ok(String::from("/dev/loop128")));
    lc_next_free!(lc_next_free_err_1, 123, -1, Err(String::from("Operation not permitted")));
    lc_next_free!(lc_next_free_err_2, 123, -2, Err(String::from("No such file or directory")));
    lc_next_free!(lc_next_free_err_3, 123, -3, Err(String::from("No such process")));
    #[test]
    #[should_panic(expected = "assertion failed")]
    #[allow(unused_must_use)]
    fn ln_next_free_panic() {
        libc::set_return_value(-1);
        let lc = LoopControl { fd: -1 };
        lc.next_free();
    }
}
