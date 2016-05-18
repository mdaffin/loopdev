extern crate rustc_serialize;
extern crate docopt;
extern crate loopdev;

use docopt::Docopt;
use std::io::Write;
use std::process::exit;
use loopdev::{LoopControl, LoopDevice};

const USAGE: &'static str = "
Usage:
 losetup attach [--offset=<num>] <image> [<loopdev>]
 losetup detach <file>
 losetup find
 losetup [list] [--free|--used]
 losetup (--help|--version)

Set up and control loop devices.

Options:
 -f, --free          find unused devices
 -u, --used          find used devices
 -o, --offset <num>  start at at <num> into file
 -h, --help          display this help and exit
 -V, --version       output version information and exit
";

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

#[derive(Debug, RustcDecodable)]
struct Args {
    cmd_attach: bool,
    cmd_detach: bool,
    cmd_find: bool,
    cmd_list: bool,
    arg_image: String,
    arg_loopdev: Option<String>,
    arg_file: String,
    flag_free: bool,
    flag_used: bool,
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

fn attach(image: String, loopdev: Option<String>) {
    exit_on_error!(match loopdev {
                       None => LoopControl::open().and_then(|lc| lc.next_free()),
                       Some(dev) => LoopDevice::open(&dev),
                   }
                   .and_then(|ld| ld.attach(&image, 0)))
}

#[allow(unused_variables)]
fn detach(dev: String) {
    exit_on_error!(LoopDevice::open(&dev).and_then(|ld| ld.detach()))
}

fn list(free: bool, used: bool) {
    exit_on_error!(Err(String::from("TODO: list loop devices")))
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());
    if args.cmd_find {
        find();
    } else if args.cmd_attach {
        attach(args.arg_image, args.arg_loopdev);
    } else if args.cmd_detach {
        detach(args.arg_file);
    } else {
        // No flags given default to find all
        if !args.flag_free && !args.flag_used {
            list(true, true)
        } else {
            list(args.flag_free, args.flag_used);
        }
    }
}
