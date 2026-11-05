use core::{
    fmt::{Debug, Display},
    iter::Peekable,
};
use std::ffi::{OsStr, OsString};

#[derive(Debug, Clone)]
pub struct Command {
    name: String,
    help: Option<String>,
    args: Vec<ArgumentSpec>,
    opts: Vec<OptionSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct ArgumentConfig {
    help: Option<String>,
    nargs: Nargs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Nargs {
    #[default]
    Single,
    Exact(usize),
    Any,
}

#[derive(Debug, Clone, Default)]
pub struct OptionConfig {
    help: Option<String>,
    short: Option<char>,
    takes_value: bool,
}

#[derive(Debug, Clone)]
struct ArgumentSpec {
    name: String,
    config: ArgumentConfig,
}

#[derive(Debug, Clone)]
struct OptionSpec {
    name: String,
    config: OptionConfig,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Args {
    values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
struct Value {
    kind: ValueKind,
    name: String,
    values: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq)]
enum ValueKind {
    Option,
    Positional,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Args(Args),
    Help(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidOptionEncoding,
    UnknownArgument(OsString),
    UnknownOption(String),
    MissingArgument(String),
    MissingOptionValue(String),
    UnexpectedOptionValue(String),
    InvalidArgumentCount(String),
}

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            help: None,
            args: Vec::new(),
            opts: vec![OptionSpec {
                name: "help".into(),
                config: OptionConfig {
                    help: Some("Print help".into()),
                    short: Some('h'),
                    takes_value: false,
                },
            }],
        }
    }

    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }

    /// # Panics
    ///
    /// On configuration mistakes: a duplicate argument name,
    /// or an argument added after a `Nargs::Limitless` one (it could never be filled).
    pub fn argument(mut self, name: impl Into<String>, config: ArgumentConfig) -> Self {
        let name = name.into();

        assert!(
            self.args.iter().all(|a| a.name != name),
            "duplicate argument name '{name}'"
        );
        assert!(
            self.args.last().map(|a| a.config.nargs) != Some(Nargs::Any),
            "argument '{name}' follows an 'any' argument and could never be filled"
        );

        self.args.push(ArgumentSpec { name, config });
        self
    }

    /// # Panics
    ///
    /// On a duplicate long name or short character (including the built-in `--help` / `-h`).
    pub fn option(mut self, name: impl Into<String>, config: OptionConfig) -> Self {
        let name = name.into();

        assert!(
            self.opts.iter().all(|o| o.name != name),
            "duplicate option name '--{name}'"
        );

        if let Some(short) = config.short {
            assert!(
                self.opts.iter().all(|o| o.config.short != Some(short)),
                "duplicate short option '-{short}'"
            );
        }

        self.opts.push(OptionSpec { name, config });
        self
    }

    /// Collects and parses input from `std::env::args()`.
    /// It skips the first item as it assumes this is the program name.
    ///
    /// # Errors
    ///
    /// It returns an error if the input does not conform to the command.
    /// See `ParseError` for more info.
    pub fn parse(&self) -> Args {
        self.parse_args(std::env::args_os().skip(1))
    }

