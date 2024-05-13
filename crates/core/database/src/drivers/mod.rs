mod mongodb;
mod reference;

use rand::Rng;
use revolt_config::config;

pub use self::mongodb::*;
pub use self::reference::*;

/// Database information to use to create a client
pub enum DatabaseInfo {
    /// Auto-detect the database in use
    Auto,
    /// Auto-detect the database in use and create an empty testing database
    Test(String),
    /// Use the mock database
    Reference,
    /// Connect to MongoDB
    MongoDb { uri: String, database_name: String },
    /// Use existing MongoDB connection
    MongoDbFromClient(::mongodb::Client, String),
}

/// Database
#[derive(Clone, Debug)]
pub enum Database {
    /// Mock database
    Reference(ReferenceDb),
    /// MongoDB database
    MongoDb(MongoDb),
}

impl DatabaseInfo {
    /// Create a database client from the given database information
    #[async_recursion]
    pub async fn connect(self) -> Result<Database, String> {
        let config = config().await;
        println!("{config:?}");
        Ok(match self {
            DatabaseInfo::Auto => {
                if std::env::var("TEST_DB").is_ok() {
                    DatabaseInfo::Test(format!(
                        "revolt_test_{}",
                        rand::thread_rng().gen_range(1_000_000..10_000_000)
                    ))
                    .connect()
                    .await?
                } else if !config.database.mongodb.is_empty() {
                    DatabaseInfo::MongoDb {
                        uri: config.database.mongodb,
                        database_name: "revolt".to_string(),
                    }
                    .connect()
                    .await?
                } else {
                    DatabaseInfo::Reference.connect().await?
                }
            }
            DatabaseInfo::Test(database_name) => {
                match std::env::var("TEST_DB")
                    .expect("`TEST_DB` environment variable should be set to REFERENCE or MONGODB")
                    .as_str()
                {
                    "REFERENCE" => DatabaseInfo::Reference.connect().await?,
                    "MONGODB" => {
                        DatabaseInfo::MongoDb {
                            uri: config.database.mongodb,
                            database_name,
                        }
                        .connect()
                        .await?
                    }
                    _ => unreachable!("must specify REFERENCE or MONGODB"),
                }
            }
            DatabaseInfo::Reference => Database::Reference(Default::default()),
            DatabaseInfo::MongoDb { uri, database_name } => {
                let client = ::mongodb::Client::with_uri_str(uri)
                    .await
                    .map_err(|_| "Failed to init db connection.".to_string())?;

                Database::MongoDb(MongoDb(client, database_name))
            }
            DatabaseInfo::MongoDbFromClient(client, database_name) => {
                Database::MongoDb(MongoDb(client, database_name))
            }
        })
    }
}
