// Settings UI entry point.
//
// This is a temporary preview: it polls the local metrics collectors
// directly so they can be verified without a paired device. Once the
// WebSocket server exists, the device will receive this data instead of the
// UI polling it — see docs/architecture.md.

import { t, applyTranslations, setCurrentLanguage } from "./i18n.js";
// Vendored from the `tucano` npm package by `npm run vendor` — never edit
// these files directly. See docs/adr/0012-tucano.md.
import { init as initTucano, Select as TucanoSelect, confirm as tucanoConfirm } from "./vendor/tucano/tucano.esm.js";

const { invoke } = window.__TAURI__.core;

// 2s, not 1s: this poll drives the heaviest recurring work in the app
// (sysinfo refresh + a full DOM re-render), and CPU/memory/disk don't
// change fast enough for a 1s cadence to actually add useful precision —
// see docs/adr/0011-lighter-polling.md.
const POLL_INTERVAL_MS = 2000;
const SPARKLINE_SAMPLE_COUNT = 30; // ~1 minute of history at 1 sample/2s

// Status colors, kept in sync with the `@theme` block in src/input.css.
// Green and amber sit close together for deuteranopia (red-green color
// blindness) at this saturation, so severity is always paired with a text
// label too — see temp-status below — never color alone.
const COLOR_BRAND = "#5cec01";
const COLOR_WARNING = "#ff8a00";
const COLOR_CRITICAL = "#ef4444";

const cpuValueEl = document.querySelector("#cpu-value");
const cpuCoreCountEl = document.querySelector("#cpu-core-count");
const cpuRingArcEl = document.querySelector("#cpu-ring-arc");
const cpuSparklineAreaEl = document.querySelector("#cpu-sparkline-area");
const cpuSparklineLineEl = document.querySelector("#cpu-sparkline-line");

const memoryPctValueEl = document.querySelector("#memory-pct-value");
const memoryTotalEl = document.querySelector("#memory-total");
const memoryRingArcEl = document.querySelector("#memory-ring-arc");
const memorySparklineAreaEl = document.querySelector("#memory-sparkline-area");
const memorySparklineLineEl = document.querySelector("#memory-sparkline-line");

const tempValueEl = document.querySelector("#temp-value");
const tempStatusEl = document.querySelector("#temp-status");
const tempBarFillEl = document.querySelector("#temp-bar-fill");
const tempDetailEl = document.querySelector("#temp-detail");
const tempTagEl = document.querySelector("#temp-tag");

const processRowsEl = document.querySelector("#process-rows");

const diskListEl = document.querySelector("#disk-list");
const diskTotalEl = document.querySelector("#disk-total");
const networkRxEl = document.querySelector("#network-rx");
const networkTxEl = document.querySelector("#network-tx");

// The dashboard card only ever shows status + (when connected) the usage
// bars — Connect/Disconnect live in the settings dialog instead, so the
// card isn't the one place doing double duty as data and as controls.
const claudeStatusTagEl = document.querySelector("#claude-status-tag");
const claudeDisconnectedNoteEl = document.querySelector("#claude-disconnected-note");
const claudeConnectedEl = document.querySelector("#claude-connected");
const claudeLimitsEl = document.querySelector("#claude-limits");
const claudeUpdatedEl = document.querySelector("#claude-updated");

const claudeSettingsStatusTagEl = document.querySelector("#claude-settings-status-tag");
const claudeConnectButtonEl = document.querySelector("#claude-connect-button");
const claudeDisconnectButtonEl = document.querySelector("#claude-disconnect-button");
const claudeErrorEl = document.querySelector("#claude-error");

const weatherCityEl = document.querySelector("#weather-city");
const weatherIconEl = document.querySelector("#weather-icon");
const weatherIconShapeEl = document.querySelector("#weather-icon-shape");
const weatherTempEl = document.querySelector("#weather-temp");
const weatherStatusEl = document.querySelector("#weather-status");
const weatherDetailEl = document.querySelector("#weather-detail");

// No open/close refs needed here — the gear button and the dialog itself
// carry `data-tuc-modal`/`data-tuc-modal-close`, which Tucano's own modal
// auto-init (see `initTucano()` below) wires up without our code touching
// it.
const settingsLanguageEl = document.querySelector("#settings-language");
const settingsLocationCurrentEl = document.querySelector("#settings-location-current");
const settingsLocationClearButtonEl = document.querySelector("#settings-location-clear-button");
const settingsLocationErrorEl = document.querySelector("#settings-location-error");

