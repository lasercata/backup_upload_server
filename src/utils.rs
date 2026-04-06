//! Definition of misc functions

use chrono::Local;
use dotenv::dotenv;

/// Loads the `.env` file's variables if it exists.
pub fn load_dotenv() -> () {
    dotenv().ok(); // error is ignored, variables can be defined in environment
}

/// Checks that the given token is allowed
pub fn verify_token(token: &str) -> bool {
    let token_env: String = std::env::var("token").expect("'token' env var not set");
    token_env == token
}

/// Sanitize the given string (`filename`).
/// It only keeps alphanumeric values, and .-_
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

/// Sanitize the filename and add a timestamp to it.
/// new format (without brackets):
///     YYYY-MM-dd_hh:mm:ss__[in_filename]
pub fn timestamp_filename(in_filename: &str) -> String {
    let sanitized = sanitize_filename(&in_filename);
    let timestamp = Local::now().format("%Y-%m-%d_%H:%M:%S__").to_string();
    let final_filename = format!("{}{}", timestamp, sanitized);

    final_filename
}

