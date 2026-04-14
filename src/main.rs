#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::*;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use watchtower::db;
    use watchtower::frontend::app::App;

    // Load .env
    dotenvy::dotenv().ok();

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter("watchtower=info,actix_web=info")
        .init();

    // Star Wars startup banner
    println!("\n{}", r#"
    ╔══════════════════════════════════════════════════════════╗
    ║                                                          ║
    ║   ⚔️  Watchtower — Rust Edition                         ║
    ║                                                          ║
    ║   "I find your lack of security disturbing."             ║
    ║                                                          ║
    ║   🌐 Server: http://0.0.0.0:66                           ║
    ║   📊 Dashboard: http://localhost:66                       ║
    ║                                                          ║
    ╚══════════════════════════════════════════════════════════╝
    "#);

    println!("🚀 Execute Order 66 — Server initializing...\n");

    // Database
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///app/data/watchtower.db?mode=rwc".into());
    let pool = db::init_db(&db_url).await;
    let pool_data = web::Data::new(pool);

    // Leptos config
    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);

    println!("✅ Database initialized");
    println!("✅ Listening on {}", addr);
    println!("────────────────────────────────────────\n");

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        actix_web::App::new()
            .app_data(pool_data.clone())
            .configure(watchtower::api::configure)
            .route("/api/{tail:.*}", web::to(HttpResponse::NotFound))
            .leptos_routes(leptos_options.to_owned(), routes.clone(), App)
            .service(Files::new("/", site_root.clone()))
            .wrap(middleware::Compress::default())
            .wrap(tracing_actix_web::TracingLogger::default())
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // This is the WASM entry point — hydrate() in lib.rs handles it
}
