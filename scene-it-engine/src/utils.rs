use std::io;

pub fn read_from_console() -> String {
    let mut input = String::new();
    let mut retries = 3;
    loop {
        if let Err(err) = io::stdin().read_line(&mut input) {
            if retries == 0 {
                eprintln!(
                    "We continue to have an issue reading your input. Please restart the application and try again and hopefully this will resolve the issue."
                );
                dbg!(err);
                panic!("Failed to read user input from console");
            }
            eprintln!("Seems there's an error reading your input. Please try again.");
            retries -= 1;
        } else {
            break;
        }
    }
    input.trim().into()
}

pub fn trim_input(input: &str) -> String {
    input
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
