use std::env;
use std::net::{IpAddr, Ipv4Addr};
use std::vec::Vec;

mod server;

#[tokio::main]
pub async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let mut port: u16 = 6789;
    if args.len() == 3 {
        if !args[1].is_empty() {
            ip = args[1]
                .parse::<IpAddr>()
                .expect("Arg 1 is not a valid IP address.");
        }

        if !args[2].is_empty() {
            port = args[2].parse::<u16>().expect("Arg 2 is not a valid port.");
        }
    }

    let server = server::Server::new(ip, port);
    server.run().await;
}

#[cfg(test)]
mod server_tests {
    use super::*;
    use curio_lib::test_consumer::TestConsumer;
    use curio_lib::test_producer::TestProducer;
    use curio_lib::types::consumer::ConsumerTypes;
    use curio_lib::types::messages::AuctionOutbid;
    use curio_lib::types::messages::AuctionUpdate;
    use curio_lib::types::messages::NotificationMessage;

    use tokio::time::*;

    #[tokio::test]
    async fn test_producer_consumer() {
        let server_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let server_port: u16 = 6789;
        let test_message = NotificationMessage::AuctionOutbid(AuctionOutbid {
            public_auction_id: "".to_owned(),
            price: 10.0,
            seconds_left: 120.0,
            outbidder: "AndrewFromTN".to_owned(),
            outbidee_email: "andrew@curiocity.io".to_owned(),
        });

        tokio::spawn(async move {
            let server = server::Server::new(server_ip, server_port);
            server.run().await;
        });

        delay_for(Duration::from_secs(2)).await;

        let cloned_test_message = test_message.clone();
        let consumer_producer_handle = tokio::spawn(async move {
            let mut consumer = TestConsumer::new(server_ip, server_port, ConsumerTypes::SMS);
            consumer
                .init_with_server(NotificationMessage::AuctionOutbid(AuctionOutbid::default()))
                .await;

            let consumer_read_delay_handle = tokio::spawn(async move {
                delay_for(Duration::from_secs(10)).await;
            });

            tokio::spawn(async move {
                let producer = TestProducer::new(server_ip, server_port);
                producer.run(test_message).await;
            });

            tokio::select! {
                _ = consumer_read_delay_handle => {
                    assert_eq!(consumer.last_message_content.unwrap(), cloned_test_message);
                },
                _ = consumer.read_messages() => {}
            }
        });

        consumer_producer_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_consumer_does_not_receive() {
        let server_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let server_port: u16 = 6789;
        let test_message = NotificationMessage::AuctionOutbid(AuctionOutbid {
            public_auction_id: "".to_owned(),
            price: 10.0,
            seconds_left: 120.0,
            outbidder: "AndrewFromTN".to_owned(),
            outbidee_email: "andrew@curiocity.io".to_owned(),
        });

        tokio::spawn(async move {
            let server = server::Server::new(server_ip, server_port);
            server.run().await;
        });

        delay_for(Duration::from_secs(2)).await;

        let consumer_producer_handle = tokio::spawn(async move {
            let mut consumer = TestConsumer::new(server_ip, server_port, ConsumerTypes::SMS);
            consumer
                .init_with_server(NotificationMessage::AuctionUpdate(AuctionUpdate::default()))
                .await;

            let consumer_read_delay_handle = tokio::spawn(async move {
                delay_for(Duration::from_secs(10)).await;
            });

            tokio::spawn(async move {
                let producer = TestProducer::new(server_ip, server_port);
                producer.run(test_message).await;
            });

            tokio::select! {
                _ = consumer_read_delay_handle => {
                    assert_eq!(consumer.last_message_content, None);
                },
                _ = consumer.read_messages() => {}
            }
        });

        consumer_producer_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_consumers_receive() {
        let server_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let server_port: u16 = 6789;
        let test_message = NotificationMessage::AuctionOutbid(AuctionOutbid {
            public_auction_id: "".to_owned(),
            price: 10.0,
            seconds_left: 120.0,
            outbidder: "AndrewFromTN".to_owned(),
            outbidee_email: "andrew@curiocity.io".to_owned(),
        });

        tokio::spawn(async move {
            let server = server::Server::new(server_ip, server_port);
            server.run().await;
        });

        delay_for(Duration::from_secs(2)).await;

        let cloned_test_message = test_message.clone();
        let consumer_producer_handle = tokio::spawn(async move {
            let mut consumer = TestConsumer::new(server_ip, server_port, ConsumerTypes::SMS);
            consumer
                .init_with_server(NotificationMessage::AuctionOutbid(AuctionOutbid::default()))
                .await;

            let mut consumer_2 = TestConsumer::new(server_ip, server_port, ConsumerTypes::Email);
            consumer_2
                .init_with_server(NotificationMessage::AuctionOutbid(AuctionOutbid::default()))
                .await;

            let consumer_read_delay_handle = tokio::spawn(async move {
                delay_for(Duration::from_secs(10)).await;
            });

            tokio::spawn(async move {
                let producer = TestProducer::new(server_ip, server_port);
                producer.run(test_message).await;
            });

            tokio::select! {
                _ = consumer_read_delay_handle => {
                    assert_eq!(consumer.last_message_content.unwrap(), cloned_test_message);
                    assert_eq!(consumer_2.last_message_content.unwrap(), cloned_test_message);
                },
                _ = consumer.read_messages() => {},
                _ = consumer_2.read_messages() => {},
            }
        });

        consumer_producer_handle.await.unwrap();
    }

    //ToDo(andrew): Write backup service test
}
