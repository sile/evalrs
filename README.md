evalrs
======

[![Crates.io: evalrs](http://meritbadge.herokuapp.com/evalrs)](https://crates.io/crates/evalrs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust code snippet evaluator.

This tiny command evaluates Rust source code which read from the standard input stream.

It can handle `extern crate` declaration and has simple caching mechanism.

[Documentation](https://docs.rs/crate/evalrs/)

Installation
------------

Execute following command on your terminal:

```bash
# Installs `evalrs` command
$ cargo install evalrs

# Shows help message
$ evalrs -h
```

Usage Examples
--------------

`evalrs` command reads Rust code snippet from the standard input stream and evaluates it:

```bash
$ echo 'println!("Hello World!")' | evalrs
   Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.daiPxHtjV2VR)
    Finished debug [unoptimized + debuginfo] target(s) in 0.51 secs
     Running `target\debug\evalrs_temp.exe`
Hello World!
```

If target code includes `extern crate` declarations,
the latest version of those crates will be downloaded and cached:

```bash
# First time
$ echo 'extern crate num_cpus; println!("{} CPUs", num_cpus::get())' | evalrs
    Updating registry `https://github.com/rust-lang/crates.io-index`
   Compiling libc v0.2.18
   Compiling num_cpus v1.2.0
   Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.HSRNyVQbM6s3)
    Finished debug [unoptimized + debuginfo] target(s) in 0.55 secs
     Running `target\debug\evalrs_temp.exe`
4 CPUs

# Second time (cached crates are used)
$ echo 'extern crate num_cpus; println!("{} CPUs", num_cpus::get())' | evalrs
    Updating registry `https://github.com/rust-lang/crates.io-index`
   Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.4QzdqRG5cY0x)
    Finished debug [unoptimized + debuginfo] target(s) in 0.24 secs
     Running `target\debug\evalrs_temp.exe`
4 CPUs
```

If you want to use a specific version of an external crate,
you will be able to specify it at a trailing comment of the `extern crate` declaration.

```bash
$ evalrs << EOS
extern crate num_cpus; // "1.2.0"
extern crate some_local_crate; // {path = "/peth/to/some_local_crate"}

println!("{} CPUs", num_cpus::get());
EOS
```

The command wraps input code snippet (except `extern crate` declarations) with a main function.
But, if the code has a line which starts with "fn main()",
it will be passed to `rustc` command without modification.

```bash
# The first execution is equivalent to the second.
$ evalrs << EOS
let a = 1;
let b = 2;
println!("a + b = {}", a + b);
EOS
   Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.gSXTXNaB6o8o)
    Finished debug [unoptimized + debuginfo] target(s) in 0.53 secs
     Running `target/debug/evalrs_temp`
a + b = 3

$ evalrs << EOS
fn main() {
    let a = 1;
    let b = 2;
    println!("a + b = {}", a + b);
}
EOS
   Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.0kYvCRAj0TWI)
    Finished debug [unoptimized + debuginfo] target(s) in 0.20 secs
     Running `target/debug/evalrs_temp`
a + b = 3
```

Emacs Integration
-----------------

As an example of integration with Emacs,
you can use [quickrun](https://github.com/syohex/emacs-quickrun) package
to evaluate Rust code in a buffer by using `evalrs` command.

First, install `quickrun` package as follows:

```lisp
(require 'package)
(add-to-list 'package-archives '("melpa" . "https://melpa.org/packages/") t)
(package-initialize)
(package-refresh-contents)
(package-install 'quickrun)
```

Next, add a quickrun command to execute `evalrs`:

```lisp
(quickrun-add-command
 "evalrs"
 '((:command . "evalrs")
   (:exec . ("cat %s | %c %a")))
 :default "evalrs")
```

Now, you can evaluate Rust code snippet in a buffer quickly:

```rust
extern crate num_cpus;

println!("You have {} CPU cores", num_cpus::get());

// Type following to evaluate this buffer:
//
// M-x quickrun RET evalrs
```
