//! Configuration for searching in files and directories
//!
//!
#![warn(missing_docs)]

use std::fs;
use std::error::Error;
use std::env;

use colored::{ColoredString, Colorize};
use clap::ArgMatches;

pub mod thread_pool;
use thread_pool::ThreadPool;

/// Configuration built from the matched arguments.
#[derive(Clone)]
pub struct Config {
    /// String to look for
    pub query: String,
    /// Optional file/dir to search in (by default ".")
    pub file_path: String,
    /// Current directory when calling minigrep
    pub original_path: String,
    /// Ignore case while looking for matches
    pub ignore_case: bool,
    /// Search in hidden files and directories
    pub hidden_files: bool,
    /// TODO: by default will ignore patterns on a .gitignore.
    /// This option forces the search on these patterns
    pub force_git: bool,
}

impl Config {
    /// Creates a config from the matched arguments on the minigrep call.
    ///
    /// # Panics
    ///
    /// For now, the method to obtain the original_path can fail
    pub fn build(
        matches: ArgMatches,
    ) -> Result<Config, &'static str> {
        let ignore_case = matches.get_flag("ic");

        let hidden_files = matches.get_flag("hidden_files");

        let force_git = matches.get_flag("force_git");

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
            original_path: env::current_dir().unwrap().to_str().unwrap().to_string(),
            query,
            file_path,
            ignore_case,
            hidden_files,
            force_git,
        })
    }
}

/// Searches a **file** with the given configuration.
///
/// # Panics
///
/// The file path in [`Config`] should be a file and not a directory.
///
/// For searching a directory recursively you should use [`run_dir`].
pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&config.file_path)?;

    let results = if config.ignore_case {
        search_case_insensitive(&config.query, &contents)
    } else {
        search(&config.query, &contents)
    };

    if !results.is_empty() {
        let path = std::path::Path::new(&config.file_path);
        let filename = path.to_str().unwrap().strip_prefix(&config.original_path);

        let mut output = format!("{}\n", filename.unwrap().purple());
        for (indices, (line_number, line)) in results.iter() {
            output.push_str(&format!("  {: >3}: ", (line_number+1).to_string().yellow()));

            let chunks = split_by_matches(line, indices.to_owned(), config.query.len());
            for str in chunks.iter() {
                output.push_str(&format!("{str}"));
            }
            output.push('\n');
        }
        println!("{output}");
    }
    
    Ok(())
}

