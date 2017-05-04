#[macro_use]
extern crate clap;
extern crate loopdev;

use std::io::Write;
use std::process::exit;
use loopdev::{LoopControl, LoopDevice};

macro_rules! exit_on_error {
    ($e:expr) => ({match $e {
            Ok(d) => d,
            Err(err) => {
                writeln!(&mut std::io::stderr(), "{}", err).unwrap();
                exit(1)
            },
        }
    })
}

fn find() {
    match LoopControl::open().and_then(|lc| lc.next_free()) {
        Ok(ld) => println!("{}", ld.get_path().unwrap().display()),
        Err(err) => {
            writeln!(&mut std::io::stderr(), "{}", err).unwrap();
            exit(1)
        }
    }
}

fn attach(image: &str, loopdev: Option<&str>, offset: u64) {
    exit_on_error!(match loopdev {
                           None => LoopControl::open().and_then(|lc| lc.next_free()),
                           Some(dev) => LoopDevice::open(&dev),
                       }
                       .and_then(|ld| ld.attach(&image, offset)))
}

fn detach(dev: &str) {
    exit_on_error!(LoopDevice::open(dev).and_then(|ld| ld.detach()))
}

fn list(_free: bool, _used: bool) {
    unimplemented!();
}

fn main() {
    let matches = clap_app!(losetup =>
        (version: "0.1.2")
        (author: "Michael Daffin <michael@daffin.io>")
        (about: "Setup and control loop devices")
        (@subcommand find =>
            (about: "find the next free loop device")
	)
        (@subcommand attach =>
            (about: "attach the loop device to a backing file")
	    (@arg image: +required "the backing file to attach")
	    (@arg loopdev: "the loop device to attach")
            (@arg offset: -o --offset +takes_value "the offset within the file to start at")
	)
        (@subcommand detach =>
            (about: "detach the loop device from the backing file")
	    (@arg file: +required "The file to detach")
	)
        (@subcommand list =>
            (about: "list the available loop devices")
            (@arg free: -f --free "find free devices")
            (@arg used: -u --used "find used devices")
	)
    ).get_matches();

    if let Some(_) = matches.subcommand_matches("find") {
        find();
    } else if let Some(matches) = matches.subcommand_matches("attach") {
        let image = matches.value_of("image").unwrap();
        let loopdev = matches.value_of("loopdev");
        attach(image, loopdev, matches.value_of("offset").unwrap_or(0));
    } else if let Some(matches) = matches.subcommand_matches("detach") {
        let file = matches.value_of("file").unwrap();
        detach(file);
    } else {
        list(matches.is_present("free"), matches.is_present("used"));
    }
}