    /// Parses exact input.
    /// It assumes the first item is _not_ program name but the first actual argument or option.
    ///
    /// # Errors
    ///
    /// It returns an error if the input does not conform to the command.
    /// See `ParseError` for more info.
    pub fn parse_args<I, T>(&self, args: I) -> Args
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        match self.get_matches(args) {
            Ok(Outcome::Args(args)) => args,
            Ok(Outcome::Help(help)) => {
                eprintln!("{help}");
                std::process::exit(0);
            }
            Err(err) => err.exit(),
        }
    }

    /// # Errors
    ///
    /// This method errors if the cli args did not conform to the command.
    pub fn get_matches<I, T>(&self, args: I) -> Result<Outcome, ParseError>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let tokens = Tokenizer {
            args: args.into_iter().map(Into::into),
            terminated: false,
        };
        Parser::new(self, tokens).parse()
    }

    fn find_long(&self, name: &str) -> Option<&OptionSpec> {
        self.opts.iter().find(|o| o.name == name)
    }

    fn find_short(&self, c: char) -> Option<&OptionSpec> {
        self.opts.iter().find(|o| o.config.short == Some(c))
    }

    pub fn usage(&self) -> String {
        let mut sections = Vec::new();

        if let Some(help) = &self.help {
            sections.push(help.clone());
        }

        sections.push(self.usage_line());

        if !self.args.is_empty() {
            sections.push(self.arguments_section());
        }
        if !self.opts.is_empty() {
            sections.push(self.options_section());
        }

        let mut out = sections.join("\n\n");
        out.push('\n');
        out
    }

    fn usage_line(&self) -> String {
        let mut line = format!("Usage: {}", self.name);

        if !self.opts.is_empty() {
            line.push_str(" [OPTIONS]");
        }

        for argument in &self.args {
            let name = format!("<{}>", argument.name);
            match argument.config.nargs {
                Nargs::Single => {
                    line.push(' ');
                    line.push_str(&name);
                }
                Nargs::Exact(count) => {
                    for _ in 0..count {
                        line.push(' ');
                        line.push_str(&name);
                    }
                }
                Nargs::Any => {
                    line.push(' ');
                    line.push_str(&name);
                    line.push_str("...");
                }
            }
        }

        line
    }

    fn arguments_section(&self) -> String {
        let rows: Vec<(String, Option<&str>)> = self
            .args
            .iter()
            .map(|a| (format!("<{}>", a.name), a.config.help.as_deref()))
            .collect();

        format!("Arguments:\n{}", render_rows(&rows))
    }

    fn options_section(&self) -> String {
        let rows: Vec<(String, Option<&str>)> = self
            .opts
            .iter()
            .map(|o| {
                let label = match o.config.short {
                    Some(short) => format!("-{short}, --{}", o.name),
                    None => format!("    --{}", o.name),
                };
                (label, o.config.help.as_deref())
            })
            .collect();

        format!("Options:\n{}", render_rows(&rows))
    }
}

/// Renders aligned `  label  help` rows (no trailing newline). The column width
/// is computed from the exact strings being printed.
fn render_rows(rows: &[(String, Option<&str>)]) -> String {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);

    rows.iter()
        .map(|(label, help)| match help {
            Some(help) => format!("  {label:<width$}  {help}"),
            None => format!("  {label}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    /// `--name` or `--name=value`.
    Long {
        name: String,
        value: Option<OsString>,
    },
    /// `-abc`: everything after the dash. Whether this is a cluster of flags or
    /// `-ovalue` depends on the option specs, so the parser decides.
    Short(String),
    /// A plain value. Also produced for `-` and for anything after `--`.
    Positional(OsString),
}

struct Tokenizer<I: Iterator<Item = OsString>> {
    args: I,
    terminated: bool,
}

impl<I: Iterator<Item = OsString>> Iterator for Tokenizer<I> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        for arg in self.args.by_ref() {
            if self.terminated {
                return Some(Ok(Token::Positional(arg)));
            }

            let bytes = arg.as_encoded_bytes();

            if bytes == b"--" {
                self.terminated = true;
                continue;
            }

            if let Some(long) = bytes.strip_prefix(b"--") {
                let (name, value) = match bytes_split_once(long, b'=') {
                    Some((name, value)) => (name, Some(value)),
                    None => (long, None),
                };

                let name = match core::str::from_utf8(name) {
                    Ok(name) => name.to_owned(),
                    Err(_) => return Some(Err(ParseError::InvalidOptionEncoding)),
                };

                let value = match value.map(str::from_utf8) {
                    Some(Ok(s)) => Some(OsString::from(s)),
                    Some(Err(_)) => return Some(Err(ParseError::InvalidOptionEncoding)),
                    None => None,
                };

                return Some(Ok(Token::Long { name, value }));
            }

            if bytes.len() > 1 && bytes.starts_with(b"-") {
                let short = match core::str::from_utf8(&bytes[1..]) {
                    Ok(short) => short.to_owned(),
                    Err(_) => return Some(Err(ParseError::InvalidOptionEncoding)),
                };

                return Some(Ok(Token::Short(short)));
            }

            return Some(Ok(Token::Positional(arg)));
        }

        None
    }
}