const cpuHistory = [];
const memoryHistory = [];

function formatBytes(bytes) {
  const gigabytes = bytes / 1024 ** 3;
  return `${gigabytes.toFixed(1)} GB`;
}

/// Process memory is usually well under 1 GB, where `formatBytes`'s fixed
/// one-decimal GB reads as "0.1 GB" for everything small; show MB up to 1 GB.
function formatProcessBytes(bytes) {
  const megabytes = bytes / 1024 ** 2;
  if (megabytes < 1024) {
    return `${megabytes.toFixed(0)} MB`;
  }
  return formatBytes(bytes);
}

function formatRate(bytesPerSecond) {
  if (bytesPerSecond < 1024) return `${bytesPerSecond.toFixed(0)} B/s`;
  const kilobytes = bytesPerSecond / 1024;
  if (kilobytes < 1024) return `${kilobytes.toFixed(1)} KB/s`;
  return `${(kilobytes / 1024).toFixed(1)} MB/s`;
}

/// Returns a severity color for a 0–100 usage percentage.
function usageSeverityColor(pct) {
  if (pct >= 90) return COLOR_CRITICAL;
  if (pct >= 70) return COLOR_WARNING;
  return COLOR_BRAND;
}

/// Sets a usage ring's fill (0–100) and color. The ring's `stroke-dasharray`
/// is fixed at 100 via `pathLength="100"` on the SVG element, so the arc is
/// drawn by offsetting it: 100 (nothing drawn) down to 0 (full circle).
function setRing(arcEl, pct, color) {
  const clamped = Math.max(0, Math.min(100, pct));
  arcEl.style.strokeDashoffset = String(100 - clamped);
  arcEl.style.stroke = color;
}

/// Renders a 0–100 usage history as a thin line with a soft area fill
/// underneath, matching the sparkline mark spec: a 2px line, no per-point
/// markers. Shared by the CPU and Memory cards.
function renderSparkline(history, areaEl, lineEl) {
  const width = 240;
  const height = 48;
  const maxSamples = SPARKLINE_SAMPLE_COUNT;
  const points = history.map((value, index) => {
    const x = (index / Math.max(maxSamples - 1, 1)) * width;
    const y = height - (Math.max(0, Math.min(100, value)) / 100) * height;
    return [x, y];
  });

  if (points.length < 2) {
    lineEl.setAttribute("d", "");
    areaEl.setAttribute("d", "");
    return;
  }

  const linePath = points.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
  const [firstX] = points[0];
  const [lastX] = points[points.length - 1];
  const areaPath = `${linePath} L${lastX.toFixed(1)},${height} L${firstX.toFixed(1)},${height} Z`;

  lineEl.setAttribute("d", linePath);
  areaEl.setAttribute("d", areaPath);
}

/// Pushes a sample into a rolling history array, capped at
/// `SPARKLINE_SAMPLE_COUNT`.
function pushHistory(history, value) {
  history.push(value);
  if (history.length > SPARKLINE_SAMPLE_COUNT) {
    history.shift();
  }
}

function renderCpu(cpu) {
  const pct = cpu.usage_pct;
  const color = usageSeverityColor(pct);

  cpuValueEl.textContent = `${pct.toFixed(0)}%`;
  setRing(cpuRingArcEl, pct, color);
  cpuSparklineLineEl.style.stroke = color;

  cpuCoreCountEl.hidden = false;
  cpuCoreCountEl.textContent = t(cpu.core_count === 1 ? "cpu.core" : "cpu.cores", { n: cpu.core_count });

  pushHistory(cpuHistory, pct);
  renderSparkline(cpuHistory, cpuSparklineAreaEl, cpuSparklineLineEl);
}

function renderMemory(memory) {
  const pct = (memory.used_bytes / memory.total_bytes) * 100;
  const color = usageSeverityColor(pct);

  memoryPctValueEl.textContent = `${pct.toFixed(0)}%`;
  setRing(memoryRingArcEl, pct, color);
  memorySparklineLineEl.style.stroke = color;

  memoryTotalEl.hidden = false;
  memoryTotalEl.textContent = t("memory.of", {
    used: formatBytes(memory.used_bytes),
    total: formatBytes(memory.total_bytes),
  });

  pushHistory(memoryHistory, pct);
  renderSparkline(memoryHistory, memorySparklineAreaEl, memorySparklineLineEl);
}

