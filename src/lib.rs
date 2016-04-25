#[cfg(not(test))]
extern crate libc;
#[cfg(test)]
extern crate libc as real_libc;
extern crate errno;

use libc::{c_int, O_RDWR, open, close, ioctl};

const LOOP_PREFIX: &'static str = "/dev/loop";

#[derive(Debug,PartialEq)]
pub struct LoopControl {
    fd: libc::c_int,
}

pub fn open_loop_control(dev: &str) -> Result<LoopControl, String> {
    let fd: c_int;
    let loctl = std::ffi::CString::new(dev).unwrap();

    unsafe {
        fd = open(loctl.as_ptr(), O_RDWR);
        if fd < 0 {
            return Err(format!("{}: {}", dev, errno::errno()));
        }
    };
    Ok(LoopControl { fd: fd })
}

impl LoopControl {
    pub fn next_free(&self) -> Result<String, String> {
        let result: i32;
        unsafe {
            result = ioctl(self.fd, 0x4C82);
        }
        if result < 0 {
            panic!("result < 0");
        }
        Ok(format!("{}{}", LOOP_PREFIX, result))
    }
}

impl Drop for LoopControl {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
        };
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

    macro_rules! lc_open {
        ( $name:ident, $x:expr ) => {
            #[test]
            fn $name() {
                libc::set_return_value($x);
                assert_eq!(open_loop_control("/dev/loop-control").unwrap(),
                           LoopControl { fd: $x });
            }
        };
        ( $name:ident, $x:expr, $e:expr ) => {
            #[test]
            fn $name() {
                libc::set_return_value($x);
                assert_eq!(open_loop_control("/dev/loop-control"), $e);
            }
        };
    }

    lc_open!(lc_open_0, 0);
    lc_open!(lc_open_1, 1);
    lc_open!(lc_open_2, 2);
    lc_open!(lc_open_100, 100);
    lc_open!(lc_open_large, 1024 * 1024 * 1024);
    lc_open!(lc_open_err,
             -1,
             Err(String::from("/dev/loop-control: Operation not permitted")));

    #[test]
    fn lc_next_free() {
        let lc = LoopControl { fd: 2 };
        println!("{:?}", lc.next_free().unwrap());
    }
}