fn bytes_split_once(bytes: &[u8], by: u8) -> Option<(&[u8], &[u8])> {
    let mut parts = bytes.split(|byte| *byte == by);
    Some((parts.next()?, parts.next()?))
}

struct Parser<'a, I: Iterator<Item = OsString>> {
    cmd: &'a Command,
    tokens: Peekable<Tokenizer<I>>,
    args: Args,
}

impl<'a, I: Iterator<Item = OsString>> Parser<'a, I> {
    fn new(cmd: &'a Command, tokenizer: Tokenizer<I>) -> Self {
        Self {
            cmd,
            tokens: tokenizer.peekable(),
            args: Args::default(),
        }
    }

    fn parse(mut self) -> Result<Outcome, ParseError> {
        while let Some(token) = self.tokens.next() {
            match token? {
                Token::Positional(value) => self.positional(value)?,
                Token::Long { name, value } => self.long(&name, value)?,
                Token::Short(cluster) => self.short(&cluster)?,
            }

            if self.args.flag("help") {
                return Ok(Outcome::Help(self.cmd.usage()));
            }
        }

        for spec in &self.cmd.args {
            if self.args.value(&spec.name).is_none() {
                return Err(ParseError::MissingArgument(spec.name.clone()));
            }
        }

        Ok(Outcome::Args(self.args))
    }

    /// Consumes the next token only if it is a plain value.
    fn next_positional(&mut self) -> Option<Result<OsString, ParseError>> {
        match self
            .tokens
            .next_if(|token| matches!(token, Ok(Token::Positional(_)) | Err(_)))
        {
            Some(Ok(Token::Positional(value))) => Some(Ok(value)),
            Some(Err(err)) => Some(Err(err)),
            _ => None,
        }
    }

    fn positional(&mut self, first: OsString) -> Result<(), ParseError> {
        let spec = self
            .cmd
            .args
            .iter()
            .find(|spec| self.args.value(&spec.name).is_none())
            .ok_or_else(|| ParseError::UnknownArgument(first.clone()))?;

        let wanted = match spec.config.nargs {
            Nargs::Single => 1,
            Nargs::Exact(count) => count,
            Nargs::Any => usize::MAX,
        };

        let mut values = vec![first];
        while values.len() < wanted {
            match self.next_positional() {
                Some(value) => values.push(value?),
                None => break,
            }
        }

        if matches!(spec.config.nargs, Nargs::Exact(count) if values.len() != count) {
            return Err(ParseError::InvalidArgumentCount(spec.name.clone()));
        }

        self.args.values.push(Value {
            kind: ValueKind::Positional,
            name: spec.name.clone(),
            values,
        });
        Ok(())
    }

    fn long(&mut self, name: &str, inline: Option<OsString>) -> Result<(), ParseError> {
        let spec = self
            .cmd
            .find_long(name)
            .ok_or_else(|| ParseError::UnknownOption(format!("--{name}")))?;

        self.apply(spec, inline)
    }

    /// Handles `-abc` (flag cluster) and `-ovalue` (value attached to the last option).
    fn short(&mut self, cluster: &str) -> Result<(), ParseError> {
        for (index, c) in cluster.char_indices() {
            let spec = self
                .cmd
                .find_short(c)
                .ok_or_else(|| ParseError::UnknownOption(format!("-{c}")))?;

            if spec.config.takes_value {
                let rest = &cluster[index + c.len_utf8()..];
                let inline = (!rest.is_empty()).then(|| OsString::from(rest));
                return self.apply(spec, inline);
            }

            self.apply(spec, None)?;
        }

        Ok(())
    }

    fn apply(&mut self, spec: &OptionSpec, inline: Option<OsString>) -> Result<(), ParseError> {
        let value = if spec.config.takes_value {
            let value = match inline {
                Some(value) => value,
                None => self
                    .next_positional()
                    .ok_or_else(|| ParseError::MissingOptionValue(spec.name.clone()))??,
            };
            Some(value)
        } else {
            if inline.is_some() {
                return Err(ParseError::UnexpectedOptionValue(spec.name.clone()));
            }
            None
        };

        match self.args.values.iter_mut().find(|v| v.name == spec.name) {
            Some(v) => {
                if let Some(value) = value {
                    v.values.push(value);
                }
            }
            None => {
                self.args.values.push(Value {
                    kind: ValueKind::Option,
                    name: spec.name.clone(),
                    values: match value {
                        Some(value) => vec![value],
                        None => Vec::new(),
                    },
                });
            }
        }

        Ok(())
    }
}

