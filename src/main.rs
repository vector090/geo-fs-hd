use axum::extract::{FromRef, State};
use axum::http::Method;
use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, cors::{Any, CorsLayer}};

const PORT: u16 = 80;
const HOST: &str = "0.0.0.0";

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";

const CACHE_DB: &str = "cache";
const SERVERS: [&str; 9] = [
    "https://www.google.com/maps/vt/lyrs=s&x={x}&y={y}&z={z}",
    "https://mt0.google.com/vt/lyrs=s&x={x}&y={y}&z={z}",
    "https://mt1.google.com/vt/lyrs=s&x={x}&y={y}&z={z}",
    "https://mt2.google.com/vt/lyrs=s&x={x}&y={y}&z={z}",
    "https://mt3.google.com/vt/lyrs=s&x={x}&y={y}&z={z}",
    "https://khms0.google.com/kh/v=968?x={x}&y={y}&z={z}",
    "https://khms1.google.com/kh/v=968?x={x}&y={y}&z={z}",
    "https://khms2.google.com/kh/v=968?x={x}&y={y}&z={z}",
    "https://khms3.google.com/kh/v=968?x={x}&y={y}&z={z}",
];

#[derive(Clone)]
struct AppState {
    db: sled::Db,
    client: reqwest::Client,
}

impl FromRef<AppState> for reqwest::Client {
    fn from_ref(app_state: &AppState) -> reqwest::Client {
        app_state.client.clone()
    }
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        // allow `GET` when accessing the resource
        .allow_methods([Method::GET])
        // allow requests from any origin
        .allow_origin(Any);

    // create our db
    let db_options = sled::Config::default()
        .path(CACHE_DB)
        .mode(sled::Mode::HighThroughput)
        .cache_capacity(1024 * 1024 * 1024)
        .flush_every_ms(Some(1000))
        .use_compression(true);
    let db = db_options.open().unwrap();

    // create our client
    let client = reqwest::Client::new();

    // create our application state
    let state = AppState {
        db: db.clone(),
        client,
    };

    // build our application with a route
    let app = Router::new()
        // `GET /users` goes to `create_user`
        .route("/map/:z/:x/:y", get(get_tile))
        // add static files
        .nest_service("/", ServeDir::new("./static"))
        // create a state that holds our database
        .with_state(state)
        // create middleware
        .layer(ServiceBuilder::new().layer(cors));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", HOST, PORT))
        .await
        .unwrap();

    // run the server
    println!("Listening on port {}", PORT);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C signal handler");
        })
        .await
        .unwrap();

    // close the db
    println!("Closing DB");
    db.flush().unwrap();
    let size = db.size_on_disk().unwrap() as f64 / 1024.0 / 1024.0;
    println!("DB new size is {:.2} MB with {} entries", size, db.len());
}

// get a tile and cache it if it is not alread, else send the image
async fn get_tile(
    Path((z, x, y)): Path<(u8, i32, u32)>,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    // calculate the x
    let size = 2_i32.pow(z as u32) as u32;
    let x = x.rem_euclid(size as i32);

    // create image
    let img: Vec<u8>;

    // check if cache exists
    let path = format!("{}/{}/{}", z, x, y);
    if let Ok(Some(bytes)) = state.db.get(&path) {
        img = bytes.to_vec();
    } else {
        let mut attempt: u8 = 0;
        loop {
            let server = SERVERS[(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64
                % SERVERS.len() as u64) as usize];

            let url = server
                .replace("{x}", &x.to_string())
                .replace("{y}", &y.to_string())
                .replace("{z}", &z.to_string());

            let res = state
                .client
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await;

            match res {
                Ok(response) => {
                    if response.status() == 200 {
                        let bytes = response.bytes().await.unwrap();
                        img = bytes.to_vec();
                        break;
                    } else {
                        attempt += 1;
                        if attempt >= 3 || response.status() == 404 {
                            let mut headers = HeaderMap::new();
                            headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                headers,
                                "Internal Server Error".as_bytes().to_vec(),
                            );
                        }
                    }
                }
                Err(_) => {
                    attempt += 1;
                    if attempt >= 3 {
                        let mut headers = HeaderMap::new();
                        headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            headers,
                            "Internal Server Error".as_bytes().to_vec(),
                        );
                    }
                }
            }
        }

        // save to cache async (dont wait for it to finish)
        let img = img.clone();
        tokio::spawn(async move {
            state.db.insert(&path, img).unwrap();
        });
    }

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "max-age=31536000".parse().unwrap());

    (StatusCode::OK, headers, img)
}