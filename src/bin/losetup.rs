extern crate rustc_serialize;
extern crate docopt;
extern crate loopdev;

use docopt::Docopt;
use std::io::Write;
use std::process::exit;
use loopdev::LoopControl;

const USAGE: &'static str = "
Usage:
 losetup attach <image> [<loopdev>]
 losetup detach <file>
 losetup find
 losetup [list] [--free|--used]
 losetup (--help|--version)

Set up and control loop devices.

Options:
 -f, --free     find unused devices
 -u, --used     find used devices
 -h, --help     display this help and exit
 -V, --version  output version information and exit
";

macro_rules! exit_error {
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
}

fn find() {
    match LoopControl::open("/dev/loop-control").and_then(|lc| lc.next_free()) {
        Ok(ld) => println!("{}", ld.get_path().unwrap().display()),
        Err(err) => {
            writeln!(&mut std::io::stderr(), "{}", err).unwrap();
            exit(1)
        }
    }
}

fn attach(image: String, loopdev: String) {
    let mut ld = LoopControl::open("/dev/loop-control").and_then(|lc| lc.next_free()).unwrap();
    ld.attach(&image, 0).unwrap();
}

#[allow(unused_variables)]
fn detach(file: String) {
    exit_error!(Err(String::from("TODO: command detach")))
}

fn list() {
    exit_error!(Err(String::from("TODO: command list")))
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());
    if args.cmd_find {
        find();
    } else if args.cmd_attach {
        // let loopdev = args.arg_loopdev.unwrap_or(exit_error!(find()));
        attach(args.arg_image, String::from(""));
    } else if args.cmd_detach {
        detach(args.arg_file);
    } else {
        list();
    }
}