impl ArgumentConfig {
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }

    pub fn any(mut self) -> Self {
        self.nargs = Nargs::Any;
        self
    }

    /// # Panics
    ///
    /// This method panics if n == 0.
    pub fn exact(mut self, n: usize) -> Self {
        assert!(n > 0, "exact argument count cannot be zero");
        self.nargs = Nargs::Exact(n);
        self
    }

    pub fn single(mut self) -> Self {
        self.nargs = Nargs::Single;
        self
    }
}

impl OptionConfig {
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }

    pub fn short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    pub fn takes_value(mut self, takes_value: bool) -> Self {
        self.takes_value = takes_value;
        self
    }
}

impl Args {
    fn value(&self, name: &str) -> Option<&Value> {
        self.values.iter().find(|v| v.name == name)
    }

    /// # Panics
    ///
    /// This method panics if more than 1 value was provided.
    pub fn try_get_raw(&self, name: &str) -> Option<&OsStr> {
        match self.value(name)?.values.as_slice() {
            [] => None,
            [one] => Some(one),
            _ => panic!("'{name}' has multiple values"),
        }
    }

    /// # Panics
    ///
    /// This method panics if no values were provided.
    pub fn try_get(&self, name: &str) -> Option<&str> {
        let raw = self.try_get_raw(name)?;
        match raw.to_str() {
            Some(s) => Some(s),
            None => panic!("'{name}'s value is not valid utf8"),
        }
    }

    /// # Panics
    ///
    /// This method panics if no value was provided or if the value is not validi utf8.
    pub fn get(&self, name: &str) -> &str {
        match self.try_get(name) {
            Some(value) => value,
            None => panic!("'{name}' was not provided"),
        }
    }

    pub fn try_get_many_raw(&self, name: &str) -> Option<&[OsString]> {
        Some(&self.value(name)?.values)
    }

    pub fn try_get_many(&self, name: &str) -> Option<Vec<&str>> {
        self.try_get_many_raw(name)?
            .iter()
            .map(|raw| raw.to_str())
            .collect()
    }

    /// # Panics
    ///
    /// This method panics if no values were provided.
    pub fn get_many_raw(&self, name: &str) -> &[OsString] {
        match self.try_get_many_raw(name) {
            Some(many) => many,
            None => panic!("'{name}' received no values"),
        }
    }

    /// # Panics
    ///
    /// This method panics if the values were not provided
    /// or if any of the values is not valid utf8.
    pub fn get_many(&self, name: &str) -> Vec<&str> {
        match self
            .get_many_raw(name)
            .iter()
            .map(|raw| raw.to_str())
            .collect::<Option<Vec<_>>>()
        {
            Some(many) => many,
            None => panic!("'{name}' received non-utf8 values"),
        }
    }

    /// # Panics
    ///
    /// This method panics if `name` is not an option.
    pub fn flag(&self, name: &str) -> bool {
        match self.value(name) {
            Some(value) if value.kind == ValueKind::Option => true,
            Some(_) => panic!("'{name}' is not a flag option"),
            None => false,
        }
    }
}

fn exit_with_error<T: Display>(err: T) -> ! {
    eprintln!("error: {err}");
    eprintln!("Try '--help' for more information.");
    #[expect(clippy::exit, reason = "for api convenience we allow exiting")]
    std::process::exit(1);
}

impl ParseError {
    pub fn exit(&self) -> ! {
        exit_with_error(self)
    }
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::InvalidOptionEncoding => write!(f, "invalid option encoding"),
            ParseError::UnknownArgument(arg) => write!(f, "unknown argument: {}", arg.display()),
            ParseError::UnknownOption(arg) => write!(f, "unknown option: {arg}"),
            ParseError::MissingArgument(name) => write!(f, "missing required argument: {name}"),
            ParseError::MissingOptionValue(name) => write!(f, "option '--{name}' requires a value"),
            ParseError::UnexpectedOptionValue(name) => {
                write!(f, "option '--{name}' does not take a value")
            }
            ParseError::InvalidArgumentCount(name) => {
                write!(f, "invalid number of values for argument '{name}'")
            }
        }
    }
}

