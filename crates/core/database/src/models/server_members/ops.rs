use ::mongodb::SessionCursor;
use revolt_result::Result;

use crate::{FieldsMember, Member, MemberCompositeKey, PartialMember};

mod mongodb;
mod reference;

#[derive(Debug)]
pub enum ChunkedServerMembersGenerator {
    #[cfg(feature = "mongodb")]
    MongoDb {
        session: ::mongodb::ClientSession,
        cursor: Option<SessionCursor<Member>>,
    },

    Reference {
        offset: i32,
        data: Option<Vec<Member>>,
    },
}

impl ChunkedServerMembersGenerator {
    #[cfg(feature = "mongodb")]
    pub fn new_mongo(session: ::mongodb::ClientSession, cursor: SessionCursor<Member>) -> Self {
        ChunkedServerMembersGenerator::MongoDb {
            session,
            cursor: Some(cursor),
        }
    }

    pub fn new_reference(data: Vec<Member>) -> Self {
        ChunkedServerMembersGenerator::Reference {
            offset: 0,
            data: Some(data),
        }
    }

    pub async fn next(&mut self) -> Option<Member> {
        match self {
            #[cfg(feature = "mongodb")]
            ChunkedServerMembersGenerator::MongoDb { session, cursor } => {
                if let Some(cursor) = cursor {
                    let value = cursor.next(session).await;
                    value.map(|val| val.expect("Faild to fetch the next member"))
                } else {
                    warn!("Attempted to access a (MongoDb) server member generator without first setting a cursor");
                    None
                }
            }
            ChunkedServerMembersGenerator::Reference { offset, data } => {
                if let Some(data) = data {
                    if data.len() as i32 >= *offset {
                        None
                    } else {
                        let resp = &data[*offset as usize];
                        *offset += 1;
                        Some(resp.clone())
                    }
                } else {
                    warn!("Attempted to access a (Reference) server member generator without first providing data");
                    None
                }
            }
        }
    }
}

#[async_trait]
pub trait AbstractServerMembers: Sync + Send {
    /// Insert a new server member into the database
    async fn insert_member(&self, member: &Member) -> Result<()>;

    /// Fetch a server member by their id
    async fn fetch_member(&self, server_id: &str, user_id: &str) -> Result<Member>;

    /// Fetch all members in a server
    async fn fetch_all_members<'a>(&self, server_id: &str) -> Result<Vec<Member>>;

    /// Fetch all members in a server as an iterator
    async fn fetch_all_members_chunked(
        &self,
        server_id: &str,
    ) -> Result<ChunkedServerMembersGenerator>;

    async fn fetch_all_members_with_roles(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<Vec<Member>>;

    async fn fetch_all_members_with_roles_chunked(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<ChunkedServerMembersGenerator>;

    /// Fetch all memberships for a user
    async fn fetch_all_memberships<'a>(&self, user_id: &str) -> Result<Vec<Member>>;

    /// Fetch multiple members by their ids
    async fn fetch_members<'a>(&self, server_id: &str, ids: &'a [String]) -> Result<Vec<Member>>;

    /// Fetch member count of a server
    async fn fetch_member_count(&self, server_id: &str) -> Result<usize>;

    /// Fetch server count of a user
    async fn fetch_server_count(&self, user_id: &str) -> Result<usize>;

    /// Update information for a server member
    async fn update_member(
        &self,
        id: &MemberCompositeKey,
        partial: &PartialMember,
        remove: Vec<FieldsMember>,
    ) -> Result<()>;

    /// Delete a server member by their id
    async fn delete_member(&self, id: &MemberCompositeKey) -> Result<()>;
}
