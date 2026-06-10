use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::process::{Command, ExitCode};

const SSH_HOST: &str = "root@YOUR_LINUX_HOST";
const SSH_KEY: &str = "~/.ssh/id_rsa";
const LOCAL_DIR: &str = ".";
const REMOTE_DIR: &str = "/root/pshred-router";

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build and deployment tasks for pshred-router")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Setup,
    Sync,
    Build,
    Run {
        #[arg(short, long, default_value = "eth0")]
        interface: String,
        #[arg(short, long, default_value = "8001")]
        port: u16,
        #[arg(long)]
        skb_mode: bool,
    },
    Ssh,
    Check,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => setup_remote()?,
        Commands::Sync => sync_code()?,
        Commands::Build => {
            sync_code()?;
            build_remote()?;
        }
        Commands::Run {
            interface,
            port,
            skb_mode,
        } => {
            sync_code()?;
            build_remote()?;
            run_remote(&interface, port, skb_mode)?;
        }
        Commands::Ssh => ssh_interactive()?,
        Commands::Check => {
            sync_code()?;
            check_remote()?;
        }
    }

    Ok(())
}

fn setup_remote() -> Result<()> {
    println!("Setting up remote environment...");

    let setup_script = r#"
        set -e
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
        rustup install nightly
        rustup component add rust-src --toolchain nightly
        cargo install bpf-linker
        apt-get update && apt-get install -y llvm clang
        echo "Setup complete!"
    "#;

    ssh_cmd(&["bash", "-c", setup_script])?;
    Ok(())
}

fn sync_code() -> Result<()> {
    println!("Syncing code to remote...");

    let ssh_cmd = format!("ssh -i {SSH_KEY}");
    let dest = format!("{SSH_HOST}:{REMOTE_DIR}");

    let args = [
        "-avz",
        "--delete",
        "-e",
        &ssh_cmd,
        "--exclude=target",
        "--exclude=.git",
        "--exclude=*.o",
        "--exclude=.DS_Store",
        LOCAL_DIR,
        &dest,
    ];

    run_cmd("rsync", &args)?;
    Ok(())
}

fn build_remote() -> Result<()> {
    println!("Building on remote...");

    ssh_cmd(&[
        "bash",
        "-c",
        &format!(
            "cd {REMOTE_DIR} && source ~/.cargo/env && cargo build --release -p pshred-loader"
        ),
    ])?;

    Ok(())
}

fn run_remote(interface: &str, port: u16, skb_mode: bool) -> Result<()> {
    println!("Running on remote (interface={interface}, port={port})...");

    let mut cmd = format!(
        "cd {REMOTE_DIR} && source ~/.cargo/env && \
         RUST_LOG=info cargo run --release -p pshred-loader -- \
         --interface {interface} --port {port}"
    );

    if skb_mode {
        cmd.push_str(" --skb-mode");
    }

    ssh_cmd(&["bash", "-c", &cmd])?;
    Ok(())
}

fn check_remote() -> Result<()> {
    println!("Checking on remote...");

    ssh_cmd(&[
        "bash",
        "-c",
        &format!("cd {REMOTE_DIR} && source ~/.cargo/env && cargo check --workspace"),
    ])?;

    Ok(())
}

fn ssh_interactive() -> Result<()> {
    run_cmd(
        "ssh",
        &[
            "-i",
            SSH_KEY,
            SSH_HOST,
            "-t",
            &format!("cd {REMOTE_DIR}; bash"),
        ],
    )?;
    Ok(())
}

fn ssh_cmd(args: &[&str]) -> Result<()> {
    let mut full_args = vec!["-i", SSH_KEY, SSH_HOST];
    full_args.extend(args);
    run_cmd("ssh", &full_args)
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute {program}"))?;

    if !status.success() {
        bail!("{program} exited with {status}");
    }

    Ok(())
}
