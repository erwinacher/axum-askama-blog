use chrono::{NaiveDate, ParseError};
use serde::Deserialize;
use askama::Template;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router
};

//use serde::Deserialize;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use http::{header::{CACHE_CONTROL, HeaderValue}, status};
use tower_layer::Layer;
use gray_matter::{engine::YAML, Matter, ParsedEntity};
use axum::extract::Path;
use std::{fs, path::{PathBuf} };
use pulldown_cmark::{html, Options, Parser};


// Post parts
#[derive(Debug, Deserialize)]
#[derive(Clone)]
struct FrontMatter {
    title: String,
    date: String,
    excerpt: Option<String>,
    category: Option<String>,
    read_time: Option<String>,
    slug: String,
}

#[derive(Debug)]
struct Post {
    title: String,
    date: NaiveDate,
    excerpt: String,
    category: String,
    read_time: String,
    slug: String,
    html: String,
}

#[derive(Template)]
#[template(path = "post.html")]
struct PostTemplate<'a> {
    post: &'a Post,
}

// Template struct automatically binds to templates/hello.html
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    title: &'a str,
    name: &'a str,
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate<'a> {
    slug: &'a str,
}

// handler function
async fn index_handler() -> impl IntoResponse {
    let template = IndexTemplate { title: "Portfolio Website", name: "Ervin" };
    Html(template.render().unwrap())
}

fn load_post(slug: &str) -> Option<Post> {
    let path = PathBuf::from(format!("content/posts/{}.md", slug));
    let content = fs::read_to_string(&path).ok()?;

    // Parse front matter + markdown body
    let matter = Matter::<YAML>::new();
    let result: ParsedEntity<FrontMatter> = matter.parse(&content).ok()?;

    // ✅ Safely unwrap the Option<FrontMatter> only once
    let fm = result.data?;

    // Convert markdown -> HTML
    let markdown = result.content.trim();

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);

    // Parse date safely
    let date = NaiveDate::parse_from_str(&fm.date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

    let slug_val = if fm.slug.trim().is_empty() {
        slug.to_owned()
    } else {
        fm.slug
    };

    Some(Post {
        title: fm.title,
        date,
        excerpt: fm.excerpt.unwrap_or_default(),
        category: fm.category.unwrap_or_else(|| "misc".into()),
        read_time: fm.read_time.unwrap_or_else(|| "5 min".into()),
        slug: slug.to_string(),
        html: html_out,
    })
}

/// Render any Askama template into an Axum response.
fn render<T: Template>(tpl: &T, status: axum::http::StatusCode) -> impl IntoResponse {
    match tpl.render() {
        Ok(s) => (status, axum::response::Html(s)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {e}"),
        )
            .into_response(),
    }
}

async fn show_post(Path(slug): Path<String>) -> impl IntoResponse {
    if let Some(post) = load_post(&slug) {
        render(&PostTemplate { post: &post }, axum::http::StatusCode::OK).into_response()
    } else {
        render(&NotFoundTemplate { slug: &slug }, axum::http::StatusCode::NOT_FOUND).into_response()
    }
}

#[tokio::main]
async fn main() {
    let static_files = ServeDir::new("static")
        .append_index_html_on_directories(false);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/blog/{slug}", get(show_post))
        // serve statuic files under /static/
        //.nest_service("/static", ServeDir::new("static"));
        .nest_service(
        "/static",
        SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=1"),
        )
        .layer(static_files),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3002")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

