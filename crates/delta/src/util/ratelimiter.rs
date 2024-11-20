use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use authifier::models::Session;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::uri::Origin;
use rocket::http::{Method, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use rocket::{Data, Request, Response};

use revolt_rocket_okapi::gen::OpenApiGenerator;
use revolt_rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use serde::Serialize;

use dashmap::DashMap;
use once_cell::sync::Lazy;

/// Ratelimit Bucket
#[derive(Clone, Copy, Debug)]
struct Entry {
    used: u8,
    reset: u128,
}

static MAP: Lazy<DashMap<u64, Entry>> = Lazy::new(DashMap::new);

/// Get the current time from Unix Epoch as a Duration
fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards...")
}

impl Entry {
    /// Find bucket by its key
    pub fn from(key: u64) -> Entry {
        MAP.get(&key).map(|x| *x).unwrap_or_else(|| Entry {
            used: 0,
            reset: now().add(Duration::from_secs(10)).as_millis(),
        })
    }

    /// Deduct one unit from the bucket and save
    pub fn deduct(&mut self) {
        let current_time = now().as_millis();
        if current_time > self.reset {
            self.used = 1;
            self.reset = now().add(Duration::from_secs(10)).as_millis();
        } else {
            self.used += 1;
        }
    }

    /// Save information
    pub fn save(self, key: u64) {
        MAP.insert(key, self);
    }

    /// Get remaining units in the bucket
    pub fn get_remaining(&self, limit: u8) -> u8 {
        if now().as_millis() > self.reset {
            limit
        } else {
            limit - self.used
        }
    }

    /// Get how long bucket has until reset
    pub fn left_until_reset(&self) -> u128 {
        let current_time = now().as_millis();
        if current_time > self.reset {
            0
        } else {
            self.reset - current_time
        }
    }
}

/// Ratelimit Guard
#[derive(Serialize, Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct Ratelimiter {
    key: u64,
    limit: u8,
    remaining: u8,
    reset: u128,
}

/// Find bucket from given request
///
/// Optionally, include a resource id to hash against.
fn resolve_bucket<'r>(request: &'r rocket::Request<'_>) -> (&'r str, Option<&'r str>) {
    if let Some(segment) = request.routed_segment(0) {
        let resource = request.routed_segment(1);

        let method = request.method();
        match (segment, resource, method) {
            ("users", target, Method::Patch) => ("user_edit", target),
            ("users", _, _) => {
                if let Some("default_avatar") = request.routed_segment(2) {
                    return ("default_avatar", None);
                }

                ("users", None)
            }
            ("bots", _, _) => ("bots", None),
            ("channels", Some(id), _) => {
                if request.method() == Method::Post {
                    if let Some("messages") = request.routed_segment(2) {
                        return ("messaging", Some(id));
                    }
                }

                ("channels", Some(id))
            }
            ("servers", Some(id), _) => ("servers", Some(id)),
            ("auth", _, _) => {
                if request.method() == Method::Delete {
                    ("auth_delete", None)
                } else {
                    ("auth", None)
                }
            }
            ("swagger", _, _) => ("swagger", None),
            ("safety", Some("report"), _) => ("safety_report", Some("report")),
            ("safety", _, _) => ("safety", None),
            _ => ("any", None),
        }
    } else {
        ("any", None)
    }
}

/// Resolve per-bucket limits
fn resolve_bucket_limit(bucket: &str) -> u8 {
    match bucket {
        "user_edit" => 2,
        "users" => 20,
        "bots" => 10,
        "messaging" => 10,
        "channels" => 15,
        "servers" => 5,
        "auth" => 15,
        "auth_delete" => 255,
        "default_avatar" => 255,
        "swagger" => 100,
        "safety" => 15,
        "safety_report" => 3,
        _ => 20,
    }
}

/// Find the remote IP of the client
fn to_ip(request: &'_ rocket::Request<'_>) -> String {
    request
        .remote()
        .map(|x| x.ip().to_string())
        .unwrap_or_default()
}

