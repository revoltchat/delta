use std::collections::HashMap;

use cached::proc_macro::cached;
use config::{Config, File, FileFormat};
use futures_locks::RwLock;
use once_cell::sync::Lazy;
use serde::Deserialize;

#[cfg(not(debug_assertions))]
use std::env;

static CONFIG_BUILDER: Lazy<RwLock<Config>> = Lazy::new(|| {
    RwLock::new({
        let mut builder = Config::builder().add_source(File::from_str(
            include_str!("../Revolt.toml"),
            FileFormat::Toml,
        ));

        if std::env::var("TEST_DB").is_ok() {
            builder = builder.add_source(File::from_str(
                include_str!("../Revolt.test.toml"),
                FileFormat::Toml,
            ));
        } else if std::path::Path::new("Revolt.toml").exists() {
            builder = builder.add_source(File::new("Revolt.toml", FileFormat::Toml));
        }

        builder.build().unwrap()
    })
});

// https://gifbox.me/view/gT5mqxYKCZv-twilight-meow

#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub mongodb: String,
    pub redis: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Hosts {
    pub app: String,
    pub api: String,
    pub events: String,
    pub autumn: String,
    pub january: String,
    pub livekit: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiRegistration {
    pub invite_only: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiSmtp {
    pub host: String,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub reply_to: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiVapid {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiFcm {
    pub api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiSecurityCaptcha {
    pub hcaptcha_key: String,
    pub hcaptcha_sitekey: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiSecurity {
    pub authifier_shield_key: String,
    pub voso_legacy_token: String,
    pub captcha: ApiSecurityCaptcha,
    pub trust_cloudflare: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiWorkers {
    pub max_concurrent_connections: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiLiveKit {
    pub url: String,
    pub key: String,
    pub secret: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Api {
    pub registration: ApiRegistration,
    pub smtp: ApiSmtp,
    pub vapid: ApiVapid,
    pub fcm: ApiFcm,
    pub security: ApiSecurity,
    pub workers: ApiWorkers,
    pub livekit: ApiLiveKit,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesLimits {
    pub group_size: usize,
    pub bots: usize,
    pub message_length: usize,
    pub message_replies: usize,
    pub message_attachments: usize,
    pub message_embeds: usize,
    pub message_reactions: usize,
    pub servers: usize,
    pub server_emoji: usize,
    pub server_roles: usize,
    pub server_channels: usize,

    pub attachment_size: usize,
    pub avatar_size: usize,
    pub background_size: usize,
    pub icon_size: usize,
    pub banner_size: usize,
    pub emoji_size: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesLimitsCollection {
    pub default: FeaturesLimits,

    #[serde(flatten)]
    pub roles: HashMap<String, FeaturesLimits>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Features {
    pub limits: FeaturesLimitsCollection,
    pub webhooks_enabled: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Sentry {
    pub api: String,
    pub events: String,
    pub voice_ingress: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub database: Database,
    pub hosts: Hosts,
    pub api: Api,
    pub features: Features,
    pub sentry: Sentry,
}

impl Settings {
    pub fn preflight_checks(&self) {
        if self.api.smtp.host.is_empty() {
            #[cfg(not(debug_assertions))]
            if !env::var("REVOLT_UNSAFE_NO_EMAIL").map_or(false, |v| v == *"1") {
                panic!("Running in production without email is not recommended, set REVOLT_UNSAFE_NO_EMAIL=1 to override.");
            }

            #[cfg(debug_assertions)]
            log::warn!("No SMTP settings specified! Remember to configure email.");
        }

        if self.api.security.captcha.hcaptcha_key.is_empty() {
            #[cfg(not(debug_assertions))]
            if !env::var("REVOLT_UNSAFE_NO_CAPTCHA").map_or(false, |v| v == *"1") {
                panic!("Running in production without CAPTCHA is not recommended, set REVOLT_UNSAFE_NO_CAPTCHA=1 to override.");
            }

            #[cfg(debug_assertions)]
            log::warn!("No Captcha key specified! Remember to add hCaptcha key.");
        }
    }
}

pub async fn init() {
    println!(
        ":: Revolt Configuration ::\n\x1b[32m{:?}\x1b[0m",
        config().await
    );
}

pub async fn read() -> Config {
    CONFIG_BUILDER.read().await.clone()
}

#[cached(time = 30)]
pub async fn config() -> Settings {
    read().await.try_deserialize::<Settings>().unwrap()
}

/// Configure logging and common Rust variables
pub async fn setup_logging(release: &'static str, dsn: String) -> Option<sentry::ClientInitGuard> {
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    if std::env::var("ROCKET_ADDRESS").is_err() {
        std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    }

    pretty_env_logger::init();
    log::info!("Starting {release}");

    if dsn.is_empty() {
        None
    } else {
        Some(sentry::init((
            dsn,
            sentry::ClientOptions {
                release: Some(release.into()),
                ..Default::default()
            },
        )))
    }
}

#[macro_export]
macro_rules! configure {
    ($application: ident) => {
        let config = $crate::config().await;
        let _sentry = $crate::setup_logging(
            concat!(env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION")),
            config.sentry.$application,
        )
        .await;
    };
}

#[cfg(feature = "test")]
#[cfg(test)]
mod tests {
    use crate::init;

    #[async_std::test]
    async fn it_works() {
        init().await;
    }
}
