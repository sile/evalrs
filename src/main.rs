//! A Rust code snippet evaluator.
//!
//! This tiny command evaluates Rust source code which read from the standard input stream.
//!
//! It can handle `extern crate` declaration and has simple caching mechanism.
//!
//! Installation
//! ------------
//!
//! Execute following command on your terminal:
//!
//! ```bash
//! # Installs `evalrs` command
//! $ cargo install evalrs
//!
//! # Shows help message
//! $ evalrs -h
//! ```
//!
//! Usage Examples
//! --------------
//!
//! `evalrs` command reads Rust code snippet from the standard input stream and evaluates it:
//!
//! ```bash
//! $ echo 'println!("Hello World!")' | evalrs
//!    Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.daiPxHtjV2VR)
//!     Finished debug [unoptimized + debuginfo] target(s) in 0.51 secs
//!      Running `target\debug\evalrs_temp.exe`
//! Hello World!
//! ```
//!
//! If target code includes `extern crate` declarations,
//! the latest version of those crates will be downloaded and cached:
//!
//! ```bash
//! # First time
//! $ echo 'extern crate num_cpus; println!("{} CPUs", num_cpus::get())' | evalrs
//!     Updating registry `https://github.com/rust-lang/crates.io-index`
//!    Compiling libc v0.2.18
//!    Compiling num_cpus v1.2.0
//!    Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.HSRNyVQbM6s3)
//!     Finished debug [unoptimized + debuginfo] target(s) in 0.55 secs
//!      Running `target\debug\evalrs_temp.exe`
//! 4 CPUs
//!
//! # Second time
//! $ echo 'extern crate num_cpus; println!("{} CPUs", num_cpus::get())' | evalrs
//!     Updating registry `https://github.com/rust-lang/crates.io-index`
//!    Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.4QzdqRG5cY0x)
//!     Finished debug [unoptimized + debuginfo] target(s) in 0.24 secs
//!      Running `target\debug\evalrs_temp.exe`
//! 4 CPUs
//! ```
//!
//! The command wraps input code snippet (except `extern crate` declarations) with a main function.
//! But, if the code has a line which starts with "fn main()",
//! it will be passed to `rustc` command without modification.
//!
//! ```bash
//! # The first execution is equivalent to the second.
//! $ evalrs << EOS
//! let a = 1;
//! let b = 2;
//! println!("a + b = {}", a + b);
//! EOS
//!    Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.gSXTXNaB6o8o)
//!     Finished debug [unoptimized + debuginfo] target(s) in 0.53 secs
//!      Running `target/debug/evalrs_temp`
//! a + b = 3
//!
//! $ evalrs << EOS
//! fn main() {
//!     let a = 1;
//!     let b = 2;
//!     println!("a + b = {}", a + b);
//! }
//! EOS
//!    Compiling evalrs_temp v0.0.0 (file:///tmp/evalrs_temp.0kYvCRAj0TWI)
//!     Finished debug [unoptimized + debuginfo] target(s) in 0.20 secs
//!      Running `target/debug/evalrs_temp`
//! a + b = 3
//! ```
//!
//! Emacs Integration
//! -----------------
//!
//! As an example of integration with Emacs,
//! you can use [quickrun](https://github.com/syohex/emacs-quickrun) package
//! to evaluate Rust code in a buffer by using `evalrs` command.
//!
//! First, install `quickrun` package as follows:
//!
//! ```lisp
//! (require 'package)
//! (add-to-list 'package-archives '("melpa" . "https://melpa.org/packages/") t)
//! (package-initialize)
//! (package-refresh-contents)
//! (package-install 'quickrun)
//! ```
//!
//! Next, add a quickrun command to execute `evalrs`:
//!
//! ```lisp
//! (quickrun-add-command
//!  "evalrs"
//!  '((:command . "evalrs")
//!    (:exec . ("cat %s | %c %a")))
//!  :default "evalrs")
//! ```
//!
//! Now, you can evaluate Rust code snippet in a buffer quickly:
//!
//! ```no_run
//! extern crate num_cpus;
//!
//! println!("You have {} CPU cores", num_cpus::get());
//!
//! // Type following to evaluate this buffer:
//! //
//! // M-x quickrun RET evalrs
//! ```