impl core::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd() -> Command {
        Command::new("prog")
            .help("A test program")
            .argument("input", ArgumentConfig::default().help("Input file"))
            .argument("rest", ArgumentConfig::default().any())
            .option("verbose", OptionConfig::default().short('v'))
            .option("quiet", OptionConfig::default().short('q'))
            .option("out", OptionConfig::default().short('o').takes_value(true))
    }

    fn parse(args: &[&str]) -> Result<Args, ParseError> {
        match cmd().get_matches(args.iter().copied())? {
            Outcome::Args(args) => Ok(args),
            Outcome::Help(_) => panic!("unexpected help"),
        }
    }

    #[test]
    fn positionals_fill_in_order() {
        let args = parse(&["a", "b", "c"]).unwrap();
        assert_eq!(args.try_get("input").unwrap(), "a");
        assert_eq!(args.try_get_many("rest").unwrap(), ["b", "c"]);
    }

    #[test]
    fn long_and_short_values() {
        let args = parse(&["a", "b", "--out", "x", "-o", "y", "--out=z", "-ow"]).unwrap();
        assert_eq!(args.try_get_many("out").unwrap(), &["x", "y", "z", "w"]);
    }

    #[test]
    fn short_cluster() {
        let args = parse(&["a", "b", "-vqo", "file"]).unwrap();
        assert!(args.flag("verbose") && args.flag("quiet"));
        assert_eq!(args.try_get("out"), Some("file"));
    }

    #[test]
    fn terminator_and_dash() {
        let args = parse(&["a", "--", "-x", "--y", "-"]).unwrap();
        assert_eq!(args.try_get_many("rest").unwrap(), ["-x", "--y", "-"]);
    }

    #[test]
    fn help_short_circuits() {
        let out = cmd().get_matches(["a", "--help"]).unwrap();
        assert!(matches!(out, Outcome::Help(_)));
        let out = cmd().get_matches(["-h"]).unwrap();
        assert!(matches!(out, Outcome::Help(_)));
    }

    #[test]
    fn errors() {
        assert_eq!(parse(&[]), Err(ParseError::MissingArgument("input".into())));
        assert_eq!(
            parse(&["a", "b", "--nope"]),
            Err(ParseError::UnknownOption("--nope".into()))
        );
        assert_eq!(
            parse(&["a", "b", "-x"]),
            Err(ParseError::UnknownOption("-x".into()))
        );
        assert_eq!(
            parse(&["a", "b", "--out"]),
            Err(ParseError::MissingOptionValue("out".into()))
        );
        assert_eq!(
            parse(&["a", "b", "--verbose=1"]),
            Err(ParseError::UnexpectedOptionValue("verbose".into()))
        );
    }

    #[test]
    fn multiple_count() {
        let cmd = Command::new("p").argument("pair", ArgumentConfig::default().exact(2));
        cmd.get_matches(["a", "b"]).unwrap();
        assert_eq!(
            cmd.get_matches(["a"]),
            Err(ParseError::InvalidArgumentCount("pair".into()))
        );
    }

    #[test]
    fn usage_is_aligned() {
        let usage = cmd().usage();
        let help = usage.lines().find(|l| l.contains("Print help")).unwrap();
        let out = usage.lines().find(|l| l.contains("--out")).unwrap();
        let verbose = usage.lines().find(|l| l.contains("--verbose")).unwrap();
        assert!(out.starts_with("  -o, --out"));
        assert!(verbose.starts_with("  -v, --verbose"));
        // Every option row pads its label to the same column.
        let col = |line: &str| line.len() - line.trim_start().len();
        assert_eq!(col(help), col(out));
        assert_eq!(col(out), col(verbose));
    }

    #[test]
    #[should_panic(expected = "duplicate short option")]
    fn duplicate_short_panics() {
        let _: Command = Command::new("p").option("hello", OptionConfig::default().short('h'));
    }
}
