use anyhow::Context;
use anyhow::Result;
use kotosiro_sharing::config;
use kotosiro_sharing::logging;
use kotosiro_sharing::server::Server;
use tracing::debug;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let app = clap::Command::new("kotosiro-sharing")
        .author("Shingo OKAWA <shingo.okawa.g.h.c@gmail.com>")
        .version(kotosiro_sharing::VERSION)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("server")
                .about("Launch the server process")
                .after_help("The server implements Delta Sharing REST protocol."),
        );
    logging::setup();
    debug!(
        db_url = config::fetch::<String>("db_url"),
        server_addr = config::fetch::<String>("server_addr"),
        server_bind = config::fetch::<String>("server_bind"),
        use_json_log = config::fetch::<bool>("use_json_log"),
        log_filter = config::fetch::<String>("log_filter"),
    );
    let args = app.get_matches();
    match args.subcommand().expect("subcommand is required") {
        ("server", _args) => {
            info!("kotosiro sharing server is starting");
            let server = Server::new().await.context("failed to create server")?;
            server.start().await.context("failed to start server")
        }
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