#[macro_use]
extern crate clap;
extern crate regex;
extern crate tempdir;

use std::env;
use std::io::{self, Write, Read};
use std::fs::{self, File};
use std::process::{self, Command};
use clap::App;
use tempdir::TempDir;
use regex::Regex;

fn main() {
    let _matches = App::new("evalrs")
        .author(crate_authors!())
        .version(crate_version!())
        .about("A Rust code snippet evaluator")
        .get_matches();

    // Reads standard input stream.
    let input = {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .expect("Cannot read string from standard input stream");
        buf
    };

    // Makes manifest data and source code.
    let manifest = make_manifest(&input);
    let source_code = make_source_code(&input);

    // Sets up temporary project.
    let project_dir = TempDir::new("evalrs_temp").expect("Cannot create temporary directory");
    let cache_dir = env::temp_dir().join("evalrs_cache/");
    {
        // Writes manifest data to `Cargo.toml` file.
        let manifest_file = project_dir.path().join("Cargo.toml");
        let mut manifest_file = File::create(manifest_file)
            .expect("Cannot create 'Cargo.toml' file");
        manifest_file.write_all(manifest.as_bytes()).expect("Cannot write to 'Cargo.toml' file");
    }
    {
        // Writes source code to `src/main.rs` file.
        let src_dir = project_dir.path().join("src/");
        fs::create_dir(src_dir.clone()).expect("Cannot create 'src/' directory");
        let main_file = src_dir.join("main.rs");
        let mut main_file = File::create(main_file).expect("Cannot create 'main.rs' file");
        main_file.write_all(source_code.as_bytes()).expect("Cannot write to 'main.rs' file");
    }
    {
        // Sets up cache data.
        let target_dir = project_dir.path().join("target/");
        let cache_target_dir = cache_dir.join("target/");
        fs::create_dir_all(cache_target_dir.clone())
            .expect("Cannot create cache 'target/' directory");
        fs::rename(cache_target_dir, target_dir)
            .expect("Cannot move 'target/' from cache directory");
    }

    // Builds and executes command
    let mut child = Command::new("cargo")
        .arg("run")
        .current_dir(project_dir.path())
        .spawn()
        .expect("Cannot execute 'cargo run'");
    let exit_status = child.wait().expect("Cannot wait child process");

    // Moves 'target/' to cache directory
    {
        let target_dir = project_dir.path().join("target/");
        let cache_target_dir = cache_dir.join("target/");
        if !cache_target_dir.exists() {
            fs::rename(target_dir, cache_target_dir)
                .expect("Cannot move 'target/' to cache directory");
        }
    }

    if let Some(code) = exit_status.code() {
        process::exit(code);
    }
}

fn make_manifest(input: &str) -> String {
    let re = Regex::new(r"extern\s+crate\s+([a-z0-9_]+)\s*;").unwrap();
    let dependencies = re.captures_iter(input)
        .map(|cap| format!("{} = \"*\"\n", cap.at(1).unwrap()))
        .collect::<String>();
    format!(r#"
[package]
name = "evalrs_temp"
version = "0.0.0"

[dependencies]
{}
"#,
            dependencies)
}

fn make_source_code(input: &str) -> String {
    if Regex::new(r"^fn main\(\)").unwrap().is_match(input) {
        return input.to_string();
    }
    let re = Regex::new(r"(extern\s+crate\s+[a-z0-9_]+\s*;)").unwrap();
    let crate_lines = re.captures_iter(input)
        .map(|cap| format!("{}\n", cap.at(1).unwrap()))
        .collect::<String>();
    let body = re.replace_all(input, "");
    format!("
{}
fn main() {{
{}
}}",
            crate_lines,
            body)
}
