use stdplus::cli::{ArgumentConfig, Command, OptionConfig};

fn main() {
    let command = Command::new("greet")
        .argument(
            "name",
            ArgumentConfig::default().help("The name of the person to greet"),
        )
        .option(
            "count",
            OptionConfig::default()
                .help("How many times to greet")
                .takes_value(true),
        );

    let args = command.parse();

    let name = args.get("name");
    let count = args.try_get("count").unwrap_or("1");

    let Ok(count) = count.parse::<u8>() else {
        eprintln!("error: invalid 'count' -> {count}");
        std::process::exit(1);
    };

    for _ in 0..count {
        println!("Hello, {name}!");
    }
}
