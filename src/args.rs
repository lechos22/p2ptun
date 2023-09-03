use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// server port
    #[arg(short, long, default_value_t = 4567)]
    port: u16,
}

impl Args {
    pub fn get_port(&self) -> u16 {
        self.port
    }
}
