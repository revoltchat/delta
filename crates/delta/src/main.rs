#[macro_use]
extern crate rocket;
#[macro_use]
extern crate revolt_rocket_okapi;
#[macro_use]
extern crate serde_json;

pub mod routes;
pub mod util;

use revolt_config::config;
use revolt_database::events::client::EventV1;
use revolt_database::{Database, MongoDb};
use rocket::{Build, Rocket};
use rocket_cors::{AllowedOrigins, CorsOptions};
use rocket_prometheus::PrometheusMetrics;
use std::net::Ipv4Addr;
use std::str::FromStr;

use async_std::channel::unbounded;
use authifier::config::{
    Captcha, Config as AuthifierConfig, EmailVerificationConfig, ResolveIp, SMTPSettings, Shield,
    Template, Templates,
};
use authifier::{Authifier, AuthifierEvent};
use rocket::data::ToByteUnit;
use livekit_api::services::room::RoomClient;

pub async fn web() -> Rocket<Build> {
    // Get settings
    let config = config().await;

    // Ensure environment variables are present
    config.preflight_checks();

    // Setup database
    let db = revolt_database::DatabaseInfo::Auto.connect().await.unwrap();
    log::info!("database_here {db:?}");
    db.migrate_database().await.unwrap();

    // Setup Authifier event channel
    let (sender, receiver) = unbounded();

    // Setup Authifier
    let authifier = Authifier {
        database: match db.clone() {
            Database::Reference(_) => Default::default(),
            Database::MongoDb(MongoDb(client, _)) => authifier::Database::MongoDb(
                authifier::database::MongoDb(client.database("revolt")),
            ),
        },
        config: Default::default(),
        // config: authifier_config().await,
        event_channel: Some(sender),
    };

    // Launch a listener for Authifier events
    async_std::task::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            match &event {
                AuthifierEvent::CreateSession { .. } | AuthifierEvent::CreateAccount { .. } => {
                    EventV1::Auth(event).global().await
                }
                AuthifierEvent::DeleteSession { user_id, .. }
                | AuthifierEvent::DeleteAllSessions { user_id, .. } => {
                    let id = user_id.to_string();
                    EventV1::Auth(event).private(id).await
                }
            }
        }
    });

    // Launch background task workers
    async_std::task::spawn(revolt_database::tasks::start_workers(
        db.clone(),
        authifier.database.clone(),
    ));

    // Configure CORS
    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::All,
        allowed_methods: [
            "Get", "Put", "Post", "Delete", "Options", "Head", "Trace", "Connect", "Patch",
        ]
        .iter()
        .map(|s| FromStr::from_str(s).unwrap())
        .collect(),
        ..Default::default()
    }
    .to_cors()
    .expect("Failed to create CORS.");

    // Configure Swagger
    let swagger = revolt_rocket_okapi::swagger_ui::make_swagger_ui(
        &revolt_rocket_okapi::swagger_ui::SwaggerUIConfig {
            url: "../openapi.json".to_owned(),
            ..Default::default()
        },
    )
    .into();

    // Voice handler
    let voice_client = revolt_voice::VoiceClient::new(config.api.livekit.url.clone(), config.api.livekit.key.clone(), config.api.livekit.secret.clone());

    // Configure Rocket
    let rocket = rocket::build();
    let prometheus = PrometheusMetrics::new();

    routes::mount(config, rocket)
        .attach(prometheus.clone())
        .mount("/metrics", prometheus)
        .mount("/", rocket_cors::catch_all_options_routes())
        .mount("/", util::ratelimiter::routes())
        .mount("/swagger/", swagger)
        .manage(authifier)
        .manage(db)
        .manage(cors.clone())
        .manage(voice_client)
        .attach(util::ratelimiter::RatelimitFairing)
        .attach(cors)
        .configure(rocket::Config {
            limits: rocket::data::Limits::default().limit("string", 5.megabytes()),
            address: Ipv4Addr::new(0, 0, 0, 0).into(),
            ..Default::default()
        })
}

pub async fn authifier_config() -> AuthifierConfig {
    let config = config().await;

    let mut auth_config = AuthifierConfig {
        email_verification: if !config.api.smtp.host.is_empty() {
            EmailVerificationConfig::Enabled {
                smtp: SMTPSettings {
                    from: config.api.smtp.from_address,
                    host: config.api.smtp.host,
                    username: config.api.smtp.username,
                    password: config.api.smtp.password,
                    reply_to: Some(
                        config
                            .api
                            .smtp
                            .reply_to
                            .unwrap_or("support@revolt.chat".into()),
                    ),
                    port: config.api.smtp.port,
                    use_tls: config.api.smtp.use_tls,
                },
                expiry: Default::default(),
                templates: Templates {
                    verify: Template {
                        title: "Verify your Revolt account.".into(),
                        text: include_str!("templates/verify.txt").into(),
                        url: format!("{}/login/verify/", config.hosts.app),
                        html: Some(include_str!("templates/verify.html").into()),
                    },
                    reset: Template {
                        title: "Reset your Revolt password.".into(),
                        text: include_str!("templates/reset.txt").into(),
                        url: format!("{}/login/reset/", config.hosts.app),
                        html: Some(include_str!("templates/reset.html").into()),
                    },
                    deletion: Template {
                        title: "Confirm account deletion.".into(),
                        text: include_str!("templates/deletion.txt").into(),
                        url: format!("{}/delete/", config.hosts.app),
                        html: Some(include_str!("templates/deletion.html").into()),
                    },
                    welcome: None,
                },
            }
        } else {
            EmailVerificationConfig::Disabled
        },
        ..Default::default()
    };

    auth_config.invite_only = config.api.registration.invite_only;

    if !config.api.security.captcha.hcaptcha_key.is_empty() {
        auth_config.captcha = Captcha::HCaptcha {
            secret: config.api.security.captcha.hcaptcha_key,
        };
    }

    if !config.api.security.authifier_shield_key.is_empty() {
        auth_config.shield = Shield::Enabled {
            api_key: config.api.security.authifier_shield_key,
            strict: false,
        };
    }

    if config.api.security.trust_cloudflare {
        auth_config.resolve_ip = ResolveIp::Cloudflare;
    }

    auth_config
}

#[launch]
async fn rocket() -> _ {
    // Configure logging and environment
    revolt_config::configure!(api);

    // Start web server
    web().await
}
