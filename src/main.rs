use regex::Regex;
use std::borrow::Cow;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process::{self, Command};
use tempfile::Builder;

const TMP_PROJECT_NAME: &str = "evalrs_temp";

struct Args {
    snippet: Option<String>,
    print_result: bool,
    quiet: bool,
    release: bool,
}

impl Args {
    fn parse() -> noargs::Result<Option<Self>> {
        let mut args = noargs::raw_args();
        args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
        args.metadata_mut().app_description = "Rust code snippet evaluator";

        if noargs::VERSION_FLAG.take(&mut args).is_present() {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        noargs::HELP_FLAG.take_help(&mut args);

        let print_result = noargs::flag("print-result")
            .short('p')
            .doc("Prints the evaluation result using `println!(\"{:?}\", result)`")
            .take(&mut args)
            .is_present();

        let quiet = noargs::flag("quiet")
            .short('q')
            .doc("Don't show cargo's build messages if succeeded")
            .take(&mut args)
            .is_present();

        let release = noargs::flag("release")
            .doc("Builds artifacts in release mode, with optimizations")
            .take(&mut args)
            .is_present();

        let snippet = noargs::arg("[SNIPPET]")
            .doc(concat!(
                "Rust code snippet to be evaluated. ",
                "If this is omitted, the snippet will be read from the standard input."
            ))
            .take(&mut args)
            .present()
            .map(|a| a.value().to_owned());

        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(None);
        }

        Ok(Some(Self {
            snippet,
            print_result,
            quiet,
            release,
        }))
    }
}

fn main() -> noargs::Result<()> {
    let Some(args) = Args::parse()? else {
        return Ok(());
    };

    let input = if let Some(snippet) = args.snippet.clone() {
        snippet
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
    let source_code = make_source_code(&input, &args);

    // Sets up temporary project.
    let project_dir = Builder::new()
        .prefix(TMP_PROJECT_NAME)
        .tempdir()
        .expect("Cannot create temporary directory");
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

    // Build command
    let mut command = Command::new("cargo");
    command.arg("build");
    if args.quiet {
        command.arg("--quiet");
    }
    if args.release {
        command.arg("--release");
    }
    let mut exit_status = command
        .current_dir(project_dir.path())
        .spawn()
        .expect("Cannot execute 'cargo build'")
        .wait()
        .expect("Cannot wait cargo process");

    // Execute the built command, done separately from building command
    // to ensure execution in the working directory.
    if exit_status.success() {
        let path = project_dir
            .path()
            .join("target")
            .join(if args.release { "release" } else { "debug" })
            .join(TMP_PROJECT_NAME);
        // At this point the previous exit status was zero, so we're only
        // interested in the new exit status that could potentially be
        // nonzero.
        exit_status = Command::new(path)
            .spawn()
            .expect("Cannot execute the built command")
            .wait()
            .expect("Cannot wait built process");
    }

    // Moves 'target/' to cache directory
    {
        let target_dir = project_dir.path().join("target/");
        let cache_target_dir = cache_dir.join("target/");
        if !cache_target_dir.exists() {
            fs::rename(target_dir, cache_target_dir)
                .expect("Cannot move 'target/' to cache directory");
        }
    }

    exit_on_fail(exit_status);

    Ok(())
}

/**
Exit immediately if the `ExitStatus` from the child process wasn't
nonzero, propagating the exit code if it exists.
*/
fn exit_on_fail(exs: process::ExitStatus) {
    if !exs.success() {
        match exs.code() {
            Some(code) => process::exit(code),
            None => {
                eprintln!("Failed to get exit status code");
                process::exit(1);
            }
        }
    }
}

fn make_manifest(input: &str) -> String {
    let re = Regex::new(r"extern\s+crate\s+([a-z0-9_]+)\s*;(\s*//(.+))?").unwrap();
    let dependencies = re
        .captures_iter(input)
        .map(|cap| {
            if let Some(value) = cap.get(3) {
                if value.as_str().contains('=') {
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
name = "{}"
version = "0.0.0"

[dependencies]
{}
"#,
        TMP_PROJECT_NAME, dependencies
    )
}

fn make_source_code(input: &str, args: &Args) -> String {
    let re = Regex::new(r"(?m)^# ").unwrap();
    let input = re.replace_all(input, "");

    if Regex::new(r"(?m)^\s*fn +main *\( *\)")
        .unwrap()
        .is_match(&input)
    {
        return input.to_string();
    }
    let re = Regex::new(r"(extern\s+crate\s+[a-z0-9_]+\s*;)").unwrap();
    let crate_lines = re
        .captures_iter(&input)
        .map(|cap| format!("{}\n", &cap[1]))
        .collect::<String>();
    let mut body = re.replace_all(&input, "");
    if args.print_result {
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