function renderTemperature(cpu) {
  if (cpu.temp_c == null) {
    tempValueEl.textContent = "—";
    tempStatusEl.textContent = "";
    tempBarFillEl.style.width = "0%";
    tempTagEl.hidden = true;
    tempDetailEl.textContent = t("temp.unavailable");
    return;
  }

  const temp = cpu.temp_c;
  // Thresholds are a rough guideline, not per-CPU thermal spec: comfortable
  // below 70°C, worth watching from 70°C, hot from 85°C.
  let color = COLOR_BRAND;
  let status = t("temp.normal");
  if (temp >= 85) {
    color = COLOR_CRITICAL;
    status = t("temp.critical");
  } else if (temp >= 70) {
    color = COLOR_WARNING;
    status = t("temp.high");
  }

  tempValueEl.textContent = `${temp.toFixed(0)}°C`;
  tempStatusEl.textContent = status;
  tempStatusEl.style.color = color;
  tempBarFillEl.style.width = `${Math.max(0, Math.min(100, temp))}%`;
  tempBarFillEl.style.backgroundColor = color;

  tempTagEl.hidden = false;
  if (cpu.temp_estimated) {
    tempTagEl.textContent = t("temp.estimated");
    tempDetailEl.textContent = t("temp.note");
  } else {
    tempTagEl.textContent = t("temp.measured");
    tempDetailEl.textContent = "";
  }
}

function renderDisks(disks) {
  diskListEl.replaceChildren();

  if (disks.length > 0) {
    const totalBytes = disks.reduce((sum, disk) => sum + disk.total_bytes, 0);
    diskTotalEl.hidden = false;
    diskTotalEl.textContent =
      disks.length > 1 ? t("disk.totalSuffix", { total: formatBytes(totalBytes) }) : formatBytes(totalBytes);
  }

  if (disks.length === 0) {
    const empty = document.createElement("p");
    empty.className = "text-sm text-ink-faint";
    empty.textContent = t("disk.noData");
    diskListEl.append(empty);
    return;
  }

  for (const disk of disks) {
    const pct = (disk.used_bytes / disk.total_bytes) * 100;

    const row = document.createElement("div");

    const labelRow = document.createElement("div");
    labelRow.className = "mb-1 flex items-center justify-between text-sm";

    const nameEl = document.createElement("span");
    nameEl.className = "truncate text-ink";
    nameEl.textContent = disk.name;

    const detailEl = document.createElement("span");
    detailEl.className = "shrink-0 pl-2 tabular-nums text-ink-muted";
    detailEl.textContent = t("disk.of", { used: formatBytes(disk.used_bytes), total: formatBytes(disk.total_bytes) });

    labelRow.append(nameEl, detailEl);

    const track = document.createElement("div");
    track.className = "h-1.5 w-full overflow-hidden rounded-full bg-surface-raised";
    const fill = document.createElement("div");
    fill.className = "h-full rounded-full";
    fill.style.width = `${Math.max(0, Math.min(100, pct))}%`;
    fill.style.backgroundColor = usageSeverityColor(pct);
    track.append(fill);

    row.append(labelRow, track);
    diskListEl.append(row);
  }
}

function renderNetwork(network) {
  networkRxEl.textContent = formatRate(network.rx_bps);
  networkTxEl.textContent = formatRate(network.tx_bps);
}

