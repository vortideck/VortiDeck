use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use warp::Filter;

/// HTTP server for serving icon files to mobile clients
pub struct IconServer {
    icons_dir: Arc<PathBuf>,
    ip: IpAddr,
    port: Option<u16>,
}

impl IconServer {
    pub fn new(icons_dir: PathBuf, ip: IpAddr) -> Self {
        Self {
            icons_dir: Arc::new(icons_dir),
            ip,
            port: None,
        }
    }

    /// Find an available port for the icons server
    pub async fn find_available_port(&self) -> Option<u16> {
        for port in 9002..9100 {
            if TcpListener::bind(SocketAddr::new(self.ip, port))
                .await
                .is_ok()
            {
                return Some(port);
            }
        }
        None
    }

    /// Start the HTTP server on the specified port
    pub async fn start(&mut self, port: u16) -> Result<(), String> {
        self.port = Some(port);

        let icons_dir = self.icons_dir.clone();
        let route = warp::path!("icons" / String / String)
            .and(warp::any().map(move || icons_dir.clone()))
            .and_then(serve_icon_file);

        // Add CORS headers for cross-origin requests from mobile apps
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET"]);

        let routes = route.with(cors);

        println!("🖼️  Icons server starting on {}:{}", self.ip, port);

        warp::serve(routes).run((self.ip, port)).await;

        Ok(())
    }

    /// Start the server on an automatically found port
    pub async fn start_auto_port(&mut self) -> Result<u16, String> {
        let port = self
            .find_available_port()
            .await
            .ok_or("No available ports found for icons server")?;

        // Start server in background task
        let icons_dir = self.icons_dir.clone();
        let ip = self.ip;

        tokio::spawn(async move {
            let route = warp::path!("icons" / String / String)
                .and(warp::any().map(move || icons_dir.clone()))
                .and_then(serve_icon_file);

            let cors = warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["content-type"])
                .allow_methods(vec!["GET"]);

            let routes = route.with(cors);

            println!("🖼️  Icons server running on {}:{}", ip, port);
            warp::serve(routes).run((ip, port)).await;
        });

        self.port = Some(port);
        Ok(port)
    }

    /// Get the current server port
    pub fn get_port(&self) -> Option<u16> {
        self.port
    }

    /// Get the server URL
    pub fn get_server_url(&self) -> Option<String> {
        self.port.map(|port| format!("http://{}:{}", self.ip, port))
    }

    /// Health check endpoint
    pub fn is_running(&self) -> bool {
        self.port.is_some()
    }
}

/// Serve individual icon files
async fn serve_icon_file(
    button_id: String,
    file_name: String,
    icons_dir: Arc<PathBuf>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let file_path = icons_dir.join(&button_id).join(&file_name);

    if file_path.exists() && file_path.is_file() {
        match tokio::fs::read(file_path).await {
            Ok(contents) => {
                // Determine content type based on file extension
                let content_type = match file_name.split('.').last() {
                    Some("webp") => "image/webp",
                    Some("png") => "image/png",
                    Some("jpg") | Some("jpeg") => "image/jpeg",
                    Some("gif") => "image/gif",
                    _ => "application/octet-stream",
                };

                Ok(warp::http::Response::builder()
                    .header("Content-Type", content_type)
                    .header("Cache-Control", "public, max-age=3600") // Cache for 1 hour
                    .body(contents)
                    .unwrap())
            }
            Err(_) => Err(warp::reject::not_found()),
        }
    } else {
        Err(warp::reject::not_found())
    }
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct IconServerConfig {
    pub ip: IpAddr,
    pub port_range: (u16, u16),
    pub enable_cors: bool,
    pub cache_max_age: u32,
}

impl Default for IconServerConfig {
    fn default() -> Self {
        Self {
            ip: "0.0.0.0".parse().unwrap(),
            port_range: (9002, 9100),
            enable_cors: true,
            cache_max_age: 3600, // 1 hour
        }
    }
}
