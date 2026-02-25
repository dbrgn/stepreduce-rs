# Stepreduce

This library is a port of the [stepreduce C++ library](https://gitlab.com/sethhillbrand/stepreduce) by
Seth Hillbrand to Rust. It compresses STEP CAD files in a lossless way. The implementation is roughly 5x
faster compared to the original implementation (see section _Performance_ below).

_(Update 2026-02-25: After [some
optimizations](https://gitlab.com/sethhillbrand/stepreduce/-/issues/3) the C++
project is now slightly faster again than this Rust codebase.)_

## Reliability

**DISCLAIMER:** The initial conversion as well as Bugfixing was done with the help of Claude Opus 4.6. While I
(the crate author) reviewed the generated code (which is not very hard to read), I have only superficial
knowledge of how the STEP format works, and thus I currently cannot guarantee the correctness of the STEP
reduction logic beyond the automated test suite. (Note however that the original logic contained at least one
bug that resulted in broken STEP files, which is fixed in this codebase.)

This project is an experiment, to see how far one can get with early 2026 LLMs to do such a conversion when
guided by a test suite. While the project uses lots of test files to ensure that the generated output
generally corresponds to the original project and is also correct, if reliability is a top priority to you,
then either:

- Don't use this crate
- ...or even better, review the code to ensure the STEP reduction logic is sound (and let me know)

I'm also happy to transfer the crate to anyone that would want to keep maintaining it.

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

## Performance

_(Update 2026-02-25: After [some
optimizations](https://gitlab.com/sethhillbrand/stepreduce/-/issues/3) the C++
project is now slightly faster again than this Rust codebase.)_

Surprisingly, given a 1 MiB test file (`00010546_919044145dd24288a1945b5c_step_008.step`) on a AMD Ryzen 9
5900X CPU, the initial version of the Rust code was roughly 3.5x faster than the original C++ code (145 vs 505
ms). This is probably attributable primarily to the use of the Rust `regex` crate, which is known to have a
high-performance implementation, while the [C++ std::regex library is known to be
slow](https://stackoverflow.com/questions/70583395/why-is-stdregex-notoriously-much-slower-than-other-regular-expression-librarie).

Combined with a few other optimizations, **this library ends up being roughly 5x faster than the original C++
stepreduce library**.

Hyperfine test run:

```
$ hyperfine --warmup 3 \
  'bench/cpp-baseline bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null' \
  'bench/rs-baseline bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null' \
  'bench/rs-find-numbers bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null' \
  'bench/rs-prealloc bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null'

Benchmark 1: bench/cpp-baseline bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
  Time (mean ± σ):     505.4 ms ±   2.7 ms    [User: 497.4 ms, System: 5.8 ms]
  Range (min … max):   501.9 ms … 510.6 ms    10 runs

Benchmark 2: bench/rs-baseline bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
  Time (mean ± σ):     145.2 ms ±   2.2 ms    [User: 137.9 ms, System: 6.6 ms]
  Range (min … max):   142.0 ms … 149.8 ms    20 runs

Benchmark 3: bench/rs-find-numbers bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
  Time (mean ± σ):     104.5 ms ±   1.7 ms    [User: 98.3 ms, System: 5.6 ms]
  Range (min … max):   101.1 ms … 106.8 ms    28 runs

Benchmark 4: bench/rs-prealloc bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
  Time (mean ± σ):      97.2 ms ±   1.8 ms    [User: 90.8 ms, System: 5.9 ms]
  Range (min … max):    94.5 ms … 101.8 ms    31 runs

Summary
  bench/rs-prealloc bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null ran
    1.07 ± 0.03 times faster than bench/rs-find-numbers bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
    1.49 ± 0.04 times faster than bench/rs-baseline bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
    5.20 ± 0.10 times faster than bench/cpp-baseline bench/00010546_919044145dd24288a1945b5c_step_008.step /dev/null
```

Note: Further performance gains could be achieved by replacing the `REF_PATTERN` regex with explicit Rust
code, but in that case it was not worth the additional code complexity compared to the speedup.

## Tests

This repository contains a set of roughly 80 test files generated with the
original C++ program. Tests that verify identical output are run with `cargo
test`.

Additionally, there are correctness tests in the `validation/` directory, which ensure that
certain geometric properties of a model don't change with the reduction.

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
