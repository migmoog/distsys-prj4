use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Project4 {
    // Path to the hostsfile
    #[arg(short = 'h')]
    pub hostsfile: PathBuf,

    #[arg(short = 'v')]
    pub proposal_value: Option<char>,

    #[arg(short = 't')]
    pub proposal_delay: Option<u64>,
}
