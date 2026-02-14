# Stepreduce

This library is a 1:1 port of the [stepreduce C++ library](https://gitlab.com/sethhillbrand/stepreduce) by
Seth Hillbrand to Rust. It compresses STEP CAD files in a lossless way.

The initial conversion was done with the help of Claude Opus 4.6.

## Library

Use the `reduce` function. Example:

```rust
use stepreduce::{ReduceOptions, reduce};

let step_data = b"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=FOO('x');\nENDSEC;\nEND-ISO-10303-21;\n";
let opts = ReduceOptions::default();
let reduced = reduce(step_data, &opts).unwrap();
assert!(!reduced.is_empty());
```

## CLI Binary

This project includes a Rust library and an optional CLI binary.

To build the binary, enable the `cli` Cargo feature (enabled by default).

## Tests

This repository contains a set of roughly 80 test files generated with the
original C++ program. Tests that verify identical output are run with `cargo
test`.

A larger test corpus is available in the separate
[`stepreduce-rs-tests`](https://github.com/dbrgn/stepreduce-rs-tests) repository.

## License

As the original C++ project, this project is released under the GNU GPL v3.0 or later. See `LICENSE` file.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License
    as published by the Free Software Foundation; either version 3
    of the License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program; if not, you may find one here:
    https://www.gnu.org/licenses/old-licenses/gpl-3.0.html
    or you may search the https://www.gnu.org website for the version 3 license,
    or you may write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA
