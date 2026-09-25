//! Claude plan limits, read via the Claude Code status line.
//!
//! See [ADR 0005](../../../../docs/adr/0005-claude-plan-limits-source.md) for
//! why this is the chosen source, and
//! [`protocol/providers/claude/`](../../../../protocol/providers/claude/README.md)
//! for the data shape.
//!
//! ## How this works
//!
//! Claude Code runs whatever command `statusLine.command` in
//! `~/.claude/settings.json` names, once per status line update, piping it a
//! JSON object on stdin and printing whatever it writes to stdout. This
//! module implements two sides of that:
//!
//! - [`connect`] points that command at this binary (`<exe> claude-statusline`).
//!   Before doing so, it backs up whatever `statusLine` value was already
//!   there — the user's own, or another tool's — so [`disconnect`] can put it
//!   back exactly.
//! - [`run_bridge`] is what actually runs each time Claude Code calls it: it
//!   reads the plan-limit fields out of the JSON on stdin, saves them for the
//!   agent's UI, and then re-runs whatever command was backed up (forwarding
//!   the same stdin and printing its stdout), so the user's own status line
//!   keeps working unchanged. **It never inspects or depends on what that
//!   backed-up command does** — it's treated as an opaque passthrough,
//!   because it may belong to another application entirely.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const APP_DATA_DIR_NAME: &str = "com.espia.agent";
const PLAN_LIMITS_FILE: &str = "claude_plan_limits.json";
const BACKUP_FILE: &str = "claude_statusline_backup.json";
const BRIDGE_SUBCOMMAND: &str = "claude-statusline";

// ---------------------------------------------------------------------------
// Data shapes
// ---------------------------------------------------------------------------

/// One rate-limit window, as this agent stores it — matches `plan_limits.*`
/// in the protocol's `claude` provider.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlanLimitWindow {
    pub used_pct: f64,
    /// Unix epoch milliseconds. `None` once the window has reset but no
    /// fresh reading has arrived yet — see [`protocol/providers/claude`].
    pub resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlanLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<PlanLimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seven_day: Option<PlanLimitWindow>,
    /// Unix epoch milliseconds when this was last refreshed by
    /// [`run_bridge`] — i.e. the last time Claude Code updated its status
    /// line. Absent until the bridge has run at least once.
    pub updated_at: i64,
}

/// The subset of Claude Code's status line JSON this module reads. Extra
/// fields are ignored, and the whole thing is optional — if Claude Code ever
/// changes this shape, the bridge still runs, just without plan limits.
#[derive(Deserialize, Default)]
struct StatusLineInput {
    #[serde(default)]
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize, Default)]
struct RateLimits {
    five_hour: Option<RateWindow>,
    seven_day: Option<RateWindow>,
}

#[derive(Deserialize)]
struct RateWindow {
    used_percentage: f64,
    /// Unix epoch **seconds**, per Claude Code's status line docs.
    resets_at: i64,
}

/// What the agent's settings UI shows for this provider.
#[derive(Serialize, Clone, Debug)]
pub struct ClaudeStatusLineStatus {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_limits: Option<PlanLimits>,
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn app_data_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or("Could not determine this OS's application data directory")?
        .join(APP_DATA_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

fn plan_limits_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join(PLAN_LIMITS_FILE))
}

fn backup_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join(BACKUP_FILE))
}

fn claude_settings_path() -> Result<PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or("Could not determine the home directory")?
        .join(".claude")
        .join("settings.json"))
}

/// Writes `value` to `path` by writing a sibling temp file and renaming it
/// over the target, so a reader never sees a half-written file.
fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    let contents =
        serde_json::to_string_pretty(value).map_err(|e| format!("Could not encode JSON: {e}"))?;
    fs::write(&tmp_path, contents)
        .map_err(|e| format!("Could not write {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Could not replace {}: {e}", path.display()))?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Plan limits state (written by `run_bridge`, read by the UI)
// ---------------------------------------------------------------------------

fn write_plan_limits(input: &StatusLineInput) -> Result<(), String> {
    let Some(rate_limits) = &input.rate_limits else {
        return Ok(());
    };
    let to_window = |w: &RateWindow| PlanLimitWindow {
        used_pct: w.used_percentage,
        resets_at: Some(w.resets_at.saturating_mul(1000)),
    };
    let limits = PlanLimits {
        five_hour: rate_limits.five_hour.as_ref().map(to_window),
        seven_day: rate_limits.seven_day.as_ref().map(to_window),
        updated_at: now_ms(),
    };
    let path = plan_limits_path()?;
    let value = serde_json::to_value(&limits).map_err(|e| e.to_string())?;
    write_json_atomic(&path, &value)
}

