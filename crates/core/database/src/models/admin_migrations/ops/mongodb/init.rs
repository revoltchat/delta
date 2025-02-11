use super::scripts::LATEST_REVISION;

use crate::mongodb::bson::doc;
use crate::mongodb::options::CreateCollectionOptions;
use crate::MongoDb;

pub async fn create_database(db: &MongoDb) {
    info!("Creating database.");
    let db = db.db();

    db.create_collection("accounts")
        .await
        .expect("Failed to create accounts collection.");

    db.create_collection("users")
        .await
        .expect("Failed to create users collection.");

    db.create_collection("channels")
        .await
        .expect("Failed to create channels collection.");

    db.create_collection("messages")
        .await
        .expect("Failed to create messages collection.");

    db.create_collection("servers")
        .await
        .expect("Failed to create servers collection.");

    db.create_collection("server_members")
        .await
        .expect("Failed to create server_members collection.");

    db.create_collection("server_bans")
        .await
        .expect("Failed to create server_bans collection.");

    db.create_collection("channel_invites")
        .await
        .expect("Failed to create channel_invites collection.");

    db.create_collection("channel_unreads")
        .await
        .expect("Failed to create channel_unreads collection.");

    db.create_collection("channel_webhooks")
        .await
        .expect("Failed to create channel_webhooks collection.");

    db.create_collection("migrations")
        .await
        .expect("Failed to create migrations collection.");

    db.create_collection("attachments")
        .await
        .expect("Failed to create attachments collection.");

    db.create_collection("attachment_hashes")
        .await
        .expect("Failed to create attachment_hashes collection.");

    db.create_collection("user_settings")
        .await
        .expect("Failed to create user_settings collection.");

    db.create_collection("safety_reports")
        .await
        .expect("Failed to create safety_reports collection.");

    db.create_collection("safety_snapshots")
        .await
        .expect("Failed to create safety_snapshots collection.");

    db.create_collection("safety_strikes")
        .await
        .expect("Failed to create safety_strikes collection.");

    db.create_collection("bots")
        .await
        .expect("Failed to create bots collection.");

    db.create_collection("ratelimit_events")
        .await
        .expect("Failed to create ratelimit_events collection.");

    db.create_collection("pubsub")
        .with_options(
            CreateCollectionOptions::builder()
                .capped(true)
                .size(1_000_000)
                .build(),
        )
        .await
        .expect("Failed to create pubsub collection.");

    db.run_command(doc! {
        "createIndexes": "users",
        "indexes": [
            {
                "key": {
                    "username": 1_i32
                },
                "name": "username",
                "unique": false,
                "collation": {
                    "locale": "en",
                    "strength": 2_i32
                }
            },
            {
                "key": {
                    "username": 1_i32,
                    "discriminator": 1_i32
                },
                "name": "username_discriminator",
                "unique": true,
                "collation": {
                    "locale": "en",
                    "strength": 2_i32
                }
            }
        ]
    })
    .await
    .expect("Failed to create username index.");

    db.run_command(doc! {
        "createIndexes": "messages",
        "indexes": [
            {
                "key": {
                    "content": "text"
                },
                "name": "content"
            },
            {
                "key": {
                    "channel": 1_i32,
                    "_id": 1_i32
                },
                "name": "channel_id_compound"
            },
            {
                "key": {
                    "author": 1_i32
                },
                "name": "author"
            },
            {
                "key": {
                    "channel": 1_i32,
                    "pinned": 1_i32
                },
                "name": "channel_pinned_compound"
            },
        ]
    })
    .await
    .expect("Failed to create message index.");

    db.run_command(doc! {
        "createIndexes": "channel_unreads",
        "indexes": [
            {
                "key": {
                    "_id.channel": 1_i32,
                    "_id.user": 1_i32,
                },
                "name": "compound_id"
            },
            {
                "key": {
                    "_id.user": 1_i32,
                },
                "name": "user_id"
            }
        ]
    })
    .await
    .expect("Failed to create channel_unreads index.");

    db.run_command(doc! {
        "createIndexes": "server_members",
        "indexes": [
            {
                "key": {
                    "_id.server": 1_i32,
                    "_id.user": 1_i32,
                },
                "name": "compound_id"
            },
            {
                "key": {
                    "_id.user": 1_i32,
                },
                "name": "user_id"
            }
        ]
    })
    .await
    .expect("Failed to create server_members index.");

    db.run_command(doc! {
        "createIndexes": "attachments",
        "indexes": [
            {
                "key": {
                    "hash": 1_i32
                },
                "name": "hash"
            },
            {
                "key": {
                    "used_for.id": 1_i32
                },
                "name": "used_for_id"
            }
        ]
    })
    .await
    .expect("Failed to create attachments index.");

    db.run_command(doc! {
        "createIndexes": "attachment_hashes",
        "indexes": [
            {
                "key": {
                    "processed_hash": 1_i32
                },
                "name": "processed_hash"
            }
        ]
    })
    .await
    .expect("Failed to create attachment_hashes index.");

    db.collection("migrations")
        .insert_one(doc! {
            "_id": 0_i32,
            "revision": LATEST_REVISION
        })
        .await
        .expect("Failed to save migration info.");

    db.run_command(doc! {
        "createIndexes": "ratelimit_events",
        "indexes": [
            {
                "key": {
                    "_id": 1_i32,
                    "target_id": 1_i32,
                    "event_type": 1_i32,
                },
                "name": "compound_key"
            }
        ]
    })
    .await
    .expect("Failed to create ratelimit_events index.");

    info!("Created database.");
}
