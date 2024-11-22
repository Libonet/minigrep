use std::env;
use std::process;

use clap::crate_version;
use clap::{Arg, Command};

use git2::Repository;
use minigrep::{
    Config,
    thread_pool::ThreadPool,
};

fn main() {
    let cmd = Command::new("minigrep")
        .about("A little copy of ripgrep")
        .version(crate_version!())
        .arg(
            Arg::new("ic")
            .short('i')
            .long("ignore_case")
            .help("Searches for any match ignoring case")
            .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("hidden_files")
            .long("hidden")
            .help("Looks in hidden files and directories")
            .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("force_git")
            .long("force_git")
            .short('g')
            .help("Looks inside directories and files found in a .gitignore")
            .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("thread_count")
            .long("thread_count")
            .short('t')
            .help("Amount of threads to use. 6 is the default")
            .default_value("6")
        )
        .arg(
            Arg::new("query")
            .help("The string to search for matches")
            .required(true)
        )
        .arg(
            Arg::new("path")
            .help("The path in which to search for the query")
            .default_value(".")
        );

    let matches = cmd.get_matches();
    let config = match Config::build(matches) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error building config: {e}"); 
            process::exit(1)
        }
    };

    let md = std::fs::metadata(&config.file_path);
    let md = match md {
        Ok(md) => md,
        Err(e) => {
            eprintln!("Error obtaining metadata: {e}");
            process::exit(2)
        }
    };

    let pool = ThreadPool::new(config.thread_count);

    let ret = 
        if md.is_dir() {
            if config.force_git{ 
                minigrep::run_dir(&config, &pool) 
            } else {
                let git_repo = match Repository::open_from_env() {
                    Ok(repo) => repo,
                    Err(e) => {
                        eprintln!("Error obtaining git repo: {e}");
                        process::exit(2)
                    }
                };
                minigrep::run_dir_with_git(&git_repo, &config, &pool)
            }
        } else {
            minigrep::run(&config)
        };


    match ret {
        Ok(_) => (),
        Err(e) => eprintln!("Application error: {e}"),
    };
}



