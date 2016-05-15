extern crate rustc_serialize;
extern crate docopt;
extern crate loopdev;

use docopt::Docopt;
use std::io::Write;
use std::process::exit;

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

fn find() -> Result<String, String> {
    let lc = loopdev::open_loop_control("/dev/loop-control");
    let ld = try!(lc.map_err(|e| format!("{}", e)).and_then(|l| l.next_free()));
    Ok(String::from(ld.device.to_str().unwrap()))
}

fn attach(image: String, loopdev: String) -> Result<(), String> {
    println!("{} : {}", image, loopdev);
    Err(String::from("TODO: command attach"))
}

fn detach(file: String) -> Result<(), String> {
    Err(String::from("TODO: command detach"))
}

fn list() -> Result<(), String> {
    Err(String::from("TODO: command list"))
}

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

fn main() {
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());
    if args.cmd_find {
        println!("{}", exit_error!(find()));
    } else if args.cmd_attach {
        let loopdev = args.arg_loopdev.unwrap_or(exit_error!(find()));
        exit_error!(attach(args.arg_image, loopdev));
    } else if args.cmd_detach {
        exit_error!(detach(args.arg_file));
    } else {
        exit_error!(list());
    }
}
