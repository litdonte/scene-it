#[derive(Debug)]
pub enum InputError {
    EmptyInput,
    TooManyChars,
    ContainsControlChars,
}

pub fn validate_input(input: &str, size_limit: Option<usize>) -> Result<String, InputError> {
    let trimmed = input.split_whitespace().collect::<Vec<_>>().join(" ");

    if trimmed.is_empty() {
        return Err(InputError::EmptyInput);
    }

    if trimmed.chars().any(|c| c.is_control()) {
        return Err(InputError::ContainsControlChars);
    }

    if size_limit.is_some_and(|s| trimmed.chars().count() > s) {
        return Err(InputError::TooManyChars);
    }

    Ok(trimmed)
}