/// Reads the last plan limits [`run_bridge`] collected, if any.
pub fn read_plan_limits() -> Option<PlanLimits> {
    let path = plan_limits_path().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

// ---------------------------------------------------------------------------
// Connect / disconnect (mutates ~/.claude/settings.json)
// ---------------------------------------------------------------------------

/// The previously-installed `statusLine` value, or that there was none.
/// `None` at the outer level means "not connected"; `Some(None)` means
/// "connected, and there was no `statusLine` key before".
fn read_backup() -> Option<Option<Value>> {
    let path = backup_path().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    let parsed: Value = serde_json::from_str(&contents).ok()?;
    let previous = parsed.get("previous_status_line").cloned().unwrap_or(Value::Null);
    // JSON null means "there was no `statusLine` key" (see `connect`), not
    // "the value was `null`" — Claude Code never sets it to that, and
    // collapsing the two here is what lets `disconnect` remove the key
    // outright instead of leaving `"statusLine": null` behind.
    Some(if previous.is_null() { None } else { Some(previous) })
}

fn is_connected() -> bool {
    backup_path().map(|p| p.exists()).unwrap_or(false)
}

#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn bridge_command_string() -> Result<String, String> {
    #[cfg(not(unix))]
    {
        return Err(
            "Connecting Claude Usage isn't implemented on this platform yet — espia targets \
             macOS first. See ADR 0004."
                .to_string(),
        );
    }
    #[cfg(unix)]
    {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Could not determine this program's own path: {e}"))?;
        let exe = exe.to_str().ok_or("This program's path is not valid UTF-8")?;
        Ok(format!("{} {BRIDGE_SUBCOMMAND}", shell_quote(exe)))
    }
}

/// Points `~/.claude/settings.json`'s `statusLine.command` at this binary,
/// backing up whatever was there first. Safe to call again while already
/// connected — it won't back up its own command over the real original.
pub fn connect() -> Result<ClaudeStatusLineStatus, String> {
    let settings_path = claude_settings_path()?;
    if !settings_path.exists() {
        return Err(format!(
            "{} does not exist — is Claude Code installed for this user?",
            settings_path.display()
        ));
    }

    let raw = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Could not read {}: {e}", settings_path.display()))?;
    let mut settings: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Could not parse {}: {e}", settings_path.display()))?;
    let settings_obj = settings
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", settings_path.display()))?;

    if !is_connected() {
        let previous = settings_obj.get("statusLine").cloned().unwrap_or(Value::Null);
        write_json_atomic(&backup_path()?, &json!({ "previous_status_line": previous }))?;
    }

    let bridge_command = bridge_command_string()?;
    let status_line = settings_obj
        .entry("statusLine")
        .or_insert_with(|| json!({}));
    if !status_line.is_object() {
        *status_line = json!({});
    }
    let status_line_obj = status_line.as_object_mut().expect("just ensured this is an object");
    status_line_obj.insert("type".to_string(), json!("command"));
    status_line_obj.insert("command".to_string(), json!(bridge_command));

    write_json_atomic(&settings_path, &settings)?;
    Ok(status())
}

/// Restores the `statusLine` value [`connect`] backed up, and forgets it.
/// Does nothing if not connected.
pub fn disconnect() -> Result<ClaudeStatusLineStatus, String> {
    let Some(previous) = read_backup() else {
        return Ok(status());
    };

    let settings_path = claude_settings_path()?;
    let raw = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Could not read {}: {e}", settings_path.display()))?;
    let mut settings: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Could not parse {}: {e}", settings_path.display()))?;
    let settings_obj = settings
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", settings_path.display()))?;

    match previous {
        Some(value) => {
            settings_obj.insert("statusLine".to_string(), value);
        }
        None => {
            settings_obj.remove("statusLine");
        }
    }
    write_json_atomic(&settings_path, &settings)?;

    if let Ok(path) = backup_path() {
        let _ = fs::remove_file(path);
    }
    if let Ok(path) = plan_limits_path() {
        let _ = fs::remove_file(path);
    }

    Ok(status())
}

