#[macro_use]
extern crate clap;
extern crate regex;
extern crate tempdir;

use clap::{App, Arg};
use regex::Regex;
use std::borrow::Cow;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process::{self, Command};
use tempdir::TempDir;

fn main() {
    let matches = App::new("evalrs")
        .author(crate_authors!())
        .version(crate_version!())
        .arg(
            Arg::with_name("PRINT_RESULT")
                .short("p")
                .long("print-result")
                .help(r#"Prints the evaluation result using `println!("{:?}", result)`"#),
        )
        .arg(
            Arg::with_name("QUIET")
                .short("q")
                .long("quiet")
                .help("Don't show cargo's build messages if succeeded"),
        )
        .arg(
            Arg::with_name("SNIPPET")
                .index(1)
                .help("Rust code snippet to be evaluated. If this is omitted, the snippet will be read from the standard input."),
        )
        .about("A Rust code snippet evaluator")
        .get_matches();

    let mut options = Options::default();
    if matches.is_present("PRINT_RESULT") {
        options.print_result = true;
    }
    if matches.is_present("QUIET") {
        options.quiet = true;
    }

    let input = if let Some(snippet) = matches.value_of("SNIPPET") {
        snippet.to_owned()
    } else {
        // Reads standard input stream.
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .expect("Cannot read string from standard input stream");
        buf
    };

    // Makes manifest data and source code.
    let manifest = make_manifest(&input);
    let source_code = make_source_code(&input, &options);

    // Sets up temporary project.
    let project_dir = TempDir::new("evalrs_temp").expect("Cannot create temporary directory");
    let cache_dir = env::temp_dir().join("evalrs_cache/");
    {
        // Writes manifest data to `Cargo.toml` file.
        let manifest_file = project_dir.path().join("Cargo.toml");
        let mut manifest_file =
            File::create(manifest_file).expect("Cannot create 'Cargo.toml' file");
        manifest_file
            .write_all(manifest.as_bytes())
            .expect("Cannot write to 'Cargo.toml' file");
    }
    {
        // Writes source code to `src/main.rs` file.
        let src_dir = project_dir.path().join("src/");
        fs::create_dir(src_dir.clone()).expect("Cannot create 'src/' directory");
        let main_file = src_dir.join("main.rs");
        let mut main_file = File::create(main_file).expect("Cannot create 'main.rs' file");
        main_file
            .write_all(source_code.as_bytes())
            .expect("Cannot write to 'main.rs' file");
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
    let mut command = Command::new("cargo");
    command.arg("run");
    if options.quiet {
        command.arg("--quiet");
        //command.stdout(Stdio::null());
    }
    let mut child = command
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
    let re = Regex::new(r"extern\s+crate\s+([a-z0-9_]+)\s*;(\s*//(.+))?").unwrap();
    let dependencies = re.captures_iter(input)
        .map(|cap| {
            if let Some(value) = cap.get(3) {
                if value.as_str().contains("=") {
                    format!("{}\n", value.as_str())
                } else {
                    format!("{} = {}\n", &cap[1], value.as_str())
                }
            } else {
                format!("{} = \"*\"\n", &cap[1])
            }
        })
        .collect::<String>();
    format!(
        r#"
[package]
name = "evalrs_temp"
version = "0.0.0"

[dependencies]
{}
"#,
        dependencies
    )
}

fn make_source_code(input: &str, options: &Options) -> String {
    let re = Regex::new(r"(?m)^# ").unwrap();
    let input = re.replace_all(input, "");

    if Regex::new(r"(?m)^\s*fn +main *\( *\)")
        .unwrap()
        .is_match(&input)
    {
        return input.to_string();
    }
    let re = Regex::new(r"(extern\s+crate\s+[a-z0-9_]+\s*;)").unwrap();
    let crate_lines = re.captures_iter(&input)
        .map(|cap| format!("{}\n", &cap[1]))
        .collect::<String>();
    let mut body = re.replace_all(&input, "");
    if options.print_result {
        body = Cow::from(format!(r#"println!("{{:?}}", {{ {} }});"#, body));
    }
    format!(
        "
{}
fn main() {{
{}
}}",
        crate_lines, body
    )
}

#[derive(Debug, Default)]
struct Options {
    print_result: bool,
    quiet: bool,
}
