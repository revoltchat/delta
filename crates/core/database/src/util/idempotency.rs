use std::num::NonZeroUsize;

use revolt_result::{create_error, Result};

#[cfg(feature = "rocket-impl")]
use revolt_result::Error;

use async_std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IdempotencyKey {
    key: String,
}

static TOKEN_CACHE: Lazy<Mutex<lru::LruCache<String, ()>>> =
    Lazy::new(|| Mutex::new(lru::LruCache::new(NonZeroUsize::new(1000).unwrap())));

impl IdempotencyKey {
    pub fn unchecked_from_string(key: String) -> Self {
        Self { key }
    }

    // Backwards compatibility.
    // Issue #109
    pub async fn consume_nonce(&mut self, v: Option<String>) -> Result<()> {
        if let Some(v) = v {
            let mut cache = TOKEN_CACHE.lock().await;
            if cache.get(&v).is_some() {
                return Err(create_error!(DuplicateNonce));
            }

            cache.put(v.clone(), ());
            self.key = v;
        }

        Ok(())
    }

    pub fn into_key(self) -> String {
        self.key
    }
}

#[cfg(feature = "rocket-impl")]
use revolt_rocket_okapi::{
    gen::OpenApiGenerator,
    request::{OpenApiFromRequest, RequestHeaderInput},
    revolt_okapi::openapi3::{Parameter, ParameterValue},
};

#[cfg(feature = "rocket-impl")]
use schemars::schema::{InstanceType, SchemaObject, SingleOrVec};

#[cfg(feature = "rocket-impl")]
impl<'r> OpenApiFromRequest<'r> for IdempotencyKey {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::Parameter(Parameter {
            name: "Idempotency-Key".to_string(),
            description: Some("Unique key to prevent duplicate requests".to_string()),
            allow_empty_value: false,
            required: false,
            deprecated: false,
            extensions: schemars::Map::new(),
            location: "header".to_string(),
            value: ParameterValue::Schema {
                allow_reserved: false,
                example: None,
                examples: None,
                explode: None,
                style: None,
                schema: SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                    ..Default::default()
                },
            },
        }))
    }
}

#[cfg(feature = "rocket-impl")]
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
};

#[cfg(feature = "rocket-impl")]
#[async_trait]
impl<'r> FromRequest<'r> for IdempotencyKey {
    type Error = Error;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(key) = request
            .headers()
            .get("Idempotency-Key")
            .next()
            .map(|k| k.to_string())
        {
            if key.len() > 64 {
                return Outcome::Error((
                    Status::BadRequest,
                    create_error!(FailedValidation {
                        error: "idempotency key too long".to_string(),
                    }),
                ));
            }

            let idempotency = IdempotencyKey { key };
            let mut cache = TOKEN_CACHE.lock().await;
            if cache.get(&idempotency.key).is_some() {
                return Outcome::Error((Status::Conflict, create_error!(DuplicateNonce)));
            }

            cache.put(idempotency.key.clone(), ());
            return Outcome::Success(idempotency);
        }

        Outcome::Success(IdempotencyKey {
            key: ulid::Ulid::new().to_string(),
        })
    }
}
