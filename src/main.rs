use askama::Template;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router
};
//use serde::Deserialize;
use tower_http::services::ServeDir;

// #[derive(Debug, Deserialize)]
// struct FrontMatter {
//     title: String,
//     date: String,
//     slug: String,
// }

// #[derive(Debug)]
// struct Post {
//     title: String,
//     date: String,
//     slug: String,
//     html: String,
// }

// #[derive(Template)]
// #[template(path = "post.html")]
// struct PostTemplate<'a> {
//     post: &'a Post,
// }

// Template struct automatically binds to templates/hello.html
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    title: &'a str,
    name: &'a str,
}



// handler function
async fn index_handler() -> impl IntoResponse {
    let template = IndexTemplate { title: "Portfolio Website", name: "Ervin" };
    Html(template.render().unwrap())
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index_handler))
        // serve statuic files under /static/
        .nest_service("/static", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3002")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

