# Curio Connect

## What is this?
I built this app as a basic message broker service for a related project. We ended up not using it and went with a proven approach such as Amazon SNS. Related repositories:

* [Curio Lib - Library Files](https://github.com/AndrewFromTN/curio-lib)
* [Curio SMTP - Email Notifications Sender](https://github.com/AndrewFromTN/curio-smtp)
* [Curio Websocket - Web Notifications Sender](https://github.com/AndrewFromTN/curio-websocket)

## Setup
1. Download Rust via RustUp from: https://www.rust-lang.org/learn/get-started
2. Run RustUp - This will require Visual Studio build tools

## Running
1. Run the cargo command for what you want to do: 'cargo build --bin server', 'cargo run --bin test_client', etc.