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
use walkdir::WalkDir;
use std::{fs, option, path::PathBuf };
use pulldown_cmark::{html, Options, Parser};


// Post parts
#[derive(Debug, Deserialize)]
#[derive(Clone)]
struct FrontMatter {
    title: String,
    date: String,
    excerpt: Option<String>,
    categories: Option<Vec<String>>,
    read_time: Option<String>,
    slug: String,
}

#[derive(Debug)]
struct Post {
    title: String,
    date: NaiveDate,
    excerpt: String,
    categories: Vec<String>,
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
#[template(path = "not_found.html")]
struct NotFoundTemplate<'a> {
    slug: &'a str,
}

#[derive(Template)]
#[template(path = "blog_index.html")]
struct BlogIndexTemplate<'a> {
    posts: &'a [Post],
}

fn load_and_process_all_posts() -> Vec<Post> {

    let mut posts = vec![];
    let matter = Matter::<YAML>::new();

    for entry in WalkDir::new("content/posts")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md" ))
        {
            let path = entry.path();

            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };

            let Ok(parsed) = matter.parse::<FrontMatter>(&content) else {
                continue;
            };

            let Some(fm) = parsed.data else {
                continue;
            };

            // Markdown -> HTML
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_STRIKETHROUGH);

            let parser = Parser::new_ext(parsed.content.trim(), options);
            let mut html_out = String::new();
            html::push_html(&mut html_out, parser);

            let date = chrono::NaiveDate::parse_from_str(&fm.date, "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let slug = if fm.slug.trim().is_empty() {
                path.file_stem().unwrap().to_string_lossy().to_string()
            } else {
                fm.slug
            };

            posts.push(Post {
                title: fm.title,
                date,
                excerpt: fm.excerpt.unwrap_or_default(),
                categories: fm.categories.unwrap_or_default(),
                read_time: fm.read_time.unwrap_or_else(|| "5 min".into()),
                slug,
                html: html_out,
            });
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));
        posts
//     mocking for now
//     let posts = vec![  Post {
//         title: "Blog 1".into(),
//         date: chrono::NaiveDate::from_ymd_opt(2025, 11, 3).unwrap(),
//         excerpt: "This is blog 1".into(),
//         category: "intro".into(),
//         read_time: "3 min".into(),
//         slug: "blog-1".into(),
//         html: String::new(),
//     },
//     Post {
//         title: "Blog 2".into(),
//         date: chrono::NaiveDate::from_ymd_opt(2025, 11, 4).unwrap(),
//         excerpt: "This is blog 2".into(),
//         category: "intro".into(),
//         read_time: "3 min".into(),
//         slug: "blog-2".into(),
//         html: String::new(),
//     },
//     Post {
//         title: "Blog 3".into(),
//         date: chrono::NaiveDate::from_ymd_opt(2025, 11, 5).unwrap(),
//         excerpt: "This is blog 3".into(),
//         category: "intro".into(),
//         read_time: "3 min".into(),
//         slug: "blog-3".into(),
//         html: String::new(),
//     }];
//     Html(BlogIndexTemplate { posts: &posts }.render().unwrap())


}

async fn blog_index() -> impl IntoResponse {
    let posts = load_and_process_all_posts();
    Html(BlogIndexTemplate{ posts: &posts }.render().unwrap())
}

// handler function
async fn index_handler() -> impl IntoResponse {

    let template = IndexTemplate {
        title: "Portfolio/Blog/Tech Website",
        name: "Erwin Acher"
    };
    Html(template.render().unwrap())
}

fn load_post(slug: &str) -> Option<Post> {
    let path = PathBuf::from(format!("content/posts/{}.md", slug));
    let content = fs::read_to_string(&path).ok()?;

    // Parse front matter + markdown body
    let matter = Matter::<YAML>::new();
    let result: ParsedEntity<FrontMatter> = matter.parse(&content).ok()?;

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
        categories: fm.categories.unwrap_or_default(),
        read_time: fm.read_time.unwrap_or_else(|| "5 min".into()),
        slug: slug_val.to_string(),
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
        .route("/blog", get(blog_index))
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

