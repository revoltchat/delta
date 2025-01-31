use crate::MongoDb;

use super::AbstractMigrations;

mod init;
mod scripts;

#[async_trait]
impl AbstractMigrations for MongoDb {
    #[cfg(test)]
    /// Drop the database
    async fn drop_database(&self) {
        self.db().drop().await.ok();
    }

    /// Migrate the database
    async fn migrate_database(&self) -> Result<(), ()> {
        info!("Migrating the database.");

        let list = self
            .list_database_names()
            .await
            .expect("Failed to fetch database names.");

        if list.iter().any(|x| x == &self.1) {
            scripts::migrate_database(self).await;
        } else {
            init::create_database(self).await;
        }

        Ok(())
    }
}
