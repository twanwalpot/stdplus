use stdplus::cli::{ArgumentConfig, Command, OptionConfig, Outcome};

#[test]
fn command_can_be_created() {
    let command = Command::new("test");
    assert!(matches!(
        command.get_matches::<&[String], &String>(&[]),
        Ok(Outcome::Args(_))
    ));
}

#[test]
fn command_accepts_help() {
    let command = Command::new("test").help("Test command");
    assert!(matches!(
        command.get_matches::<&[String], &String>(&[]),
        Ok(Outcome::Args(_))
    ));
}

#[test]
fn command_accepts_arguments() {
    let command = Command::new("test").argument("path", ArgumentConfig::default());

    let args = match command.get_matches(["file.txt"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert_eq!(args.try_get("path"), Some("file.txt"));
}

#[test]
fn command_accepts_multiple_arguments() {
    let command = Command::new("test").argument("paths", ArgumentConfig::default().exact(3));

    let args = match command.get_matches(["one", "two", "three"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert_eq!(
        args.try_get_many("paths"),
        Some(vec!["one", "two", "three"])
    );
}

#[test]
fn command_accepts_limitless_arguments() {
    let command = Command::new("test").argument("paths", ArgumentConfig::default().any());

    let args = match command.get_matches(["one", "two", "three"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert_eq!(
        args.try_get_many("paths"),
        Some(vec!["one", "two", "three"])
    );
}

#[test]
fn command_rejects_missing_arguments() {
    let command = Command::new("test").argument("path", ArgumentConfig::default());

    command.get_matches::<&[String], &String>(&[]).unwrap_err();
}

#[test]
fn command_rejects_invalid_argument_count() {
    let command = Command::new("test").argument("paths", ArgumentConfig::default().exact(3));

    command.get_matches(["one", "two"]).unwrap_err();
}

#[test]
fn command_accepts_options() {
    let command = Command::new("test").option("verbose", OptionConfig::default());

    let args = match command.get_matches(["--verbose"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert!(args.flag("verbose"));
}

#[test]
fn command_accepts_short_options() {
    let command = Command::new("test").option("verbose", OptionConfig::default().short('v'));

    let args = match command.get_matches(["-v"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert!(args.flag("verbose"));
}

#[test]
fn command_accepts_options_with_values() {
    let command = Command::new("test").option("count", OptionConfig::default().takes_value(true));

    let args = match command.get_matches(["--count", "42"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert_eq!(args.try_get("count"), Some("42"));
}

#[test]
fn command_accepts_options_with_inline_values() {
    let command = Command::new("test").option("count", OptionConfig::default().takes_value(true));

    let args = match command.get_matches(["--count=42"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert_eq!(args.try_get("count"), Some("42"));
}

#[test]
fn command_rejects_missing_option_values() {
    let command = Command::new("test").option("count", OptionConfig::default().takes_value(true));

    command.get_matches(["--count"]).unwrap_err();
}

#[test]
fn command_rejects_unexpected_option_values() {
    let command = Command::new("test").option("verbose", OptionConfig::default());

    command.get_matches(["--verbose=true"]).unwrap_err();
}

#[test]
fn command_rejects_unknown_options() {
    let command = Command::new("test");
    command.get_matches(["--unknown"]).unwrap_err();
}

#[test]
fn command_rejects_unknown_short_options() {
    let command = Command::new("test");
    command.get_matches(["-x"]).unwrap_err();
}

#[test]
fn command_rejects_unknown_arguments() {
    let command = Command::new("test");
    command.get_matches(["unknown"]).unwrap_err();
}

#[test]
fn command_accepts_multiple_options() {
    let command = Command::new("test")
        .option("verbose", OptionConfig::default().short('v'))
        .option("count", OptionConfig::default().takes_value(true));

    let args = match command.get_matches(["-v", "--count", "10"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert!(args.flag("verbose"));
    assert_eq!(args.try_get("count"), Some("10"));
}

#[test]
fn command_accepts_options_and_arguments_interleaved() {
    let command = Command::new("test")
        .option("verbose", OptionConfig::default().short('v'))
        .argument("input", ArgumentConfig::default())
        .argument("output", ArgumentConfig::default());

    let args = match command.get_matches(["/foo", "-v", "/bar"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert!(args.flag("verbose"));
    assert_eq!(args.try_get("input").unwrap(), "/foo");
    assert_eq!(args.try_get("output").unwrap(), "/bar");
}

#[test]
fn command_accepts_options_before_arguments() {
    let command = Command::new("test")
        .argument("path", ArgumentConfig::default())
        .option("verbose", OptionConfig::default());

    let args = match command.get_matches(["--verbose", "file.txt"]).unwrap() {
        Outcome::Args(args) => args,
        Outcome::Help(_) => panic!("Expected arguments"),
    };

    assert!(args.flag("verbose"));
    assert_eq!(args.try_get("path").unwrap(), "file.txt");
}

#[test]
fn command_returns_help_for_help_option() {
    let command = Command::new("test").help("Test command");

    match command.get_matches(["--help"]).unwrap() {
        Outcome::Help(help) => {
            assert!(help.contains("Test command"));
            assert!(help.contains("Usage: test"));
        }
        Outcome::Args(_) => panic!("Expected help"),
    }
}

#[test]
fn command_returns_help_for_short_help_option() {
    let command = Command::new("test");

    assert!(matches!(
        command.get_matches(["-h"]).unwrap(),
        Outcome::Help(_)
    ));
}

#[test]
fn help_includes_command_name() {
    let command = Command::new("test");

    assert!(command.usage().contains("Usage: test"));
}

#[test]
fn help_includes_command_description() {
    let command = Command::new("test").help("A test command");

    assert!(command.usage().contains("A test command"));
}

#[test]
fn help_includes_arguments() {
    let command = Command::new("test").argument(
        "path",
        ArgumentConfig::default().help("The path to process"),
    );

    let help = command.usage();

    assert!(help.contains("<path>"));
    assert!(help.contains("The path to process"));
}

#[test]
fn help_includes_options() {
    let command = Command::new("test").option(
        "verbose",
        OptionConfig::default()
            .short('v')
            .help("Enable verbose output"),
    );

    let help = command.usage();

    assert!(help.contains("--verbose"));
    assert!(help.contains("-v"));
    assert!(help.contains("Enable verbose output"));
    assert!(help.contains("--help"));
}

#[test]
fn help_formats_multiple_arguments() {
    let command = Command::new("test").argument("path", ArgumentConfig::default().exact(3));

    let help = command.usage();

    assert!(help.contains("<path> <path> <path>"));
}

#[test]
fn help_formats_limitless_arguments() {
    let command = Command::new("test").argument("path", ArgumentConfig::default().any());

    assert!(command.usage().contains("<path>..."));
}

#[test]
fn help_allows_incorrect_args() {
    let command = Command::new("test").argument("path", ArgumentConfig::default());

    if let Outcome::Args(_) = command.get_matches(["--help"]).unwrap() {
        panic!("must be help")
    }
}
