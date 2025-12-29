use std::error::Error;

// #![allow(
//     clippy::too_many_arguments,
//     http://crt.r2m03.amazontrust.com/r2m03.cer
// )]
// mod midi_ports;
mod engine;
mod models;
mod ui;




fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting app");
    ui::run().map_err(|err| Box::new(err) as Box<dyn Error>)
}