pub fn status() -> ClaudeStatusLineStatus {
    ClaudeStatusLineStatus {
        connected: is_connected(),
        plan_limits: read_plan_limits(),
    }
}

// ---------------------------------------------------------------------------
// The bridge itself — invoked as `<this binary> claude-statusline`
// ---------------------------------------------------------------------------

/// Builds the minimal status line text this agent prints when there is no
/// previous command to chain to (nothing was configured before `connect`).
fn fallback_line(limits: &PlanLimits) -> String {
    let mut parts = vec!["espia".to_string()];
    if let Some(w) = &limits.five_hour {
        parts.push(format!("5h {:.0}%", w.used_pct));
    }
    if let Some(w) = &limits.seven_day {
        parts.push(format!("7d {:.0}%", w.used_pct));
    }
    parts.join(" · ")
}

/// Runs the previously-installed status line command, forwarding `stdin` to
/// it and returning its stdout. Its own stderr and exit code are ignored —
/// if it fails, the caller falls back to its own output rather than leaving
/// the status line blank.
fn run_previous_command(command: &str, stdin: &[u8]) -> Option<String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    child.stdin.take()?.write_all(stdin).ok()?;
    let output = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The entry point for `<this binary> claude-statusline`. Returns the
/// process exit code; never panics on malformed input, since a crash here
/// would blank the user's status line.
pub fn run_bridge() -> i32 {
    let mut raw_stdin = Vec::new();
    if std::io::stdin().read_to_end(&mut raw_stdin).is_err() {
        return 0;
    }

    let input: StatusLineInput =
        serde_json::from_slice(&raw_stdin).unwrap_or_else(|_| StatusLineInput::default());
    let _ = write_plan_limits(&input);

    let previous_command = read_backup().flatten().and_then(|v| match v {
        Value::Object(obj) => obj.get("command").and_then(|c| c.as_str()).map(str::to_string),
        _ => None,
    });

    // Guard against ever chaining into ourselves — a bug in `connect`'s
    // idempotency check would otherwise be an infinite loop of the agent
    // launching itself.
    let self_path = std::env::current_exe().ok().and_then(|p| p.to_str().map(str::to_string));
    let previous_command = previous_command.filter(|cmd| match &self_path {
        Some(self_path) => !cmd.contains(self_path.as_str()),
        None => true,
    });

    let output = match previous_command {
        Some(command) => run_previous_command(&command, &raw_stdin),
        None => None,
    };

    let output = output.unwrap_or_else(|| {
        let limits = read_plan_limits().unwrap_or_default();
        fallback_line(&limits)
    });

    print!("{output}");
    0
}

