//! Main file that defines the web server using actix

// Imports
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, middleware};
use actix_multipart::Multipart;
use futures::StreamExt;
use std::fs;
use std::path::PathBuf;

mod utils;
mod file_management;

// Init
const UPLOAD_FOLDER: &str = "uploads";
const PORT: u16 = 8080;

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

    if !utils::verify_token(token) {
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

        let final_filename = utils::timestamp_filename(&filename);
        let filepath = PathBuf::from(UPLOAD_FOLDER).join(&final_filename);

        // Write file with size limit enforcement
        match file_management::write_file(filepath, field).await {
            Err(file_management::WriteError::CannotCreateFile(msg)) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": msg})),
            Err(file_management::WriteError::CannotWriteFile(msg)) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": msg})),
            Err(file_management::WriteError::FileTooLarge(msg)) => return HttpResponse::PayloadTooLarge().json(serde_json::json!({"error": msg})),
            _ => ()
        }

        let max_files: usize = std::env::var("max_files").expect("'max_files' env var not set").parse().expect("'max_files' was not an usize");

        // Cleanup old files
        if let Err(e) = file_management::cleanup_old_files(max_files, UPLOAD_FOLDER) {
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
    utils::load_dotenv();

    // Create upload directory
    fs::create_dir_all(UPLOAD_FOLDER).ok();

    println!("Starting server on http://0.0.0.0:{}", PORT);

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(
                web::scope("")
                    .route("/upload", web::post().to(upload_file))
                    .route("/health", web::get().to(health_check))
            )
    })
    .bind(format!("0.0.0.0:{}", PORT))?
    .run()
    .await
}

