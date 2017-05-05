[![Build Status](https://travis-ci.org/mdaffin/loopdev.svg?branch=master)](https://travis-ci.org/mdaffin/loopdev)
[![crates.io](https://img.shields.io/crates/v/loopdev.svg)](https://crates.io/crates/loopdev)

# loopdev

Setup and control loop devices.

Provides rust interface with similar functionalty to the linux utility `losetup`.

## [Documentation](https://docs.rs/crate/loopdev)

## Examples

```rust
use loopdev::LoopControl;
let lc = LoopControl::open().unwrap();
let ld = lc.next_free().unwrap();

println!("{}", ld.get_path().unwrap().display());

ld.attach("test.img", 0).unwrap();
// ...
ld.detach().unwrap();
```
