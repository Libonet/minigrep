use std::fs;
use std::error::Error;
use std::env;

use clap::ArgMatches;

mod thread_pool;

#[derive(Clone)]
pub struct Config {
    pub query: String,
    pub file_path: String,
    pub ignore_case: bool,
}

impl Config {
    pub fn build(
        matches: ArgMatches,
    ) -> Result<Config, &'static str> {
        let ignore_case = matches.get_flag("ic");

        let query = match matches.get_one::<String>("query") {
            Some(arg) => arg.to_string(),
            None => unreachable!("clap should check this"),
        };

        // if there's no file path, search in whole directory
        let file_path = match matches.get_one::<String>("path") {
            Some(arg) => arg.to_string(),
            None => unreachable!("default value is '.'"),
        };

        Ok(Config { 
            query,
            file_path,
            ignore_case
        })
    }
}

pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&config.file_path)?;

    let results = if config.ignore_case {
        search_case_insensitive(&config.query, &contents)
    } else {
        search(&config.query, &contents)
    };

    if !results.is_empty() {
        let path = std::path::Path::new(&config.file_path);
        let filename = path.file_name().unwrap();

        let mut output = format!("in the file: {}\n", filename.to_str().unwrap());
        for line in results {
            output.push_str(&format!("  {line}\n"));
        }

        println!("{output}");
    }
    
    Ok(())
}

pub fn run_dir(config: &Config) -> Result<(), Box<dyn Error>> {
    env::set_current_dir(&config.file_path)?;
    let entries = fs::read_dir(env::current_dir()?)?;

    let pool = thread_pool::ThreadPool::new(4);

    for entry in entries {
        match entry {
            Err(e) => eprintln!("entry error: {:?}", e),
            Ok(entry) => {
                let path = entry.path();
                let md = fs::metadata(&path).unwrap();
                if md.is_dir() {
                    let config_copy = config.clone();
                    pool.execute(move || {
                        let _ = env::set_current_dir(path);
                        let _ = run_dir(&config_copy);
                    });
                } else {
                    let mut new_config = config.clone();
                    new_config.file_path = match path.to_str() {
                        None => {
                            eprintln!("path error");
                            new_config.file_path
                        }
                        Some(str) => str.to_string(),
                    };
                    if !new_config.file_path.starts_with('.') {
                        pool.execute(move || {
                            let _ = run(&new_config);
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str>{
    contents
        .lines()
        .filter(|line| line.contains(query))
        .collect()
}

pub fn search_case_insensitive<'a>(
    query: &str,
    contents: &'a str)
-> Vec<&'a str>{
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query.to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
DUCT TAPE!";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

        assert_eq!(
            vec!["Rust:", "Trust me."],
            search_case_insensitive(query, contents)
        );
    }
}

