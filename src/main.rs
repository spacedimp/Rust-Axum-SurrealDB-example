use axum::{extract::State, routing::get, Router, Json, extract::Path, response::IntoResponse };
use surrealdb::Surreal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use surrealdb::engine::any::Any;


mod error {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::response::Response;
    use axum::Json;
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("database error")]
        Db,
    }

    impl IntoResponse for Error {
        fn into_response(self) -> Response {
            match self {
            Error::Db=> (StatusCode::INTERNAL_SERVER_ERROR, Json("An error has occurred. Please try again later.".to_string())).into_response(),
            }
        }
    }

    impl From<surrealdb::Error> for Error {
        fn from(error: surrealdb::Error) -> Self {
            eprintln!("{error}");
            Self::Db
        }
    }
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Surreal<Any>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    username: String,
}

async fn get_users(State(state): State<AppState>) -> Result<Json<Vec<User>>, error::Error>{
    let db = state.db.lock().await;
    let mut response = db.query("SELECT * FROM users").await?;

    let result: Vec<User> = response.take(0)?;
    println!("{:?}", result);

    Ok(Json(result))

}

async fn create_user(Path(uname): Path<String>, State(state): State<AppState>) -> Result<impl IntoResponse, error::Error>{
   let db = state.db.lock().await;

   let newUser: Option<User> = db.create("users").content(User {
        username: uname,
    }).await?;

    Ok("Success creating new user")
}

async fn delete_user(Path(uname): Path<String>, State(state): State<AppState>) -> Result<impl IntoResponse, error::Error>{
   let db = state.db.lock().await;

    db.query("DELETE FROM users WHERE username = $username")
        .bind(("username", uname)).await?;

    Ok("Success deleting user")
}


#[tokio::main]
async fn main() -> Result<(), error::Error>{
    let db = surrealdb::engine::any::connect("surrealkv://mydb").await?;

db.signin(surrealdb::opt::auth::Root {
    username: "root",
    password: "password",
}).await?;

db.use_ns("test_ns").use_db("test_db").await?;

db.query(
    "DEFINE TABLE IF NOT EXISTS users SCHEMAFULL;
    DEFINE FIELD IF NOT EXISTS username ON TABLE users TYPE string ASSERT string::len($value) < 14;
    DEFINE INDEX IF NOT EXISTS usernameIndex ON TABLE users COLUMNS username UNIQUE;
    "
).await?;

let app_state = AppState {
    db: Arc::new(Mutex::new(db))
};

let app = Router::new()
    .route("/", get(get_users))
    .route("/create/:uname", get(create_user))
    .route("/delete/:uname", get(delete_user))
    .with_state(app_state);

let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await.unwrap();

Ok(())
}
