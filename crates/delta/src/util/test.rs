use authifier::{
    models::{Account, Session},
    Authifier,
};
use futures::StreamExt;
use rand::Rng;
use redis_kiss::redis::aio::PubSub;
use revolt_database::{events::client::EventV1, Database, User};
use revolt_models::v0;
use rocket::local::asynchronous::Client;

pub struct TestHarness {
    pub client: Client,
    authifier: Authifier,
    pub db: Database,
    sub: PubSub,
    event_buffer: Vec<(String, EventV1)>,
}

impl TestHarness {
    pub async fn new() -> TestHarness {
        dotenvy::dotenv().ok();

        let client = Client::tracked(crate::web().await)
            .await
            .expect("valid rocket instance");

        let mut sub = redis_kiss::open_pubsub_connection()
            .await
            .expect("`PubSub`");

        sub.psubscribe("*").await.unwrap();

        let db = client
            .rocket()
            .state::<Database>()
            .expect("`Database`")
            .clone();

        let authifier = client
            .rocket()
            .state::<Authifier>()
            .expect("`Authifier`")
            .clone();

        TestHarness {
            client,
            authifier,
            db,
            sub,
            event_buffer: vec![],
        }
    }

    pub fn rand_string() -> String {
        let mut rng = rand::thread_rng();
        (&mut rng)
            .sample_iter(rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect()
    }

    pub async fn new_user(&self) -> (Account, Session, User) {
        let account = Account::new(
            &self.authifier,
            format!("{}@revolt.chat", TestHarness::rand_string()),
            "password".to_string(),
            false,
        )
        .await
        .expect("`Account`");

        let session = account
            .create_session(&self.authifier, String::new())
            .await
            .expect("`Session`");

        let user = User::create(
            &self.db,
            TestHarness::rand_string(),
            account.id.to_string(),
            None,
        )
        .await
        .expect("`User`");

        (account, session, user)
    }

    pub async fn wait_for_event<F>(&mut self, topic: &str, predicate: F) -> EventV1
    where
        F: Fn(&EventV1) -> bool,
    {
        for (msg_topic, event) in &self.event_buffer {
            if topic == msg_topic && predicate(event) {
                // does not remove from buffer
                return event.clone();
            }
        }

        let mut stream = self.sub.on_message();
        while let Some(item) = stream.next().await {
            // Not sure what should be done if we get a RedisError
            let item = item.unwrap();

            let msg_topic = item.get_channel_name();
            let payload: EventV1 = redis_kiss::decode_payload(&item).unwrap();

            if topic == msg_topic && predicate(&payload) {
                return payload;
            }

            self.event_buffer.push((msg_topic.to_string(), payload));
        }

        // WARNING: if predicate is never satisfied, this will never return
        //          should add a timeout for events so tests can fail gracefully

        unreachable!()
    }

    pub async fn wait_for_message(&mut self, channel_id: &str) -> v0::Message {
        dbg!(&self.event_buffer);

        match self
            .wait_for_event(channel_id, |event| match event {
                EventV1::Message(v0::Message { channel, .. }) => channel == channel_id,
                _ => false,
            })
            .await
        {
            EventV1::Message(message) => message,
            _ => unreachable!(),
        }
    }
}
