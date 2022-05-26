use std::env;
use std::net::{IpAddr, Ipv4Addr};
use std::vec::Vec;

use curio_lib::test_consumer::TestConsumer;
use curio_lib::types::consumer::ConsumerTypes;
use curio_lib::types::messages::AuctionOutbid;
use curio_lib::types::messages::NotificationMessage;

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

    let mut test_consumer = TestConsumer::new(ip, port, ConsumerTypes::SMS);
    test_consumer
        .init_with_server(NotificationMessage::AuctionOutbid(AuctionOutbid::default()))
        .await;
    test_consumer.read_messages().await;
}