/// Find the actual IP of the client
fn to_real_ip(request: &'_ rocket::Request<'_>) -> String {
    if let Ok(true) = std::env::var("TRUST_CLOUDFLARE").map(|x| x == "1") {
        request
            .headers()
            .get_one("CF-Connecting-IP")
            .map(|x| x.to_string())
            .unwrap_or_else(|| to_ip(request))
    } else {
        to_ip(request)
    }
}

impl Ratelimiter {
    /// Generate guard from identifier and target bucket
    pub fn from(
        identifier: &str,
        (bucket, resource): (&str, Option<&str>),
    ) -> Result<Ratelimiter, Ratelimiter> {
        let mut key = DefaultHasher::new();
        key.write(identifier.as_bytes());
        key.write(bucket.as_bytes());

        if let Some(id) = resource {
            key.write(id.as_bytes());
        }

        let key = key.finish();
        let limit = resolve_bucket_limit(bucket);
        let mut entry = Entry::from(key);

        let remaining = entry.get_remaining(limit);
        let reset = entry.left_until_reset();
        let mut ratelimiter = Ratelimiter {
            key,
            limit,
            remaining,
            reset,
        };
        if remaining == 0 {
            return Err(ratelimiter);
        }

        entry.deduct();
        entry.save(key);
        ratelimiter.remaining -= 1;
        ratelimiter.reset = entry.left_until_reset();

        Ok(ratelimiter)
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Ratelimiter {
    type Error = Ratelimiter;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let ratelimiter = request
            .local_cache_async(async {
                use rocket::outcome::Outcome;
                let identifier = if let Outcome::Success(session) = request.guard::<Session>().await
                {
                    session.id
                } else {
                    to_real_ip(request)
                };

                Ratelimiter::from(&identifier, resolve_bucket(request))
            })
            .await;

        match ratelimiter {
            Ok(ratelimiter) => Outcome::Success(*ratelimiter),
            Err(ratelimiter) => Outcome::Error((Status::TooManyRequests, *ratelimiter)),
        }
    }
}

impl<'r> OpenApiFromRequest<'r> for Ratelimiter {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}

/// Attach ratelimiter to the Rocket application
pub struct RatelimitFairing;

#[rocket::async_trait]
impl Fairing for RatelimitFairing {
    fn info(&self) -> Info {
        Info {
            name: "Ratelimiter",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        use rocket::outcome::Outcome;
        if let Outcome::Error(_) = request.guard::<Ratelimiter>().await {
            info!(
                "User rate-limited on route {}! (IP = {:?})",
                request.uri(),
                to_real_ip(request)
            );

            request.set_method(Method::Get);
            request.set_uri(Origin::parse("/ratelimit").unwrap())
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let guard = request.guard::<Ratelimiter>().await;
        let (Outcome::Success(ratelimiter) | Outcome::Error((_, ratelimiter))) = guard else {
            unreachable!()
        };
        let Ratelimiter {
            key,
            limit,
            remaining,
            reset,
        } = ratelimiter;

        response.set_raw_header("X-RateLimit-Limit", limit.to_string());
        response.set_raw_header("X-RateLimit-Bucket", key.to_string());
        response.set_raw_header("X-RateLimit-Remaining", remaining.to_string());
        response.set_raw_header("X-RateLimit-Reset-After", reset.to_string());

        if guard.is_error() {
            response.set_status(Status::TooManyRequests);
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RatelimitInformation {
    Success(Ratelimiter),
    Failure { retry_after: u128 },
}

#[async_trait]
impl<'r> FromRequest<'r> for RatelimitInformation {
    type Error = u128;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let info = match request.guard::<Ratelimiter>().await {
            Outcome::Success(ratelimiter) => RatelimitInformation::Success(ratelimiter),
            Outcome::Error((_, ratelimiter)) => RatelimitInformation::Failure {
                retry_after: ratelimiter.reset,
            },
            _ => unreachable!(),
        };
        Outcome::Success(info)
    }
}

#[rocket::get("/ratelimit")]
fn ratelimit_info(info: RatelimitInformation) -> Json<RatelimitInformation> {
    Json(info)
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![ratelimit_info]
}
