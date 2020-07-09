use getopts;
use std::env;
use std::io;
use std::io::Read;
use serde_json;
use types;
use types::*;

pub enum InputType {
    Json,
    Kakoune,
    Text
}

pub enum OutputType {
    Json,
    Kakoune,
    Text
}

pub struct Options {
    matches: getopts::Matches
}

fn options() -> getopts::Options {
    let mut options = getopts::Options::new();
    options.optflag("h", "help", "show this help message");
    options.optopt("", "input-format", "'json', 'text' (default: 'text')", "FMT");
    options.optopt("m", "mode", "parinfer mode (indent, paren, or smart) (default: smart)", "MODE");
    options.optopt("", "output-format", "'json', 'kakoune', 'text' (default: 'text')", "FMT");
    options.optopt("", "comment-char", "(default: ';')", "CC");
    options
}

pub fn usage() -> String {
    options().usage("Usage: parinfer-rust [options]")
}

struct Defaults {
    lisp_vline_symbols: bool,
    lisp_block_comment: bool,
    scheme_sexp_comment: bool,
    janet_long_strings: bool
}

fn language_defaults(language: Option<&str>) -> Defaults {
    match language {
        Some("clojure") => Defaults {
            lisp_vline_symbols: false,
            lisp_block_comment: false,
            scheme_sexp_comment: false,
            janet_long_strings: false,
        },
        Some("janet") => Defaults {
            lisp_vline_symbols: false,
            lisp_block_comment: false,
            scheme_sexp_comment: false,
            janet_long_strings: true,
        },
        Some("lisp") => Defaults {
            lisp_vline_symbols: true,
            lisp_block_comment: true,
            scheme_sexp_comment: false,
            janet_long_strings: false
        },
        Some("racket") => Defaults {
            lisp_vline_symbols: true,
            lisp_block_comment: true,
            scheme_sexp_comment: true,
            janet_long_strings: false
        },
        Some("scheme") => Defaults {
            lisp_vline_symbols: true,
            lisp_block_comment: true,
            scheme_sexp_comment: true,
            janet_long_strings: false
        },
        None    => language_defaults(Some("clojure")),
        // Unknown language.  Defaults kind of work for most lisps
        Some(_) => language_defaults(Some("clojure")),
    }
}

impl Options {
    pub fn parse(args: &[String]) -> Result<Options, String> {
        options()
            .parse(args)
            .map(|m| Options {matches: m})
            .map_err(|e| e.to_string())
    }

    pub fn want_help(&self) -> bool {
        self.matches.opt_present("h")
    }

    fn mode(&self) -> &'static str {
        match self.matches.opt_str("m") {
            None => "smart",
            Some(ref s) if s == "i" || s == "indent" => "indent",
            Some(ref s) if s == "p" || s == "paren"  => "paren",
            Some(ref s) if s == "s" || s == "smart"  => "smart",
            _ => panic!("invalid mode specified for `-m`")
        }
    }

    fn input_type(&self) -> InputType {
        match self.matches.opt_str("input-format") {
            None => InputType::Text,
            Some(ref s) if s == "text" => InputType::Text,
            Some(ref s) if s == "json" => InputType::Json,
            Some(ref s) if s == "kakoune" => InputType::Kakoune,
            Some(ref s) => panic!("unknown input format `{}`", s)
        }
    }

    pub fn output_type(&self) -> OutputType {
        match self.matches.opt_str("output-format") {
            None => OutputType::Text,
            Some(ref s) if s == "text" => OutputType::Text,
            Some(ref s) if s == "json" => OutputType::Json,
            Some(ref s) if s == "kakoune" => OutputType::Kakoune,
            Some(ref s) => panic!("unknown output fomrat `{}`", s)
        }
    }

    fn comment_char(&self) -> char {
        match self.matches.opt_str("comment-char") {
            None => ';',
            Some(ref s) if s.chars().count() == 1 =>  s.chars().next().unwrap(),
            Some(ref _s) => panic!("comment character must be a single character")
        }
    }

    pub fn request(&self) -> io::Result<Request> {
        match self.input_type() {
            InputType::Text => {
                let mut text = String::new();
                io::stdin().read_to_string(&mut text)?;
                Ok(Request {
                    mode: String::from(self.mode()),
                    text,
                    options: types::Options {
                        changes: vec![],
                        cursor_x: None,
                        cursor_line: None,
                        prev_text: None,
                        prev_cursor_x: None,
                        prev_cursor_line: None,
                        force_balance: false,
                        return_parens: false,
                        comment_char: char::from(self.comment_char()),
                        partial_result: false,
                        selection_start_line: None,
                        lisp_vline_symbols: false,
                        lisp_block_comment: false,
                        scheme_sexp_comment: false,
                        janet_long_strings: false
                    }
                })
            },
            InputType::Kakoune => {
                let Defaults {
                    lisp_vline_symbols,
                    lisp_block_comment,
                    scheme_sexp_comment,
                    janet_long_strings
                } = match env::var("kak_opt_filetype") {
                    Ok(filetype) => language_defaults(Some(&filetype)),
                    Err(_)       => language_defaults(None),
                };
                Ok(Request {
                    mode: String::from(self.mode()),
                    text: env::var("kak_selection").unwrap(),
                    options: types::Options {
                        changes: vec![],
                        cursor_x: env::var("kak_opt_parinfer_cursor_char_column")
                            .map(|s| s.parse::<Column>().unwrap() - 1)
                            .ok(),
                        cursor_line: env::var("kak_opt_parinfer_cursor_line")
                            .map(|s| s.parse::<LineNumber>().unwrap() - 1)
                            .ok(),
                        prev_text: env::var("kak_opt_parinfer_previous_text")
                            .ok(),
                        prev_cursor_x: env::var("kak_opt_parinfer_previous_cursor_char_column")
                            .map(|s| s.parse::<Column>().unwrap() - 1)
                            .ok(),
                        prev_cursor_line: env::var("kak_opt_parinfer_previous_cursor_line")
                            .map(|s| s.parse::<LineNumber>().unwrap() - 1)
                            .ok(),
                        force_balance: false,
                        return_parens: false,
                        comment_char: char::from(self.comment_char()),
                        partial_result: false,
                        selection_start_line: None,
                        lisp_vline_symbols,
                        lisp_block_comment,
                        scheme_sexp_comment,
                        janet_long_strings,
                    }
                })
            },
            InputType::Json => {
                let mut input = String::new();
                io::stdin().read_to_string(&mut input)?;
                Ok(serde_json::from_str(&input)?)
            },
        }
    }

}
