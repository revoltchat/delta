use std::{collections::HashSet, ops::BitXor, time::Duration};

use crate::{
    mongodb::{
        bson::{doc, from_bson, from_document, to_document, Bson, DateTime, Document},
        options::FindOptions,
    },
    MongoDb, DISCRIMINATOR_SEARCH_SPACE,
};
use futures::StreamExt;
use rand::seq::SliceRandom;
use revolt_permissions::DEFAULT_WEBHOOK_PERMISSIONS;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Serialize, Deserialize)]
struct MigrationInfo {
    _id: i32,
    revision: i32,
}

pub const LATEST_REVISION: i32 = 27;

pub async fn migrate_database(db: &MongoDb) {
    let migrations = db.col::<Document>("migrations");
    let data = migrations
        .find_one(None, None)
        .await
        .expect("Failed to fetch migration data.");

    if let Some(doc) = data {
        let info: MigrationInfo =
            from_document(doc).expect("Failed to read migration information.");

        let revision = run_migrations(db, info.revision).await;

        migrations
            .update_one(
                doc! {
                    "_id": info._id
                },
                doc! {
                    "$set": {
                        "revision": revision
                    }
                },
                None,
            )
            .await
            .expect("Failed to commit migration information.");

        info!("Migration complete. Currently at revision {}.", revision);
    } else {
        panic!("Database was configured incorrectly, possibly because initalization failed.")
    }
}

