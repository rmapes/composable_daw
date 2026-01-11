use std::error::Error;

use crate::threads::ui;

mod models;
mod threads;


fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting app");
    ui::run().map_err(|err| Box::new(err) as Box<dyn Error>)
}