function renderProcesses(processes) {
  processRowsEl.replaceChildren();

  if (processes.length === 0) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.className = "py-3 text-ink-faint";
    cell.colSpan = 3;
    cell.textContent = t("processes.noData");
    row.append(cell);
    processRowsEl.append(row);
    return;
  }

  for (const process of processes) {
    // Built with createElement/textContent, not innerHTML — a process name
    // is OS-supplied, not trusted markup.
    const row = document.createElement("tr");

    const nameCell = document.createElement("td");
    nameCell.className = "max-w-0 truncate py-2 pr-2";
    nameCell.title = `${process.name} (PID ${process.pid})`;
    nameCell.textContent = process.name;

    // Not color-coded like the system-wide rings above: sysinfo reports
    // per-process CPU relative to one core, so a busy multi-threaded process
    // legitimately reads well past 100% on a multi-core machine — the 70/90
    // thresholds that make sense for total system usage would misleadingly
    // flag ordinary processes as "critical".
    const cpuCell = document.createElement("td");
    cpuCell.className = "py-2 pr-2 text-right tabular-nums text-ink";
    cpuCell.textContent = `${process.cpu_pct.toFixed(1)}%`;

    const memoryCell = document.createElement("td");
    memoryCell.className = "py-2 pl-2 text-right tabular-nums text-ink-muted";
    memoryCell.textContent = formatProcessBytes(process.memory_bytes);

    row.append(nameCell, cpuCell, memoryCell);
    processRowsEl.append(row);
  }
}

async function refreshMetrics() {
  try {
    // `get_top_processes` refreshes its own process list on a slower,
    // internal cadence (see `SystemCollector::top_processes`), so it's
    // independent of `get_system_metrics` — no ordering requirement here.
    const metrics = await invoke("get_system_metrics");
    renderCpu(metrics.cpu);
    renderMemory(metrics.memory);
    renderTemperature(metrics.cpu);
    renderDisks(metrics.disks);
    renderNetwork(metrics.network);

    const processes = await invoke("get_top_processes");
    renderProcesses(processes);
  } catch (error) {
    console.error("Could not read system metrics:", error);
  }
}

// --- Claude Usage -----------------------------------------------------
//
// Polled on its own, slower timer: it's a cheap file read on the Rust side
// (no sysinfo refresh), and plan limits only change when Claude Code itself
// updates its status line, not every second.

const CLAUDE_POLL_INTERVAL_MS = 5000;

function formatRelativeTime(ms) {
  const seconds = Math.round((Date.now() - ms) / 1000);
  if (seconds < 5) return t("common.justNow");
  if (seconds < 60) return t("common.secondsAgo", { n: seconds });
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return t("common.minutesAgo", { n: minutes });
  return t("common.hoursAgo", { n: Math.round(minutes / 60) });
}

/// `targetMs` is a future Unix epoch millisecond timestamp — see
/// `PlanLimitWindow::resets_at` in providers::claude.
function formatCountdown(targetMs) {
  const diffMinutes = Math.round((targetMs - Date.now()) / 60_000);
  if (diffMinutes <= 0) return t("common.resetting");
  if (diffMinutes < 60) return t("common.resetsIn", { value: t("common.min", { n: diffMinutes }) });
  const hours = Math.floor(diffMinutes / 60);
  const minutes = diffMinutes % 60;
  if (hours < 24) return t("common.resetsIn", { value: t("common.hoursMinutes", { n: hours, m: minutes }) });
  const days = Math.floor(hours / 24);
  return t("common.resetsIn", { value: t("common.daysHours", { n: days, m: hours % 24 }) });
}

function renderClaudeLimitWindow(label, window) {
  const row = document.createElement("div");

  const labelRow = document.createElement("div");
  labelRow.className = "mb-1 flex items-center justify-between text-sm";
  const nameEl = document.createElement("span");
  nameEl.className = "text-ink";
  nameEl.textContent = label;
  const pctEl = document.createElement("span");
  pctEl.className = "tabular-nums text-ink-muted";
  pctEl.textContent = `${window.used_pct.toFixed(0)}%`;
  labelRow.append(nameEl, pctEl);

  const track = document.createElement("div");
  track.className = "h-1.5 w-full overflow-hidden rounded-full bg-surface-raised";
  const fill = document.createElement("div");
  fill.className = "h-full rounded-full";
  fill.style.width = `${Math.max(0, Math.min(100, window.used_pct))}%`;
  fill.style.backgroundColor = usageSeverityColor(window.used_pct);
  track.append(fill);

  row.append(labelRow, track);

  // `resets_at` is absent, not just null, right after a window resets and
  // before fresh data confirms the next reset time — see the protocol's
  // `plan_limits` fields.
  if (window.resets_at != null) {
    const resetEl = document.createElement("p");
    resetEl.className = "mt-1 text-xs text-ink-faint";
    resetEl.textContent = formatCountdown(window.resets_at);
    row.append(resetEl);
  }

  return row;
}

