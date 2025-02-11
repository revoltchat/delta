#[macro_use]
extern crate log;

use amqprs::{
    channel::{
        BasicConsumeArguments, Channel, ExchangeDeclareArguments, QueueBindArguments,
        QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::AsyncConsumer,
    FieldTable,
};
use revolt_config::{config, Settings};
use tokio::sync::Notify;

mod consumers;
use consumers::{
    inbound::{
        ack::AckConsumer, fr_accepted::FRAcceptedConsumer, fr_received::FRReceivedConsumer,
        generic::GenericConsumer, mass_mention::MassMessageConsumer, message::MessageConsumer,
    },
    outbound::{apn::ApnsOutboundConsumer, fcm::FcmOutboundConsumer, vapid::VapidOutboundConsumer},
};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let config = config().await;
    pretty_env_logger::init();

    // Setup database
    let db = revolt_database::DatabaseInfo::Auto.connect().await.unwrap();
    let authifier: authifier::Database;

    if let Some(client) = match &db {
        revolt_database::Database::Reference(_) => None,
        revolt_database::Database::MongoDb(mongo) => Some(mongo),
    } {
        authifier =
            authifier::Database::MongoDb(authifier::database::MongoDb(client.database("revolt")));
    } else {
        panic!("Mongo is not in use, can't connect via authifier!")
    }

    let mut connections: Vec<(Channel, Connection)> = Vec::new();

    // An explainer of how this works:
    // The inbound connections are on separate routing keys, such that they only receive the proper payload
    // from their respective api (prod or test).
    // However, the outbound queues that go to the services are routed to receive from both, so that messages
    // sent from beta are still notified on prod, and vice versa.

    // This'll require some interesting shimming if we need to add more events once this is in prod (different payloads between prod and test),
    // but that sounds like a problem for future us.

    // inbound: generic
    connections.push(
        make_queue_and_consume(
            &config,
            &config.pushd.generic_queue,
            config.pushd.get_generic_routing_key().as_str(),
            None,
            GenericConsumer::new(db.clone(), authifier.clone()),
        )
        .await,
    );

    // inbound: messages
    connections.push(
        make_queue_and_consume(
            &config,
            &config.pushd.message_queue,
            config.pushd.get_message_routing_key().as_str(),
            None,
            MessageConsumer::new(db.clone(), authifier.clone()),
        )
        .await,
    );

    // inbound: FR received
    connections.push(
        make_queue_and_consume(
            &config,
            &config.pushd.fr_received_queue,
            config.pushd.get_fr_received_routing_key().as_str(),
            None,
            FRReceivedConsumer::new(db.clone(), authifier.clone()),
        )
        .await,
    );

    // inbound: FR accepted
    connections.push(
        make_queue_and_consume(
            &config,
            &config.pushd.fr_accepted_queue,
            config.pushd.get_fr_accepted_routing_key().as_str(),
            None,
            FRAcceptedConsumer::new(db.clone(), authifier.clone()),
        )
        .await,
    );

    connections.push(
        make_queue_and_consume(
            &config,
            &config.pushd.mass_mention_queue,
            config.pushd.get_mass_mention_routing_key().as_str(),
            None,
            MassMessageConsumer::new(db.clone(), authifier.clone()),
        )
        .await,
    );

    if !config.pushd.apn.pkcs8.is_empty() {
        connections.push(
            make_queue_and_consume(
                &config,
                &config.pushd.apn.queue,
                &config.pushd.apn.queue,
                None,
                ApnsOutboundConsumer::new(db.clone()).await.unwrap(),
            )
            .await,
        );

        let mut table = FieldTable::new();
        table.insert("x-message-deduplication".try_into().unwrap(), "true".into());

        connections.push(
            make_queue_and_consume(
                &config,
                &config.pushd.ack_queue,
                &config.pushd.ack_queue,
                Some(table),
                AckConsumer::new(db.clone(), authifier.clone()),
            )
            .await,
        );
    }

    if !config.pushd.fcm.auth_uri.is_empty() {
        connections.push(
            make_queue_and_consume(
                &config,
                &config.pushd.fcm.queue,
                &config.pushd.fcm.queue,
                None,
                FcmOutboundConsumer::new(db.clone()).await.unwrap(),
            )
            .await,
        )
    }

    if !config.pushd.vapid.public_key.is_empty() {
        connections.push(
            make_queue_and_consume(
                &config,
                &config.pushd.vapid.queue,
                &config.pushd.vapid.queue,
                None,
                VapidOutboundConsumer::new(db.clone()).await.unwrap(),
            )
            .await,
        )
    }

    let guard = Notify::new();
    guard.notified().await;

    for (channel, conn) in connections {
        channel.close().await.expect("Unable to close channel");
        conn.close().await.expect("Unable to close connection");
    }
}

async fn make_queue_and_consume<F>(
    config: &Settings,
    queue_name: &str,
    routing_key: &str,
    queue_args: Option<FieldTable>,
    consumer: F,
) -> (Channel, Connection)
where
    F: AsyncConsumer + Send + 'static,
{
    let connection = Connection::open(&OpenConnectionArguments::new(
        &config.rabbit.host,
        config.rabbit.port,
        &config.rabbit.username,
        &config.rabbit.password,
    ))
    .await
    .unwrap();

    let channel = connection.open_channel(None).await.unwrap();

    channel
        .exchange_declare(
            ExchangeDeclareArguments::new(&config.pushd.exchange, "direct")
                .durable(true)
                .finish(),
        )
        .await
        .expect("Failed to declare pushd exchange");

    let mut queue_name = queue_name.to_string();

    if config.pushd.production {
        queue_name += "-prd";
    } else {
        queue_name += "-tst";
    }

    let queue_name = queue_name.as_str();

    let mut args = QueueDeclareArguments::new(queue_name);
    args.durable(true);

    if let Some(arg) = queue_args {
        args.arguments(arg);
    }

    let args = args.finish();
    _ = channel.queue_declare(args).await.unwrap().unwrap();

    channel
        .queue_bind(QueueBindArguments::new(
            queue_name,
            &config.pushd.exchange,
            routing_key,
        ))
        .await
        .expect(
            "This probably means the revolt.notifications exchange does not exist in rabbitmq!",
        );

    let args = BasicConsumeArguments::new(queue_name, "")
        .manual_ack(false)
        .finish();

    let routing_key = channel.basic_consume(consumer, args).await.unwrap();
    info!(
        "Consuming routing key {} as queue {}, tag {}",
        routing_key, queue_name, routing_key
    );
    (channel, connection)
}
