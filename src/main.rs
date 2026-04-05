use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, middleware};
use actix_multipart::{Field, Multipart};
use futures::StreamExt;
use std::fs;
use std::path::PathBuf;
use chrono::Local;
use dotenv::dotenv;

const UPLOAD_FOLDER: &str = "uploads";

/// Loads the `.env` file's variables
fn load_dotenv() -> () {
    // dotenv().expect("Failed to read .env file");
    dotenv().ok();
}

/// Checks that the given token is allowed
fn verify_token(token: &str) -> bool {
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
/// new format:
///     YYYY-MM-dd_hh:mm:ss__filename
fn timestamp_filename(in_filename: &str) -> String {
    let sanitized = sanitize_filename(&in_filename);
    let timestamp = Local::now().format("%Y-%m-%d_%H:%M:%S__").to_string();
    let final_filename = format!("{}{}", timestamp, sanitized);

    final_filename
}

#[derive(Debug)]
enum WriteError {
    CannotCreateFile(String),
    CannotWriteFile(String),
    FileTooLarge(String),
}

async fn write_file(filepath: PathBuf, field: Field) -> Result<(), WriteError> {
    // Create file
    let mut file = match std::fs::File::create(&filepath) {
        Ok(f) => f,
        Err(_) => return Err(WriteError::CannotCreateFile("Failed to create file".to_string())),
    };

    // Prepare to write
    let mut size: u64 = 0;
    let mut field_data = field;

    // Get max size from .env, and convert to bytes
    let max_file_size_gb: u64 = std::env::var("max_file_size_GB")
        .expect("'max_file_size_GB' env var not set")
        .parse()
        .expect("'max_file_size_GB' was not an u64");
    let max_file_size: u64 = max_file_size_gb * (1024 * 1024 * 1024);

    // Write
    while let Some(Ok(chunk)) = field_data.next().await {
        size += chunk.len() as u64;

        if size > max_file_size {
            let _ = fs::remove_file(&filepath);
            return Err(WriteError::FileTooLarge("File too large".to_string()));
        }

        use std::io::Write;
        if let Err(_) = file.write_all(&chunk) {
            let _ = fs::remove_file(&filepath);
            return Err(WriteError::CannotWriteFile("Failed to write file".to_string()));
        }
    }

    Ok(())
}

/// Removes files so that there is no more than `nb_to_keep` files in `UPLOAD_FOLDER`.
/// It removes first the oldest files (based on metadata modified date)
fn cleanup_old_files(nb_to_keep: usize) -> std::io::Result<()> {
    // Create a vector of (path, modif_time)
    let mut files: Vec<_> = fs::read_dir(UPLOAD_FOLDER)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                let metadata = entry.metadata().ok()?;
                Some((path, metadata.modified().ok()?))
            } else {
                None
            }
        })
        .collect();

    // Sort by modification time, newest first
    files.sort_by(|a, b| b.1.cmp(&a.1));

    // Delete files beyond `nb_to_keep`
    for (path, _) in files.iter().skip(nb_to_keep) {
        fs::remove_file(path).ok();
        println!("Deleted old file: {:?}", path.file_name());
    }

    Ok(())
}

/// Endpoint to upload a file.
async fn upload_file(
    req: HttpRequest,
    mut payload: Multipart,
) -> HttpResponse {
    // Verify token
    let token = match req.headers().get("Authorization") {
        Some(header) => match header.to_str() {
            Ok(t) => t,
            Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid token format"})),
        },
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"})),
    };

    if !verify_token(token) {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}));
    }

    // Process multipart form
    while let Some(Ok(field)) = payload.next().await {
        let content_disposition = field.content_disposition();

        // Only process fields named "file"
        if content_disposition.get_name() != Some("file") {
            continue;
        }

        // Get and process filename
        let filename = match content_disposition.get_filename() {
            Some(name) => name.to_string(),
            None => return HttpResponse::BadRequest().json(serde_json::json!({"error": "No filename provided"})),
        };

        if filename.is_empty() {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Empty filename"}));
        }

        let final_filename = timestamp_filename(&filename);
        let filepath = PathBuf::from(UPLOAD_FOLDER).join(&final_filename);

        // Write file with size limit enforcement
        match write_file(filepath, field).await {
            Err(WriteError::CannotCreateFile(msg)) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": msg})),
            Err(WriteError::CannotWriteFile(msg)) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": msg})),
            Err(WriteError::FileTooLarge(msg)) => return HttpResponse::PayloadTooLarge().json(serde_json::json!({"error": msg})),
            _ => ()
        }

        let max_files: usize = std::env::var("max_files").expect("'max_files' env var not set").parse().expect("'max_files' was not an usize");

        // Cleanup old files
        if let Err(e) = cleanup_old_files(max_files) {
            eprintln!("Cleanup error: {}", e);
        }

        println!("File uploaded: {}", final_filename);

        return HttpResponse::Ok().json(serde_json::json!({
            "message": "File uploaded successfully",
            "filename": final_filename
        }));
    }

    HttpResponse::BadRequest().json(serde_json::json!({"error": "No file provided"}))
}

/// Endpoint that always reply OK (status check)
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    load_dotenv();

    // Create upload directory
    fs::create_dir_all(UPLOAD_FOLDER).ok();

    println!("Starting server on http://0.0.0.0:8080");

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(
                web::scope("")
                    .route("/upload", web::post().to(upload_file))
                    .route("/health", web::get().to(health_check))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