function renderClaudeStatus(status) {
  claudeErrorEl.hidden = true;

  const tagText = status.connected ? t("claude.connected") : t("claude.notConnected");
  for (const tagEl of [claudeStatusTagEl, claudeSettingsStatusTagEl]) {
    tagEl.textContent = tagText;
    tagEl.classList.toggle("tag-connected", status.connected);
    tagEl.classList.toggle("tag-disconnected", !status.connected);
  }

  claudeConnectButtonEl.hidden = status.connected;
  claudeDisconnectButtonEl.hidden = !status.connected;

  claudeDisconnectedNoteEl.hidden = status.connected;
  claudeConnectedEl.hidden = !status.connected;

  if (!status.connected) return;

  claudeLimitsEl.replaceChildren();
  const limits = status.plan_limits;
  if (!limits || (!limits.five_hour && !limits.seven_day)) {
    const waiting = document.createElement("p");
    waiting.className = "text-sm text-ink-faint";
    waiting.textContent = t("claude.waiting");
    claudeLimitsEl.append(waiting);
    claudeUpdatedEl.textContent = "";
    return;
  }

  if (limits.five_hour) {
    claudeLimitsEl.append(renderClaudeLimitWindow(t("claude.fiveHour"), limits.five_hour));
  }
  if (limits.seven_day) {
    claudeLimitsEl.append(renderClaudeLimitWindow(t("claude.sevenDay"), limits.seven_day));
  }
  claudeUpdatedEl.textContent = t("claude.updated", { time: formatRelativeTime(limits.updated_at) });
}

async function refreshClaudeStatus() {
  try {
    const status = await invoke("get_claude_statusline_status");
    renderClaudeStatus(status);
  } catch (error) {
    console.error("Could not read Claude status line status:", error);
  }
}

function showClaudeError(error) {
  claudeErrorEl.hidden = false;
  claudeErrorEl.textContent = typeof error === "string" ? error : t("common.genericError");
}

claudeConnectButtonEl.addEventListener("click", async () => {
  claudeConnectButtonEl.disabled = true;
  try {
    const status = await invoke("connect_claude_statusline");
    renderClaudeStatus(status);
  } catch (error) {
    showClaudeError(error);
  } finally {
    claudeConnectButtonEl.disabled = false;
  }
});

claudeDisconnectButtonEl.addEventListener("click", async () => {
  const confirmed = await tucanoConfirm({
    title: t("claude.disconnectConfirmTitle"),
    text: t("claude.disconnectConfirmText"),
    confirm: t("claude.disconnect"),
    cancel: t("common.cancel"),
    tone: "danger",
  });
  if (!confirmed) return;

  claudeDisconnectButtonEl.disabled = true;
  try {
    const status = await invoke("disconnect_claude_statusline");
    renderClaudeStatus(status);
  } catch (error) {
    showClaudeError(error);
  } finally {
    claudeDisconnectButtonEl.disabled = false;
  }
});

// --- Ambient Weather ----------------------------------------------------
//
// The Rust side already rate-limits its own outbound calls (see
// providers::weather::REFRESH_INTERVAL) — this poll is just how often the
// UI re-checks that cache, not how often it hits the network.

const WEATHER_POLL_INTERVAL_MS = 60_000;

// Swapped into `#weather-icon-shape` by `renderWeather` based on
// `weather.is_daytime` (from Open-Meteo's own `is_day`, computed against
// the *displayed* location's sunrise/sunset — not this machine's).
const WEATHER_ICON_SUN = `
  <circle cx="12" cy="12" r="4.5" />
  <path d="M12 2v2.5M12 19.5V22M4.22 4.22l1.77 1.77M18 18l1.78 1.78M2 12h2.5M19.5 12H22M4.22 19.78 6 18M18 6l1.78-1.78" />
`;
const WEATHER_ICON_MOON = `
  <path d="M20.5 14.5A8.5 8.5 0 1 1 9.5 3.5a6.5 6.5 0 0 0 11 11Z" />
`;

