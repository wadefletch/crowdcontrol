use anyhow::Result;
use clap::CommandFactory;
use clap_complete::generate;
use std::io;

use crate::commands::CompletionsArgs;
use crowdcontrol_core::Config;

pub async fn execute(_config: Config, args: CompletionsArgs) -> Result<()> {
    let mut cmd = crate::Cli::command();
    generate(args.shell, &mut cmd, "crowdcontrol", &mut io::stdout());
    Ok(())
}
