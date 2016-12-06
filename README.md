evalrs
======

[![Crates.io: evalrs](http://meritbadge.herokuapp.com/evalrs)](https://crates.io/crates/evalrs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust code snippet evaluator.

This tiny command evaluates Rust source code which read form the standard input stream.

It can handle `extern crate` declaration and has simple caching mechanism.

[Documentation](https://docs.rs/evalrs)

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

```bash
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

println!("You have {} CPU cores", num_cpus.get());

// Type following to evaluate this buffer:
//
// M-x quickrun RET evalrs
```
