// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Claude Code invokes this binary as `<exe> claude-statusline` once
/// `providers::claude::connect` points its status line at it. That path must
/// run and exit immediately — never opening the GUI — since it fires on
/// every status line refresh.
const CLAUDE_STATUSLINE_SUBCOMMAND: &str = "claude-statusline";

fn main() {
    let mut args = std::env::args();
    let _binary_path = args.next();
    if args.next().as_deref() == Some(CLAUDE_STATUSLINE_SUBCOMMAND) {
        std::process::exit(espia_core::providers::claude::run_bridge());
    }
    agent_lib::run()
}
