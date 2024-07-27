#[macro_use]
extern crate diesel;

use std::env;
use actix_web::{App, get, HttpResponse, HttpServer, post, Responder, web};
use actix_web::web::Data;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use dotenv::dotenv;
use tera::Tera;

use self::models::{NewPostHandler, Post};
use self::schema::posts::dsl::*;

pub mod schema;
pub mod models;
pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[get("/")]
async fn index(pool: Data<DbPool>, template_manager: Data<Tera>) -> impl Responder {
    let mut conn = pool.get().expect("Problemas al traer la base de datos");

    match web::block(move || { posts.load::<Post>(&mut conn) }).await {
        Ok(data) => {
            let data = data.unwrap();
            let mut context = tera::Context::new();
            context.insert("posts", &data);
            HttpResponse::Ok().content_type("text/html").body(
                template_manager.render("index.html", &context).unwrap()
            )
        }
        Err(_err) => HttpResponse::Ok().body("Error al recibir la data")
    }
}

#[get("/blog/{blog_slug}")]
async fn get_post(
    pool: Data<DbPool>,
    template_manager: Data<Tera>,
    blog_slug: web::Path<String>,
) -> impl Responder {
    let mut conn = pool.get().expect("Problemas al traer la base de datos");
    let url_slug = blog_slug.into_inner();
    match web::block(move || { posts.filter(slug.eq(url_slug)).load::<Post>(&mut conn) }).await {
        Ok(data) => {
            let data = data.unwrap();
            if data.len() == 0 {
                return HttpResponse::NotFound().finish();
            }
            let data = &data[0];
            let mut ctx = tera::Context::new();
            ctx.insert("post", data);
            HttpResponse::Ok().content_type("text/html").body(
                template_manager.render("posts.html", &ctx).unwrap()
            )
        }
        Err(_err) => HttpResponse::Ok().body("Error al recibir la data")
    }
}

#[post("/new_post")]
async fn new_post(pool: Data<DbPool>, item: web::Json<NewPostHandler>) -> impl Responder {
    let mut conn = pool.get().expect("Problemas al traer la base de datos");
    match web::block(move || { Post::create_post(&mut conn, &item) }).await {
        Ok(data) => {
            return HttpResponse::Ok().body(format!("{:?}", data));
        }
        Err(_err) => HttpResponse::Ok().body("Error al recibir la data")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("db url variable no encontrada");
    let port = env::var("PORT").expect("db url variable no encontrada");
    let port: u16 = port.parse().unwrap();
    let connection = ConnectionManager::<PgConnection>::new(db_url);
    let pool = Pool::builder().build(connection).expect("No se pudo construir la Pool");
    HttpServer::new(move || {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();
        App::new()
            .service(index)
            .service(new_post)
            .service(get_post)
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(tera))
    }).bind(("127.0.0.1", port)).unwrap().run().await
}