/// Searches a **directory** recursively with the given configuration.
pub fn run_dir(config: &Config, pool: &ThreadPool) -> Result<(), Box<dyn Error>> {
    env::set_current_dir(&config.file_path)?;
    let entries = fs::read_dir(env::current_dir()?)?;

    for entry in entries {
        match entry {
            Err(e) => eprintln!("entry error: {:?}", e),
            Ok(entry) => {
                let path = entry.path();
                let md = fs::metadata(&path)?;

                let mut new_config = config.clone();
                new_config.file_path = match path.to_str() {
                    None => {
                        eprintln!("path error");
                        new_config.file_path
                    }
                    Some(str) => str.to_string(),
                };

                let filename = path.file_name().unwrap().to_str().unwrap();

                if config.hidden_files || !filename.starts_with(".") {
                    if md.is_dir(){
                        run_dir(&new_config, pool)?;
                        env::set_current_dir("../")?;
                    } else {
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

/// Search for query (case sensitive) in contents.
///
/// # Example
///
/// ```rust
/// use minigrep::search;
///
/// let query = "test";
/// let contents = "I'm testing a case sensitive query\nTHIS IS A TesTtestTest";
///
/// assert_eq!(
///     vec![
///         (vec![4], (0, "I'm testing a case sensitive query")),
///         (vec![14], (1, "THIS IS A TesTtestTest"))],
///     search(query, contents));
/// ```
pub fn search<'a>(
    query: &'a str,
    contents: &'a str)
-> Vec<(Vec<usize>, (usize, &'a str))>{
    contents
        .lines()
        .enumerate()
        .map(|(num, line)| {
            let index: Vec<usize> = 
                line
                    .match_indices(query)
                    .map(|(i, _v)| i)
                    .collect();
            (index, (num, line))
        })
        .filter(|(index, (_num, _line))| !index.is_empty())
        .collect()
}

/// Search for query (case insensitive) in contents.
///
/// # Example
///
/// ```rust
/// use minigrep::search_case_insensitive;
///
/// let query = "test";
/// let contents = "I'm teSTing a case sensitestive query\nTHIS IS A TesTtestTest";
///
/// assert_eq!(
///     vec![
///         (vec![4, 24], (0, "I'm teSTing a case sensitestive query")),
///         (vec![10,14,18], (1, "THIS IS A TesTtestTest"))],
///     search_case_insensitive(query, contents));
/// ```
pub fn search_case_insensitive<'a>(
    query: &'a str,
    contents: &'a str)
-> Vec<(Vec<usize>, (usize, &'a str))>{
    contents
        .lines()
        .enumerate()
        .map(|(num, line)| { 
            let index: Vec<usize> = 
                line
                    .to_lowercase()
                    .match_indices(&query.to_lowercase())
                    .map(|(i, _v)| i)
                    .collect();
            (index, (num, line))
        })
        .filter(|(index, (_num, _line))| !index.is_empty())
        .collect()
}

/// Splits a given line by the indices of a matched query
/// and returns the line with the matched colored red.
/// 
/// # Example
///
/// ```rust
/// use colored::Colorize;
/// use minigrep::{ split_by_matches, search_case_insensitive};
///
/// let query = "test";
/// let contents = "I'm teSTing a case sensitestive query\nTHIS IS A TesTtestTest";
///
/// let matches = search_case_insensitive(query, contents);
/// let mut match_iter = matches.iter();
///
/// let (indices, (_num, line)) = match_iter.next().unwrap().to_owned();
/// assert_eq!(
///     vec![
///         "I'm ".normal(),
///         "teST".red(),
///         "ing a case sensi".normal(),
///         "test".red(),
///         "ive query".normal(),
///     ],
///     split_by_matches(line, indices, query.len()));
/// ```
pub fn split_by_matches(
    line: &str,
    indices: Vec<usize>,
    query_len: usize)
-> Vec<ColoredString> {
    let mut output: Vec<ColoredString> = Vec::new();
    let mut match_str = line;
    let mut real_index: usize = 0;

    for index in indices.iter() {
        let current_index = index - real_index;
        real_index += current_index;

        let (pre_match, rest) = match_str.split_at(current_index);

        if !pre_match.is_empty() { output.push(pre_match.normal()); }
        output.push(rest[..query_len].red());

        match_str =  {
                real_index += query_len;
                &rest[query_len..]
        };
    }
    output.push(match_str.normal());

    output
}

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
DUCT TAPE!";

        assert_eq!(vec![(vec![15], (1, "safe, fast, productive."))], search(query, contents));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three, rustrust.
Trust me.";

        assert_eq!(
            vec![
                (vec![0],(0, "Rust:")),
                (vec![12, 16],(2, "Pick three, rustrust.")),
                (vec![1],(3, "Trust me."))
            ],
            search_case_insensitive(query, contents)
        );
    }

    #[test]
    fn split_one_match() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
DUCT TAPE!";

        let res = search(query, contents);
        let (indices, (_num, line)) = res.first().unwrap().to_owned();


        assert_eq!(
            vec!["safe, fast, pro".normal(), "duct".red(), "ive.".normal()],
            split_by_matches(line, indices, query.len()));
    }

    #[test]
    fn split_at_zero() {
        let text = "Rust";
        let (pre, rest) = text.split_at(0);

        assert_eq!("", pre);
        assert_eq!("Rust", rest);
    }

    #[test]
    fn split_multiple_matches() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three, rustrust.
Trust me.";

        let matches = search_case_insensitive(query, contents);
        let mut match_iter = matches.iter();

        match_iter.next();
        let (indices, (_num, line)) = match_iter.next().unwrap().to_owned();
        
        assert_eq!(
            vec!["Pick three, ".normal(), "rust".red(), "rust".red(), ".".normal()],
            split_by_matches(line, indices, query.len())
        )
    }
}