/// Outdoor temperature has a wider comfortable range than the usage
/// percentages elsewhere, and cold is as worth flagging as hot — so this
/// doesn't reuse `usageSeverityColor`.
function weatherStatus(tempC) {
  if (tempC >= 35) return { label: t("weather.veryHot"), color: COLOR_CRITICAL };
  if (tempC >= 28) return { label: t("weather.hot"), color: COLOR_WARNING };
  if (tempC <= -5) return { label: t("weather.veryCold"), color: COLOR_CRITICAL };
  if (tempC <= 5) return { label: t("weather.cold"), color: COLOR_WARNING };
  return { label: t("weather.pleasant"), color: COLOR_BRAND };
}

function renderWeather(weather) {
  if (!weather) {
    weatherTempEl.textContent = "—";
    weatherStatusEl.textContent = "";
    weatherIconEl.style.color = "var(--color-ink-faint)";
    weatherCityEl.hidden = true;
    weatherDetailEl.textContent = t("weather.unavailable");
    return;
  }

  const status = weatherStatus(weather.temp_c);
  weatherIconShapeEl.innerHTML = weather.is_daytime ? WEATHER_ICON_SUN : WEATHER_ICON_MOON;

  weatherCityEl.hidden = false;
  weatherCityEl.textContent = weather.city;
  weatherTempEl.textContent = `${Math.round(weather.temp_c)}°C`;
  weatherStatusEl.textContent = status.label;
  weatherStatusEl.style.color = status.color;
  weatherIconEl.style.color = status.color;
  weatherDetailEl.textContent = [
    t("weather.feelsLike", { temp: `${Math.round(weather.feels_like_c)}°C` }),
    t("weather.humidity", { pct: Math.round(weather.humidity_pct) }),
    t("common.updatedInline", { time: formatRelativeTime(weather.updated_at) }),
  ].join(" · ");
}

async function refreshWeather() {
  try {
    const weather = await invoke("get_ambient_weather");
    renderWeather(weather);
  } catch (error) {
    console.error("Could not read ambient weather:", error);
  }
}

// --- Settings -------------------------------------------------------------

/// Re-renders everything that has language-dependent text, without waiting
/// for each feature's own poll timer — called right after the language
/// changes so the switch feels immediate.
// The persisted override's city, or `null` for automatic — kept alongside
// `locationSelect` because its own `items` list can't be trusted to still
// hold it: `refresh()` (called below on a language change) replaces
// `items` wholesale by re-reading the native `<select>`'s `<option>`s,
// which are always empty here (this select's options only ever exist as
// this JS-side `items` array — see `syncLocationSelectDisplay`).
let currentLocationCity = null;

// Same field-keeps-its-value behavior as `languageSelect` below: the
// select shows the current override (or the placeholder, for automatic),
// not a blank search box after every change. Remote selects only know
// about items an actual search returned, so there's no real "São Miguel
// do Guaporé..." item to point `setValue` at — this makes one up just to
// hold that label. Opening the field still starts a fresh search over it,
// same as clicking into `languageSelect`.
function syncLocationSelectDisplay() {
  if (currentLocationCity) {
    locationSelect.items = [{ value: "current", label: currentLocationCity, selected: false }];
    locationSelect.setValue("current", { silent: true });
  } else {
    locationSelect.items = [];
    locationSelect.clear({ silent: true });
  }
}

function refreshAllForLanguageChange() {
  applyTranslations();
  // Not covered by `applyTranslations()`: these were set once, as JS
  // options, when `locationSelect` was constructed — `refresh()` re-reads
  // them fresh from `opts`.
  locationSelect.opts.placeholder = t("settings.locationPlaceholder");
  locationSelect.opts.searchPlaceholder = t("settings.locationPlaceholder");
  locationSelect.opts.loadingText = t("settings.locationSearching");
  locationSelect.opts.emptyText = t("settings.locationEmpty");
  locationSelect.opts.errorText = t("common.genericError");
  locationSelect.refresh();
  syncLocationSelectDisplay();
  refreshMetrics();
  refreshClaudeStatus();
  refreshWeather();
}

function applySettingsToForm(settings) {
  // `silent: true`: this reflects a value we already have, not a change the
  // person made — it must not re-fire `onChange` and re-save what we just
  // loaded (or loop, when called from inside that same `onChange`).
  languageSelect.setValue(settings.language, { silent: true });
  currentLocationCity = settings.location_override?.city ?? null;
  if (settings.location_override) {
    settingsLocationCurrentEl.textContent = t("settings.locationCurrent", { city: currentLocationCity });
    settingsLocationClearButtonEl.hidden = false;
  } else {
    settingsLocationCurrentEl.textContent = t("settings.locationAutomatic");
    settingsLocationClearButtonEl.hidden = true;
  }
  syncLocationSelectDisplay();
}

