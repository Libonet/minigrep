minigrep is a small copy of [ripgrep](https://github.com/BurntSushi/ripgrep), a line-oriented search tool that recursively searches the current directory for a regex pattern.

In minigrep's case, by default it searches recursively the current directory for the query, but a file can also be provided

## Usage

```bash
minigrep [OPTIONS] <query> [path]

Arguments:
  <query>  The string to search for matches
  [path]   The path in which to search for the query [default: .]

Options:
  -i, --ignore_case  Searches for any match ignoring case
  -h, --help         Print help
  -V, --version      Print version
```
