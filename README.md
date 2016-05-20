[![Build Status](https://travis-ci.org/mdaffin/loopdev.svg?branch=master)](https://travis-ci.org/mdaffin/loopdev)

# loopdev

Setup and control loop devices.

Provides rust interface with similar functionalty to the linux utility `losetup`.

# Examples

```rust
use loopdev::LoopControl;
let lc = LoopControl::open().unwrap();
let ld = lc.next_free().unwrap();

println!("{}", ld.get_path().unwrap().display());

ld.attach("test.img", 0).unwrap();
// ...
ld.detach().unwrap();
```
