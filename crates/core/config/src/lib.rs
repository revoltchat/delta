use std::collections::HashMap;

use cached::proc_macro::cached;
use config::{Config, File, FileFormat};
use futures_locks::RwLock;
use once_cell::sync::Lazy;
use serde::Deserialize;

pub use sentry::{capture_error, capture_message, Level};

#[cfg(feature = "report-macros")]
#[macro_export]
macro_rules! report_error {
    ( $expr: expr, $error: ident $( $tt:tt )? ) => {
        $expr
            .inspect_err(|err| {
                $crate::capture_message(
                    &format!("{err:?} ({}:{}:{})", file!(), line!(), column!()),
                    $crate::Level::Error,
                );
            })
            .map_err(|_| ::revolt_result::create_error!($error))
    };
}

#[cfg(feature = "report-macros")]
#[macro_export]
macro_rules! capture_internal_error {
    ( $expr: expr ) => {
        $crate::capture_message(
            &format!("{:?} ({}:{}:{})", $expr, file!(), line!(), column!()),
            $crate::Level::Error,
        );
    };
}

#[cfg(feature = "report-macros")]
#[macro_export]
macro_rules! report_internal_error {
    ( $expr: expr ) => {
        $expr
            .inspect_err(|err| {
                $crate::capture_message(
                    &format!("{err:?} ({}:{}:{})", file!(), line!(), column!()),
                    $crate::Level::Error,
                );
            })
            .map_err(|_| ::revolt_result::create_error!(InternalError))
    };
}

/// Paths to search for configuration
static CONFIG_SEARCH_PATHS: [&str; 3] = [
    // current working directory
    "Revolt.toml",
    // current working directory - overrides file
    "Revolt.overrides.toml",
    // root directory, for Docker containers
    "/Revolt.toml",
];

/// Configuration builder
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
        }

        for path in CONFIG_SEARCH_PATHS {
            if std::path::Path::new(path).exists() {
                builder = builder.add_source(File::new(path, FileFormat::Toml));
            }
        }

        builder.build().unwrap()
    })
});

#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub mongodb: String,
    pub redis: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Rabbit {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Hosts {
    pub app: String,
    pub api: String,
    pub events: String,
    pub autumn: String,
    pub january: String,
    pub voso_legacy: String,
    pub voso_legacy_ws: String,
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
pub struct PushVapid {
    pub queue: String,
    pub private_key: String,
    pub public_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PushFcm {
    pub queue: String,
    pub key_type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PushApn {
    pub queue: String,
    pub sandbox: bool,
    pub pkcs8: String,
    pub key_id: String,
    pub team_id: String,
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
pub struct Api {
    pub registration: ApiRegistration,
    pub smtp: ApiSmtp,
    pub security: ApiSecurity,
    pub workers: ApiWorkers,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Pushd {
    pub production: bool,
    pub exchange: String,
    pub mass_mention_chunk_size: usize,

    // Queues
    pub message_queue: String,
    pub mass_mention_queue: String,
    pub fr_accepted_queue: String,
    pub fr_received_queue: String,
    pub generic_queue: String,
    pub ack_queue: String,

    pub vapid: PushVapid,
    pub fcm: PushFcm,
    pub apn: PushApn,
}

impl Pushd {
    fn get_routing_key(&self, key: String) -> String {
        match self.production {
            true => key + "-prd",
            false => key + "-tst",
        }
    }

    pub fn get_ack_routing_key(&self) -> String {
        self.get_routing_key(self.ack_queue.clone())
    }

    pub fn get_message_routing_key(&self) -> String {
        self.get_routing_key(self.message_queue.clone())
    }

    pub fn get_mass_mention_routing_key(&self) -> String {
        self.get_routing_key(self.mass_mention_queue.clone())
    }

    pub fn get_fr_accepted_routing_key(&self) -> String {
        self.get_routing_key(self.fr_accepted_queue.clone())
    }

    pub fn get_fr_received_routing_key(&self) -> String {
        self.get_routing_key(self.fr_received_queue.clone())
    }

    pub fn get_generic_routing_key(&self) -> String {
        self.get_routing_key(self.generic_queue.clone())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct FilesLimit {
    pub min_file_size: usize,
    pub min_resolution: [usize; 2],
    pub max_mega_pixels: usize,
    pub max_pixel_side: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FilesS3 {
    pub endpoint: String,
    pub path_style_buckets: bool,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub default_bucket: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Files {
    pub encryption_key: String,
    pub webp_quality: f32,
    pub blocked_mime_types: Vec<String>,
    pub clamd_host: String,
    pub scan_mime_types: Vec<String>,

    pub limit: FilesLimit,
    pub preview: HashMap<String, [usize; 2]>,
    pub s3: FilesS3,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GlobalLimits {
    pub group_size: usize,
    pub message_embeds: usize,
    pub message_replies: usize,
    pub message_reactions: usize,
    pub server_emoji: usize,
    pub server_roles: usize,
    pub server_channels: usize,

    pub new_user_hours: usize,

    pub body_limit_size: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesLimits {
    pub outgoing_friend_requests: usize,

    pub bots: usize,
    pub message_length: usize,
    pub message_attachments: usize,
    pub servers: usize,

    pub file_upload_size_limit: HashMap<String, usize>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesLimitsCollection {
    pub global: GlobalLimits,

    pub new_user: FeaturesLimits,
    pub default: FeaturesLimits,

    #[serde(flatten)]
    pub roles: HashMap<String, FeaturesLimits>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeaturesAdvanced {
    #[serde(default)]
    pub process_message_delay_limit: u16,
}

impl Default for FeaturesAdvanced {
    fn default() -> Self {
        Self {
            process_message_delay_limit: 5,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Features {
    pub limits: FeaturesLimitsCollection,
    pub webhooks_enabled: bool,
    pub mass_mentions_send_notifications: bool,
    pub mass_mentions_enabled: bool,

    #[serde(default)]
    pub advanced: FeaturesAdvanced,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Sentry {
    pub api: String,
    pub events: String,
    pub files: String,
    pub proxy: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub database: Database,
    pub rabbit: Rabbit,
    pub hosts: Hosts,
    pub api: Api,
    pub pushd: Pushd,
    pub files: Files,
    pub features: Features,
    pub sentry: Sentry,
}

impl Settings {
    pub fn preflight_checks(&self) {
        if self.api.smtp.host.is_empty() {
            log::warn!("No SMTP settings specified! Remember to configure email.");
        }

        if self.api.security.captcha.hcaptcha_key.is_empty() {
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
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    if std::env::var("ROCKET_ADDRESS").is_err() {
        std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    }

    if std::env::var("REDIS_URL").is_err() {
        // Configure redis-kiss library
        let config = config().await;
        std::env::set_var("REDIS_URI", config.database.redis);
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