// ---------------------------------------------------------------------------
// Tests
//
// These point `HOME` at a scratch directory (never the real one) so they can
// exercise `connect`/`disconnect`/`run_bridge` — including the actual
// settings.json read-modify-write — without touching a real user's files.
//
// `cargo test` runs tests in parallel by default, but `HOME` is a single
// process-wide variable, so `TestHome` serializes every test that uses it on
// `HOME_ENV_LOCK` — plain `cargo test` is safe to run as-is.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::HOME_ENV_LOCK;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::MutexGuard;

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A scratch `$HOME`, with `.claude/settings.json` seeded with
    /// `initial_status_line` (or no `statusLine` key at all, if `None`).
    /// Removed when dropped, after releasing the `HOME` lock.
    struct TestHome {
        dir: PathBuf,
        _lock: MutexGuard<'static, ()>,
    }

    impl TestHome {
        fn new(initial_status_line: Option<Value>) -> Self {
            // A poisoned lock only means an earlier test panicked while
            // holding it; the env var itself is still fine to reuse.
            let lock = HOME_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir().join(format!("espia-claude-test-{}-{n}", std::process::id()));
            let claude_dir = dir.join(".claude");
            fs::create_dir_all(&claude_dir).unwrap();

            let mut settings = json!({ "model": "sonnet" });
            if let Some(status_line) = initial_status_line {
                settings["statusLine"] = status_line;
            }
            fs::write(
                claude_dir.join("settings.json"),
                serde_json::to_string_pretty(&settings).unwrap(),
            )
            .unwrap();

            // SAFETY: `HOME_ENV_LOCK` ensures only one `TestHome` exists at a
            // time, so no other test observes this process-wide var mid-swap.
            unsafe {
                std::env::set_var("HOME", &dir);
            }
            Self { dir, _lock: lock }
        }

        fn read_settings(&self) -> Value {
            let raw = fs::read_to_string(self.dir.join(".claude/settings.json")).unwrap();
            serde_json::from_str(&raw).unwrap()
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn connect_backs_up_existing_status_line_and_installs_the_bridge() {
        let home = TestHome::new(Some(json!({ "type": "command", "command": "echo hi" })));

        let result = connect().expect("connect should succeed");
        assert!(result.connected);

        let settings = home.read_settings();
        let command = settings["statusLine"]["command"].as_str().unwrap();
        assert!(command.ends_with(&format!(" {BRIDGE_SUBCOMMAND}")), "got: {command}");
        assert_eq!(settings["model"], "sonnet", "unrelated keys must survive untouched");

        let backup = read_backup().expect("should be connected").expect("had a previous value");
        assert_eq!(backup["command"], "echo hi");
    }

    #[test]
    fn connect_backs_up_absence_when_there_was_no_status_line() {
        let _home = TestHome::new(None);

        connect().expect("connect should succeed");

        let backup = read_backup().expect("should be connected");
        assert!(backup.is_none(), "there was no statusLine before, so the backup must say so");
    }

    #[test]
    fn connect_is_idempotent_and_never_backs_up_its_own_command() {
        let _home = TestHome::new(Some(json!({ "type": "command", "command": "echo original" })));

        connect().expect("first connect should succeed");
        connect().expect("second connect should succeed");

        let backup = read_backup().unwrap().unwrap();
        assert_eq!(backup["command"], "echo original", "must still be the real original, not our own bridge command");
    }

    #[test]
    fn disconnect_restores_the_previous_command() {
        let home = TestHome::new(Some(json!({ "type": "command", "command": "echo hi" })));
        connect().expect("connect should succeed");

        let result = disconnect().expect("disconnect should succeed");
        assert!(!result.connected);

        let settings = home.read_settings();
        assert_eq!(settings["statusLine"]["command"], "echo hi");
    }

    #[test]
    fn disconnect_removes_the_key_when_there_was_none_before() {
        let home = TestHome::new(None);
        connect().expect("connect should succeed");

        disconnect().expect("disconnect should succeed");

        let settings = home.read_settings();
        assert!(settings.get("statusLine").is_none());
    }

    #[test]
    fn run_bridge_saves_plan_limits_and_chains_to_the_previous_command() {
        let _home = TestHome::new(Some(json!({
            "type": "command",
            // Echoes stdin back with a prefix, so the test can tell the
            // bridge actually forwarded it rather than swallowing it. Uses
            // `printf`, not `echo -n` — plain `/bin/sh` isn't guaranteed to
            // treat `-n` as a flag rather than literal output.
            "command": "printf 'previous saw: '; cat"
        })));
        connect().expect("connect should succeed");

        let backup = read_backup().unwrap().unwrap();
        let previous_command = backup["command"].as_str().unwrap().to_string();
        let stdin = json!({
            "rate_limits": {
                "five_hour": { "used_percentage": 12.5, "resets_at": 1_800_000_000_i64 },
                "seven_day": { "used_percentage": 40.0, "resets_at": 1_800_500_000_i64 }
            }
        });
        let stdin_bytes = serde_json::to_vec(&stdin).unwrap();

        let output = run_previous_command(&previous_command, &stdin_bytes).expect("command should run");
        assert!(output.starts_with("previous saw: "));
        assert!(output.contains("rate_limits"), "the original stdin must reach the previous command");

        write_plan_limits(&serde_json::from_slice(&stdin_bytes).unwrap()).unwrap();
        let limits = read_plan_limits().expect("plan limits should have been written");
        assert_eq!(limits.five_hour.unwrap().used_pct, 12.5);
        assert_eq!(limits.seven_day.unwrap().used_pct, 40.0);
    }

    #[test]
    fn fallback_line_is_used_when_nothing_to_chain_to() {
        let _home = TestHome::new(None);
        connect().expect("connect should succeed");

        let limits = PlanLimits {
            five_hour: Some(PlanLimitWindow { used_pct: 5.0, resets_at: Some(1) }),
            seven_day: None,
            updated_at: 1,
        };
        let line = fallback_line(&limits);
        assert_eq!(line, "espia · 5h 5%");
    }
}