// A plain instance, not `data-tuc-select` + auto-init: we need the
// `onChange` hook and a reference to call `setValue` on when settings load,
// and constructing it ourselves avoids any ambiguity about whether Tucano's
// own auto-init would also try to enhance this same <select>.
const languageSelect = new TucanoSelect("#settings-language", {
  clearable: false,
  onChange: async (language) => {
    try {
      const settings = await invoke("set_language", { language });
      setCurrentLanguage(settings.language);
      applySettingsToForm(settings);
      refreshAllForLanguageChange();
    } catch (error) {
      console.error("Could not change language:", error);
    }
  },
});

async function selectLocationSuggestion(location) {
  settingsLocationErrorEl.hidden = true;
  try {
    const settings = await invoke("select_weather_location", { location });
    applySettingsToForm(settings);
  } catch (error) {
    settingsLocationErrorEl.hidden = false;
    settingsLocationErrorEl.textContent = typeof error === "string" ? error : t("common.genericError");
    return;
  }
  refreshWeather();
}

async function clearLocationOverride() {
  settingsLocationErrorEl.hidden = true;
  try {
    const settings = await invoke("clear_location_override");
    applySettingsToForm(settings);
  } catch (error) {
    settingsLocationErrorEl.hidden = false;
    settingsLocationErrorEl.textContent = typeof error === "string" ? error : t("common.genericError");
    return;
  }
  refreshWeather();
}

// Each search replaces this map — `onChange` looks the chosen option's
// value up in it, since Tucano's <option value> can only carry a string,
// not the whole candidate (city + resolved coordinates).
const locationCandidatesByValue = new Map();

// A remote Select (`loadOptions`), the same component and pattern as the
// language picker above, just in server-search mode — see
// docs/adr/0012-tucano.md. This is also what fixes a real geocoding bug:
// picking a suggestion saves its already-resolved coordinates directly,
// so nothing ever re-searches the formatted "City, Region, Country" string
// this field used to show, which the geocoder can't match on.
const locationSelect = new TucanoSelect("#settings-location-select", {
  search: true,
  clearable: false,
  minChars: 2,
  debounce: 300,
  placeholder: t("settings.locationPlaceholder"),
  searchPlaceholder: t("settings.locationPlaceholder"),
  loadingText: t("settings.locationSearching"),
  emptyText: t("settings.locationEmpty"),
  errorText: t("common.genericError"),
  loadOptions: async (query) => {
    const candidates = await invoke("search_weather_locations", { query });
    locationCandidatesByValue.clear();
    return candidates.map((candidate, index) => {
      const value = String(index);
      locationCandidatesByValue.set(value, candidate);
      return { value, label: candidate.city };
    });
  },
  onChange: (value) => {
    // `selectLocationSuggestion` calls `applySettingsToForm` on success,
    // which sets the field's displayed value back to what actually got
    // saved (see there) — nothing to do here if `value` doesn't resolve to
    // a candidate.
    const candidate = locationCandidatesByValue.get(value);
    if (candidate) selectLocationSuggestion(candidate);
  },
});

settingsLocationClearButtonEl.addEventListener("click", clearLocationOverride);

window.addEventListener("DOMContentLoaded", async () => {
  // Wires every `data-tuc-*` element already in the page — the settings
  // dialog's open/close triggers, chiefly. Elements we construct a class
  // instance for ourselves (the language `Select`, above) are already live
  // by this point and this won't double-init them.
  initTucano();

  try {
    const settings = await invoke("get_settings");
    setCurrentLanguage(settings.language);
    applyTranslations();
    applySettingsToForm(settings);
  } catch (error) {
    console.error("Could not load settings:", error);
    applyTranslations(); // still apply the default-language strings
  }

  refreshMetrics();
  setInterval(refreshMetrics, POLL_INTERVAL_MS);

  refreshClaudeStatus();
  setInterval(refreshClaudeStatus, CLAUDE_POLL_INTERVAL_MS);

  refreshWeather();
  setInterval(refreshWeather, WEATHER_POLL_INTERVAL_MS);
});