pub async fn run_migrations(db: &MongoDb, revision: i32) -> i32 {
    info!("Starting database migration.");

    if revision <= 0 {
        info!("Running migration [revision 0]: Test migration system.");
    }

    if revision <= 1 {
        info!("Running migration [revision 1 / 2021-04-24]: Migrate to Autumn v1.0.0.");

        let messages = db.col::<Document>("messages");
        let attachments = db.col::<Document>("attachments");

        messages
            .update_many(
                doc! { "attachment": { "$exists": 1_i32 } },
                doc! { "$set": { "attachment.tag": "attachments", "attachment.size": 0_i32 } },
                None,
            )
            .await
            .expect("Failed to update messages.");

        attachments
            .update_many(
                doc! {},
                doc! { "$set": { "tag": "attachments", "size": 0_i32 } },
                None,
            )
            .await
            .expect("Failed to update attachments.");
    }

    if revision <= 2 {
        info!("Running migration [revision 2 / 2021-05-08]: Add servers collection.");

        db.db()
            .create_collection("servers", None)
            .await
            .expect("Failed to create servers collection.");
    }

    if revision <= 3 {
        info!("Running migration [revision 3 / 2021-05-25]: Support multiple file uploads, add channel_unreads and user_settings.");

        let messages = db.col::<Document>("messages");
        let mut cursor = messages
            .find(
                doc! {
                    "attachment": {
                        "$exists": 1_i32
                    }
                },
                FindOptions::builder()
                    .projection(doc! {
                        "_id": 1_i32,
                        "attachments": [ "$attachment" ]
                    })
                    .build(),
            )
            .await
            .expect("Failed to fetch messages.");

        while let Some(result) = cursor.next().await {
            let doc = result.unwrap();
            let id = doc.get_str("_id").unwrap();
            let attachments = doc.get_array("attachments").unwrap();

            messages
                .update_one(
                    doc! { "_id": id },
                    doc! { "$unset": { "attachment": 1_i32 }, "$set": { "attachments": attachments } },
                    None,
                )
                .await
                .unwrap();
        }

        db.db()
            .create_collection("channel_unreads", None)
            .await
            .expect("Failed to create channel_unreads collection.");

        db.db()
            .create_collection("user_settings", None)
            .await
            .expect("Failed to create user_settings collection.");
    }

    if revision <= 4 {
        info!("Running migration [revision 4 / 2021-06-01]: Add more server collections.");

        db.db()
            .create_collection("server_members", None)
            .await
            .expect("Failed to create server_members collection.");

        db.db()
            .create_collection("server_bans", None)
            .await
            .expect("Failed to create server_bans collection.");

        db.db()
            .create_collection("channel_invites", None)
            .await
            .expect("Failed to create channel_invites collection.");
    }

    if revision <= 5 {
        info!("Running migration [revision 5 / 2021-06-26]: Add permissions.");

        #[derive(Serialize)]
        struct Server {
            pub default_permissions: (i32, i32),
        }

        let server = Server {
            default_permissions: (0_i32, 0_i32),
        };

        db.col::<Document>("servers")
            .update_many(
                doc! {},
                doc! {
                    "$set": to_document(&server).unwrap()
                },
                None,
            )
            .await
            .expect("Failed to migrate servers.");
    }

    if revision <= 6 {
        info!("Running migration [revision 6 / 2021-07-09]: Add message text index.");

        db.db()
            .run_command(
                doc! {
                    "createIndexes": "messages",
                    "indexes": [
                        {
                            "key": {
                                "content": "text"
                            },
                            "name": "content"
                        }
                    ]
                },
                None,
            )
            .await
            .expect("Failed to create message index.");
    }

    if revision <= 7 {
        info!("Running migration [revision 7 / 2021-08-11]: Add message text index.");

        db.db()
            .create_collection("bots", None)
            .await
            .expect("Failed to create bots collection.");
    }

    if revision <= 8 {
        info!("Running migration [revision 8 / 2021-09-10]: Update to Authifier version 1.");

        db.db()
            .run_command(
                doc! {
                    "dropIndexes": "accounts",
                    "index": ["email", "email_normalised"]
                },
                None,
            )
            .await
            .expect("Failed to delete legacy account indexes.");

        let col = db.col::<Document>("sessions");
        let mut cursor = db
            .col::<Document>("accounts")
            .find(doc! {}, None)
            .await
            .unwrap();

        while let Some(doc) = cursor.next().await {
            if let Ok(account) = doc {
                let id = account.get_str("_id").unwrap();
                if let Some(sessions) = account.get("sessions") {
                    #[derive(Deserialize)]
                    struct Session {
                        id: String,
                        token: String,
                        friendly_name: String,
                        subscription: Option<Document>,
                    }

                    let sessions = from_bson::<Vec<Session>>(sessions.clone()).unwrap();
                    for session in sessions {
                        info!("Converting session {} to new format.", &session.id);

                        let mut doc = doc! {
                            "_id": session.id,
                            "token": session.token,
                            "user_id": id,
                            "name": session.friendly_name,
                        };

                        if let Some(sub) = session.subscription {
                            doc.insert("subscription", sub);
                        }

                        col.insert_one(doc, None).await.ok();
                    }
                } else {
                    info!("Account doesn't have any sessions!");
                }
            }
        }

        db.col::<Document>("accounts")
            .update_many(
                doc! {},
                doc! {
                    "$unset": {
                        "sessions": 1_i32,
                    },
                    "$set": {
                        "mfa": {
                            "recovery_codes": []
                        }
                    }
                },
                None,
            )
            .await
            .unwrap();
    }

    if revision <= 9 {
        info!("Running migration [revision 9 / 2021-09-14]: Switch from last_message to last_message_id.");

        let mut cursor = db
            .col::<Document>("channels")
            .find(doc! {}, None)
            .await
            .unwrap();

        while let Some(doc) = cursor.next().await {
            if let Ok(channel) = doc {
                let channel_id = channel.get_str("_id").unwrap();
                if let Some(last_message) = channel.get("last_message") {
                    #[derive(Serialize, Deserialize, Debug, Clone)]
                    pub struct Obj {
                        #[serde(rename = "_id")]
                        id: String,
                    }

                    #[derive(Serialize, Deserialize, Debug, Clone)]
                    #[serde(untagged)]
                    pub enum LastMessage {
                        Obj(Obj),
                        Id(String),
                    }

                    let lm = from_bson::<LastMessage>(last_message.clone()).unwrap();
                    let id = match lm {
                        LastMessage::Obj(Obj { id }) => id,
                        LastMessage::Id(id) => id,
                    };

                    info!("Converting session {} to new format.", &channel_id);
                    db.col::<Document>("channels")
                        .update_one(
                            doc! {
                                "_id": channel_id
                            },
                            doc! {
                                "$set": {
                                    "last_message_id": id
                                },
                                "$unset": {
                                    "last_message": 1_i32,
                                }
                            },
                            None,
                        )
                        .await
                        .unwrap();
                } else {
                    info!("{} has no last_message.", &channel_id);
                }
            }
        }
    }

    if revision <= 10 {
        info!("Running migration [revision 10 / 2021-11-01]: Remove nonce values on channels and servers.");

        db.col::<Document>("servers")
            .update_many(
                doc! {},
                doc! {
                    "$unset": {
                        "nonce": 1_i32,
                    }
                },
                None,
            )
            .await
            .unwrap();

        db.col::<Document>("channels")
            .update_many(
                doc! {},
                doc! {
                    "$unset": {
                        "nonce": 1_i32,
                    }
                },
                None,
            )
            .await
            .unwrap();
    }

    if revision <= 11 {
        info!("Running migration [revision 11 / 2021-11-14]: Add indexes to database.");

        db.db()
            .run_command(
                doc! {
                    "createIndexes": "messages",
                    "indexes": [
                        {
                            "key": {
                                "channel": 1_i32
                            },
                            "name": "channel"
                        }
                    ]
                },
                None,
            )
            .await
            .expect("Failed to create message index.");

        db.db()
            .run_command(
                doc! {
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
                },
                None,
            )
            .await
            .expect("Failed to create channel_unreads index.");

        db.db()
            .run_command(
                doc! {
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
                },
                None,
            )
            .await
            .expect("Failed to create server_members index.");
    }

    if revision <= 12 {
        info!("Running migration [revision 12 / 2021-11-21]: Add indexes to database.");

        db.db()
            .run_command(
                doc! {
                    "createIndexes": "messages",
                    "indexes": [
                        {
                            "key": {
                                "channel": 1_i32,
                                "_id": 1_i32
                            },
                            "name": "channel_id_compound"
                        }
                    ]
                },
                None,
            )
            .await
            .expect("Failed to create message index.");
    }

    if revision <= 13 {
        info!("Running migration [revision 13 / 22-02-2022]: Wipe legacy permission values.");

        warn!("This is a destructive operation and will wipe existing permission data (excl. defaults for SendMessage).");
        warn!("Taking a backup is advised.");
        warn!("Continuing in 10 seconds...");
        async_std::task::sleep(Duration::from_secs(10)).await;

        let servers = db.col::<Document>("servers");
        let mut cursor = servers.find(doc! {}, None).await.unwrap();

        while let Some(Ok(mut document)) = cursor.next().await {
            let id = document.get_str("_id").unwrap().to_string();
            info!("Updating server {id}");

            let mut update = doc! {};

            // Try to pluck channel permission SendMessage (0x2)
            // Structure of default_permissions used to be [server, channel]
            let has_send = document
                .get_array("default_permissions")
                .map(|x| {
                    x.get(1)
                        .map(|x| x.as_i32().map(|x| (x as u32 & 0x2) == 0x2))
                })
                .ok()
                .flatten()
                .flatten()
                .unwrap_or_default();

            update.insert(
                "default_permissions",
                // Remove Send Message permission if it wasn't originally granted
                (4000323584).bitxor(if has_send { 0 } else { (1 << 22) as u64 }) as i64,
            );

            if let Some(Bson::Document(mut roles)) = document.remove("roles") {
                for role in roles.keys().cloned().collect::<Vec<String>>() {
                    if let Some(Bson::Document(role)) = roles.get_mut(role) {
                        role.insert(
                            "permissions",
                            doc! {
                                "a": 0_i64,
                                "d": 0_i64,
                            },
                        );
                    }
                }

                update.insert("roles", roles);
            }

            servers
                .update_one(doc! { "_id": id }, doc! { "$set": update }, None)
                .await
                .unwrap();
        }

        let channels = db.col::<Document>("channels");
        let mut cursor = channels.find(doc! {}, None).await.unwrap();

        while let Some(Ok(document)) = cursor.next().await {
            let id = document.get_str("_id").unwrap().to_string();
            info!("Updating channel {id}");

            let mut unset = doc! {
                "permissions": 1_i32,
                "role_permissions": 1_i32,
            };

            // Try to pluck channel permission SendMessage (0x2)
            let has_send = document
                .get_i32("default_permissions")
                .map(|x| (x as u32 & 0x2) == 0x2)
                .unwrap_or(true);

            if has_send {
                // Let parent permissions fall through.
                unset.insert("default_permissions", 1_i32);
            }

            let mut update = doc! {
                "$unset": unset
            };

            if !has_send {
                // Block send message permission.
                update.insert(
                    "$set",
                    doc! {
                        "default_permissions": {
                            "a": 0_i64,
                            "d": (1 << 22) as i64
                        }
                    },
                );
            }

            channels
                .update_one(doc! { "_id": id }, update, None)
                .await
                .unwrap();
        }
    }

    if revision <= 14 {
        info!("Running migration [revision 14 / 21-04-2022]: Split content into content and system fields.");

        db.col::<Document>("messages")
            .update_many(
                doc! {
                    "content": {
                        "$type": "object"
                    }
                },
                doc! {
                    "$rename": {
                        "content": "system"
                    }
                },
                None,
            )
            .await
            .unwrap();
    }

    if revision <= 15 {
        info!("Running migration [revision 15 / 04-06-2022]: Migrate Authifier to latest version.");

        let db = authifier::Database::MongoDb(authifier::database::MongoDb(db.db()));
        db.run_migration(authifier::Migration::M2022_06_03EnsureUpToSpec)
            .await
            .unwrap();
    }

    if revision <= 16 {
        info!("Running migration [revision 16 / 07-07-2022]: Add `emojis` collection and Authifier migration.");

        let authifier_db = authifier::Database::MongoDb(authifier::database::MongoDb(db.db()));
        authifier_db
            .run_migration(authifier::Migration::M2022_06_09AddIndexForDeletion)
            .await
            .unwrap();

        db.db()
            .create_collection("emojis", None)
            .await
            .expect("Failed to create emojis collection.");

        db.db()
            .run_command(
                doc! {
                    "createIndexes": "emojis",
                    "indexes": [
                        {
                            "key": {
                                "parent.id": 1_i32,
                            },
                            "name": "parent_id"
                        }
                    ]
                },
                None,
            )
            .await
            .expect("Failed to create emoji parent index.");
    }

    if revision <= 17 {
        info!("Running migration [revision 17 / 15-07-2022]: Initialise `joined_at` property on server members.");

        db.col::<Document>("server_members")
            .update_many(
                doc! {},
                doc! {
                    "$set": {
                        "joined_at": DateTime::now().to_rfc3339_string()
                    }
                },
                None,
            )
            .await
            .expect("Failed to update server members.");
    }

    if revision <= 18 {
        info!("Running migration [revision 18 / 27-02-2022]: Create author index on messages. Drop plain channel index if exists.");

        if db
            .db()
            .run_command(
                doc! {
                    "dropIndexes": "messages",
                    "index": ["channel"]
                },
                None,
            )
            .await
            .is_err()
        {
            info!("Failed to drop `messages.channel` index but this is ok since that means it's probably gone.");
        }

        db.db()
            .run_command(
                doc! {
                    "createIndexes": "messages",
                    "indexes": [
                        {
                            "key": {
                                "author": 1_i32,
                            },
                            "name": "author"
                        }
                    ]
                },
                None,
            )
            .await
            .expect("Failed to create messages author index.");
    }

    if revision <= 19 {
        info!(
            "Running migration [revision 19 / 27-02-2023]: Create report / snapshot collections."
        );

        db.db()
            .create_collection("safety_reports", None)
            .await
            .unwrap();

        db.db()
            .create_collection("safety_snapshots", None)
            .await
            .unwrap();
    }

    if revision <= 20 {
        info!("Running migration [revision 20 / 28-02-2023]: Add index `snapshot.report_id`.");

        db.db()
            .run_command(
                doc! {
                    "createIndexes": "safety_snapshots",
                    "indexes": [
                        {
                            "key": {
                                "report_id": 1_i32
                            },
                            "name": "report_id"
                        }
                    ]
                },
                None,
            )
            .await
            .expect("Failed to create safety snapshot index.");
    }

    if revision <= 21 {
        info!("Running migration [revision 21 / 31-05-2023]: Add collection `safety_strikes`.");

        db.db()
            .create_collection("safety_strikes", None)
            .await
            .unwrap();
    }

    if revision <= 22 {
        info!("Running migration [revision 22 / 31-05-2023]: Add moderator_id to account strikes.");

        db.col::<Document>("safety_strikes")
            .update_many(
                doc! {},
                doc! {
                    "$set": {
                        "moderator_id": "01EX2NCWQ0CHS3QJF0FEQS1GR4"
                    }
                },
                None,
            )
            .await
            .expect("Failed to update server members.");
    }

    if revision <= 23 {
        info!("Running migration [revision 23 / 10-06-2023]: Generate discriminators for users.");

        db.db()
            .run_command(
                doc! {
                    "dropIndexes": "users",
                    "index": "username"
                },
                None,
            )
            .await
            .expect("Failed to drop existing username index.");

        #[derive(Serialize, Deserialize)]
        struct UserInformation {
            #[serde(rename = "_id")]
            id: String,
            username: String,
        }

        let re_username = regex::Regex::new(r"^(\p{L}|[\d_.-])+$").unwrap();

        let users: Vec<UserInformation> = db
            .col::<UserInformation>("users")
            .find(doc! {}, None)
            .await
            .unwrap()
            .map(|doc| doc.expect("id and username"))
            .collect()
            .await;

        let search_space: Vec<String> = DISCRIMINATOR_SEARCH_SPACE.iter().cloned().collect();
        let mut claimed: HashSet<String> = HashSet::new();

        for i in 0..users.len() {
            let info = &users[i];
            let mut discriminator = {
                let mut rng = rand::thread_rng();
                search_space.choose(&mut rng).unwrap()
            };

            if re_username.is_match(&info.username) {
                while claimed.contains(&format!("{}#{}", info.username, discriminator)) {
                    let new_discriminator = {
                        let mut rng = rand::thread_rng();
                        search_space.choose(&mut rng).unwrap()
                    };

                    info!(
                        "Re-rolled {} to {new_discriminator} from {discriminator}",
                        info.username
                    );

                    discriminator = new_discriminator;
                }

                claimed.insert(format!("{}#{}", info.username, discriminator));

                info!(
                    "({}/{}) Migrating user \"{}\" to #{} - compliant",
                    i + 1,
                    users.len(),
                    info.username,
                    discriminator
                );

                db.col::<UserInformation>("users")
                    .update_one(
                        doc! {
                            "_id": &info.id
                        },
                        doc! {
                            "$set": {
                                "discriminator": discriminator
                            }
                        },
                        None,
                    )
                    .await
                    .unwrap();
            } else {
                let mut sanitised = info
                    .username
                    .graphemes(true)
                    .filter(|s| re_username.is_match(s))
                    .collect::<String>();

                while sanitised.len() < 2 {
                    sanitised += "_";
                }

                while claimed.contains(&format!("{}#{}", sanitised, discriminator)) {
                    let new_discriminator = {
                        let mut rng = rand::thread_rng();
                        search_space.choose(&mut rng).unwrap()
                    };

                    info!("Re-rolled {sanitised} to {new_discriminator} from {discriminator}");
                    discriminator = new_discriminator;
                }

                claimed.insert(format!("{}#{}", sanitised, discriminator));

                info!(
                    "({}/{}) Migrating user \"{}\" to #{} - sanitised: \"{}\"",
                    i + 1,
                    users.len(),
                    info.username,
                    discriminator,
                    sanitised
                );

                db.col::<UserInformation>("users")
                    .update_one(
                        doc! {
                            "_id": &info.id
                        },
                        doc! {
                            "$set": {
                                "username": sanitised,
                                "discriminator": discriminator,
                                "display_name": &info.username
                            }
                        },
                        None,
                    )
                    .await
                    .unwrap();
            }
        }
    }

    if revision <= 24 {
        info!("Running migration [revision 24 / 09-06-2023]: Add collection `channel_webhooks` if not exists, update users index.");

        db.db()
            .create_collection("channel_webhooks", None)
            .await
            .ok();

        db.db()
            .run_command(
                doc! {
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
                },
                None,
            )
            .await
            .expect("Failed to create username index.");
    };

    if revision <= 25 {
        info!("Running migration [revision 25 / 11-06-2023]: Add permissions to webhooks.");

        db.col::<Document>("webhooks")
            .update_many(
                doc! {},
                doc! {
                    "$set": {
                        "permissions": *DEFAULT_WEBHOOK_PERMISSIONS as i64
                    }
                },
                None,
            )
            .await
            .expect("Failed to update webhooks.");
    }

    if revision <= 25 {
        info!("Running migration [revision 25 / 15-06-2023]: Add collection `ratelimit_events` with index.");

        db.db()
            .create_collection("ratelimit_events", None)
            .await
            .ok();

        db.db()
            .run_command(
                doc! {
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
                },
                None,
            )
            .await
            .expect("Failed to create ratelimit_events index.");
    }

    if revision <= 26 {
        info!("Running migration [revision 26 / 17-04-2024]: Add `can_publish` and `can_receive` to members");

        db.col::<Document>("server_members")
            .update_many(
                doc! {},
                doc! {
                    "$set": {
                        "can_publish": true,
                        "can_receive": true
                    }
                },
                None
            )
            .await
            .expect("Failed to update members");
    }

    // Need to migrate fields on attachments, change `user_id`, `object_id`, etc to `parent`.

    // Reminder to update LATEST_REVISION when adding new migrations.
    LATEST_REVISION.max(revision)
}
