use futures::prelude::*;
use std::collections::HashMap;
use std::mem;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::vec::Vec;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_serde::formats::*;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

use curio_lib::types::application::ConnectedApplication;
use curio_lib::types::application::ConnectedApplicationType;
use curio_lib::types::messages::ConnectMessage;
use curio_lib::types::messages::ConsumerSubscriptionMessage;
use curio_lib::types::messages::NotificationMessage;

type Subscribers = Arc<RwLock<HashMap<ConnectedApplicationType, Vec<String>>>>;

pub struct Server {
    ip: IpAddr,
    port: u16,
    subscribers: Subscribers,
}

impl Server {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Server {
            ip,
            port,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&self) {
        let mut listener = TcpListener::bind(SocketAddr::from((self.ip, self.port)))
            .await
            .expect("Failed to create the listener.");

        let (tx, _rx) = broadcast::channel::<NotificationMessage>(32);

        let mut s = listener.incoming();
        while let Some(mut socket) = s.try_next().await.unwrap() {
            let mut length_delimited = FramedRead::new(&mut socket, LengthDelimitedCodec::new());
            let mut deserialized = tokio_serde::SymmetricallyFramed::new(
                &mut length_delimited,
                SymmetricalJson::<ConnectMessage>::default(),
            );

            match deserialized.try_next().await {
                Ok(wrapped_message) => {
                    if let Some(msg) = wrapped_message {
                        let app_str: String = (&msg.source_app).into();
                        println!("Client connected: {}", app_str);

                        let is_consumer = match &msg.source_app {
                            ConnectedApplicationType::Consumer(_) => true,
                            _ => false,
                        };

                        let my_tx = tx.clone();

                        if is_consumer {
                            let mut consumer_subscription_deserialized =
                                tokio_serde::SymmetricallyFramed::new(
                                    &mut length_delimited,
                                    SymmetricalJson::<ConsumerSubscriptionMessage>::default(),
                                );

                            // Read what subscriptions they're asking for
                            match consumer_subscription_deserialized.try_next().await {
                                Ok(wrapped_subscription_message) => {
                                    if let Some(sub_msg) = wrapped_subscription_message {
                                        println!("Received client ({}) subscriptions.", app_str);
                                        let subscriptions = sub_msg.message_subscriptions;

                                        let app_id = Uuid::new_v4().to_string();
                                        let new_app = ConnectedApplication {
                                            app_id: app_id.clone(),
                                            app_type: msg.source_app,
                                            socket,
                                            rx_channel: tx.subscribe(),
                                            tx_channel: my_tx,
                                        };

                                        self.subscribers
                                            .write()
                                            .expect("Subscriber RWLock was poisened!")
                                            .entry(msg.source_app)
                                            .or_insert(Vec::new())
                                            .push(app_id);

                                        // Clone the Arc, essentially give this task a pointer to the list
                                        let cloned_list_handle = self.subscribers.clone();
                                        tokio::spawn(async move {
                                            process_consumer(
                                                new_app,
                                                subscriptions,
                                                cloned_list_handle,
                                            )
                                            .await;
                                        });
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error deserializing a consumer subscription message for {}: {}", app_str, e);
                                }
                            }
                        } else {
                            let app_id = Uuid::new_v4().to_string();
                            let new_app = ConnectedApplication {
                                app_id,
                                app_type: msg.source_app,
                                socket,
                                rx_channel: tx.subscribe(),
                                tx_channel: my_tx,
                            };

                            tokio::spawn(async move {
                                process_producer(new_app).await;
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error deserializing: {}", e);
                    break;
                }
            }
        }
    }
}

// Watch the socket and send out via channel
async fn process_producer(app: ConnectedApplication) {
    let mut socket = app.socket;
    let app_type = app.app_type;
    let my_tx = app.tx_channel;
    let length_delimited = FramedRead::new(&mut socket, LengthDelimitedCodec::new());
    let mut deserialized = tokio_serde::SymmetricallyFramed::new(
        length_delimited,
        SymmetricalJson::<NotificationMessage>::default(),
    );

    // Process incoming TCP messages and send them to subscribers
    while let Some(msg) = deserialized.next().await {
        let s: String = (&app_type).into();
        match msg {
            Ok(msg) => {
                if my_tx.send(msg).is_err() {
                    eprintln!("App ({}) producer error. No active receivers.", s);
                }
            }
            Err(_) => {
                eprintln!("App ({}) producer socket closed.", s);
                eprintln!("{:?}", msg);
                break;
            }
        }
    }
}

// Watch the channel and send out via socket
async fn process_consumer(
    mut app: ConnectedApplication,
    subscriptions: Vec<NotificationMessage>,
    subscribers: Subscribers,
) {
    let _consumer_type = match &app.app_type {
        ConnectedApplicationType::Consumer(consumer) => consumer,
        _ => unreachable!(),
    };

    let socket = app.socket;
    let mut length_delimited = FramedWrite::new(socket, LengthDelimitedCodec::new());
    let mut serialized = tokio_serde::SymmetricallyFramed::new(
        &mut length_delimited,
        SymmetricalJson::<NotificationMessage>::default(),
    );
    loop {
        match app.rx_channel.recv().await {
            Ok(msg) => {
                let is_primary = subscribers
                    .write()
                    .expect("Subscriber RWLock was poisened!")
                    .entry(app.app_type)
                    .or_default()
                    .first()
                    == Some(&app.app_id);

                // Only send to the primary service, not backups
                if is_primary {
                    let sub_iter = subscriptions.iter();
                    for result in sub_iter {
                        // If they are subscribed to this message, send it to them
                        if mem::discriminant(result) == mem::discriminant(&msg) {
                            let s: String = app.app_type.into();
                            serialized
                                .send(msg)
                                .await
                                .unwrap_or_else(|_| panic!("Failed to send to: {}", s));
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let s: String = app.app_type.into();
                if e == broadcast::RecvError::Closed {
                    eprintln!("App ({}) recv channel closed.", s);
                    break;
                }

                eprintln!("App ({}) internal message error: {}", s, e);
            }
        }
    }
}
