use std::env;
use std::net::{IpAddr, Ipv4Addr};
use std::vec::Vec;

use curio_lib::test_producer::TestProducer;
use curio_lib::types::messages::NotificationMessage;
use curio_lib::types::messages::ValidateEmail;

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

    let test_producer = TestProducer::new(ip, port);
    test_producer
        .run(NotificationMessage::ValidateEmail(ValidateEmail {
            user_name: "Andrew".to_owned(),
            user_email: "andrewfromtn@protonmail.com".to_owned(),
            token: "1234xyz".to_owned(),
        }))
        .await;
}
