// Translation strings for the settings UI.
//
// Static text lives in `index.html` as `data-i18n="key"` (textContent) or
// `data-i18n-html="key"` (innerHTML, for the handful of strings with an
// embedded link) and gets swapped by `applyTranslations()`. Text this
// module builds itself (status words, relative-time phrasing) reads `t()`
// directly — see main.js.

export const SUPPORTED_LANGUAGES = ["en", "pt", "es"];
export const DEFAULT_LANGUAGE = "en";

const en = {
  "header.noDevicePaired": "No device paired",
  "header.devicePaired": "{name} paired",
  "header.devicesPaired": "{count} devices paired",
  "header.settings": "Settings",

  "common.reading": "Reading…",
  "common.justNow": "just now",
  "common.secondsAgo": "{n}s ago",
  "common.minutesAgo": "{n} min ago",
  "common.hoursAgo": "{n} h ago",
  "common.resetsIn": "resets in {value}",
  "common.resetting": "resetting…",
  "common.updatedInline": "updated {time}",
  "common.genericError": "Something went wrong.",
  "common.cancel": "Cancel",
  "common.min": "{n} min",
  "common.hoursMinutes": "{n}h {m}m",
  "common.daysHours": "{n}d {m}h",

  "cpu.title": "CPU",
  "cpu.used": "used",
  "cpu.core": "{n} core",
  "cpu.cores": "{n} cores",

  "memory.title": "Memory",
  "memory.used": "used",
  "memory.of": "{used} of {total}",

  "temp.title": "CPU Temperature",
  "temp.estimated": "estimated",
  "temp.measured": "measured",
  "temp.unavailable": "Not available on this device.",
  "temp.note": "This platform has no single CPU package sensor; shown is the highest reading among its internal sensors.",
  "temp.normal": "Normal",
  "temp.high": "High",
  "temp.critical": "Critical",

  "claude.title": "Claude Usage",
  "claude.manageInSettings": "Manage this in",
  "claude.notConnected": "not connected",
  "claude.connected": "connected",
  "claude.descriptionHtml":
    'Reads Claude Pro/Max plan limits from the Claude Code status line (<a href="https://github.com/JuniorCarlini/espia" target="_blank" rel="noreferrer" class="underline underline-offset-2">ADR 0005</a>). Reversible any time.',
  "claude.connect": "Connect",
  "claude.disconnect": "Disconnect",
  "claude.disconnectConfirmTitle": "Disconnect Claude?",
  "claude.disconnectConfirmText": "This restores whatever status line was configured before, if any. You can connect again anytime.",
  "claude.waiting": "Waiting for Claude Code to update its status line…",
  "claude.fiveHour": "5-hour",
  "claude.sevenDay": "7-day",
  "claude.updated": "Updated {time}",

  "disk.title": "Disk & Network",
  "disk.download": "Download",
  "disk.upload": "Upload",
  "disk.noData": "No disk data.",
  "disk.of": "{used} of {total}",
  "disk.totalSuffix": "{total} total",

  "weather.title": "Ambient Weather",
  "weather.unavailable": "Unavailable — no internet connection, or the location/weather lookup failed.",
  "weather.feelsLike": "Feels like {temp}",
  "weather.humidity": "{pct}% humidity",
  "weather.footerHtml":
    'Via a public IP-location lookup and <a href="https://open-meteo.com" target="_blank" rel="noreferrer" class="underline underline-offset-2">Open-Meteo</a> — the only thing here that leaves your network.',
  "weather.veryCold": "Very cold",
  "weather.cold": "Cold",
  "weather.pleasant": "Pleasant",
  "weather.hot": "Hot",
  "weather.veryHot": "Very hot",

  "processes.title": "Top Processes",
  "processes.byCpu": "by CPU",
  "processes.process": "Process",
  "processes.cpu": "CPU",
  "processes.memory": "Memory",
  "processes.noData": "No process data.",

  "settings.title": "Settings",
  "settings.close": "Close",
  "settings.language": "Language",
  "settings.location": "Weather location",
  "settings.locationHint": "Overrides the automatic IP-based location.",
  "settings.locationPlaceholder": "e.g. São Paulo, Brazil",
  "settings.locationUseAutomatic": "Use automatic location",
  "settings.locationCurrent": "Currently: {city}",
  "settings.locationAutomatic": "Automatic (based on your IP)",
  "settings.locationEmpty": "No matches.",
  "settings.locationSearching": "Searching…",

  "pairing.title": "Pair a device",
  "pairing.description": "A device wants to connect. Type the 6-digit code shown on its own screen to confirm it's yours.",
  "pairing.codeLabel": "Code shown on the device",
  "pairing.confirm": "Confirm",
  "pairing.cancel": "Cancel",
  "pairing.invalidCode": "Enter the 6-digit code shown on the device.",
  "pairing.rejected": "That code didn't match — try again from the device's own screen.",
};

const pt = {
  "header.noDevicePaired": "Nenhum dispositivo pareado",
  "header.devicePaired": "{name} pareado",
  "header.devicesPaired": "{count} dispositivos pareados",
  "header.settings": "Configurações",

  "common.reading": "Carregando…",
  "common.justNow": "agora mesmo",
  "common.secondsAgo": "há {n}s",
  "common.minutesAgo": "há {n} min",
  "common.hoursAgo": "há {n} h",
  "common.resetsIn": "reinicia em {value}",
  "common.resetting": "reiniciando…",
  "common.updatedInline": "atualizado {time}",
  "common.genericError": "Algo deu errado.",
  "common.cancel": "Cancelar",
  "common.min": "{n} min",
  "common.hoursMinutes": "{n}h {m}m",
  "common.daysHours": "{n}d {m}h",

  "cpu.title": "CPU",
  "cpu.used": "em uso",
  "cpu.core": "{n} núcleo",
  "cpu.cores": "{n} núcleos",

  "memory.title": "Memória",
  "memory.used": "em uso",
  "memory.of": "{used} de {total}",

  "temp.title": "Temperatura da CPU",
  "temp.estimated": "estimada",
  "temp.measured": "medida",
  "temp.unavailable": "Não disponível neste dispositivo.",
  "temp.note": "Esta plataforma não tem um sensor único de pacote da CPU; é mostrada a leitura mais alta entre os sensores internos.",
  "temp.normal": "Normal",
  "temp.high": "Alta",
  "temp.critical": "Crítica",

  "claude.title": "Uso do Claude",
  "claude.manageInSettings": "Gerencie isso em",
  "claude.notConnected": "não conectado",
  "claude.connected": "conectado",
  "claude.descriptionHtml":
    'Lê os limites do plano Claude Pro/Max a partir da status line do Claude Code (<a href="https://github.com/JuniorCarlini/espia" target="_blank" rel="noreferrer" class="underline underline-offset-2">ADR 0005</a>). Reversível a qualquer momento.',
  "claude.connect": "Conectar",
  "claude.disconnect": "Desconectar",
  "claude.disconnectConfirmTitle": "Desconectar o Claude?",
  "claude.disconnectConfirmText": "Isso restaura a status line que estava configurada antes, se houver. Você pode conectar novamente quando quiser.",
  "claude.waiting": "Aguardando o Claude Code atualizar sua status line…",
  "claude.fiveHour": "5 horas",
  "claude.sevenDay": "7 dias",
  "claude.updated": "Atualizado {time}",

  "disk.title": "Disco e Rede",
  "disk.download": "Download",
  "disk.upload": "Upload",
  "disk.noData": "Nenhum dado de disco.",
  "disk.of": "{used} de {total}",
  "disk.totalSuffix": "{total} no total",

  "weather.title": "Temperatura Ambiente",
  "weather.unavailable": "Indisponível — sem conexão com a internet, ou a busca de localização/clima falhou.",
  "weather.feelsLike": "Sensação de {temp}",
  "weather.humidity": "{pct}% de umidade",
  "weather.footerHtml":
    'Via busca pública de localização por IP e <a href="https://open-meteo.com" target="_blank" rel="noreferrer" class="underline underline-offset-2">Open-Meteo</a> — a única coisa aqui que sai da sua rede.',
  "weather.veryCold": "Muito frio",
  "weather.cold": "Frio",
  "weather.pleasant": "Agradável",
  "weather.hot": "Quente",
  "weather.veryHot": "Muito quente",

  "processes.title": "Processos Principais",
  "processes.byCpu": "por CPU",
  "processes.process": "Processo",
  "processes.cpu": "CPU",
  "processes.memory": "Memória",
  "processes.noData": "Nenhum dado de processo.",

  "settings.title": "Configurações",
  "settings.close": "Fechar",
  "settings.language": "Idioma",
  "settings.location": "Localização do clima",
  "settings.locationHint": "Substitui a localização automática por IP.",
  "settings.locationPlaceholder": "ex.: São Paulo, Brasil",
  "settings.locationUseAutomatic": "Usar localização automática",
  "settings.locationCurrent": "Atual: {city}",
  "settings.locationAutomatic": "Automática (baseada no seu IP)",
  "settings.locationEmpty": "Nenhum resultado.",
  "settings.locationSearching": "Buscando…",

  "pairing.title": "Parear um dispositivo",
  "pairing.description": "Um dispositivo quer se conectar. Digite o código de 6 dígitos mostrado na tela dele para confirmar que é seu.",
  "pairing.codeLabel": "Código mostrado no dispositivo",
  "pairing.confirm": "Confirmar",
  "pairing.cancel": "Cancelar",
  "pairing.invalidCode": "Digite o código de 6 dígitos mostrado no dispositivo.",
  "pairing.rejected": "Esse código não bateu — tente de novo olhando a tela do dispositivo.",
};

const es = {
  "header.noDevicePaired": "Ningún dispositivo emparejado",
  "header.devicePaired": "{name} emparejado",
  "header.devicesPaired": "{count} dispositivos emparejados",
  "header.settings": "Configuración",

  "common.reading": "Cargando…",
  "common.justNow": "justo ahora",
  "common.secondsAgo": "hace {n}s",
  "common.minutesAgo": "hace {n} min",
  "common.hoursAgo": "hace {n} h",
  "common.resetsIn": "se reinicia en {value}",
  "common.resetting": "reiniciando…",
  "common.updatedInline": "actualizado {time}",
  "common.genericError": "Algo salió mal.",
  "common.cancel": "Cancelar",
  "common.min": "{n} min",
  "common.hoursMinutes": "{n}h {m}m",
  "common.daysHours": "{n}d {m}h",

  "cpu.title": "CPU",
  "cpu.used": "en uso",
  "cpu.core": "{n} núcleo",
  "cpu.cores": "{n} núcleos",

  "memory.title": "Memoria",
  "memory.used": "en uso",
  "memory.of": "{used} de {total}",

  "temp.title": "Temperatura de la CPU",
  "temp.estimated": "estimada",
  "temp.measured": "medida",
  "temp.unavailable": "No disponible en este dispositivo.",
  "temp.note": "Esta plataforma no tiene un único sensor de paquete de CPU; se muestra la lectura más alta entre sus sensores internos.",
  "temp.normal": "Normal",
  "temp.high": "Alta",
  "temp.critical": "Crítica",

  "claude.title": "Uso de Claude",
  "claude.manageInSettings": "Gestiona esto en",
  "claude.notConnected": "no conectado",
  "claude.connected": "conectado",
  "claude.descriptionHtml":
    'Lee los límites del plan Claude Pro/Max desde la status line de Claude Code (<a href="https://github.com/JuniorCarlini/espia" target="_blank" rel="noreferrer" class="underline underline-offset-2">ADR 0005</a>). Reversible en cualquier momento.',
  "claude.connect": "Conectar",
  "claude.disconnect": "Desconectar",
  "claude.disconnectConfirmTitle": "¿Desconectar Claude?",
  "claude.disconnectConfirmText": "Esto restaura la status line que estaba configurada antes, si la había. Puedes conectar de nuevo cuando quieras.",
  "claude.waiting": "Esperando a que Claude Code actualice su status line…",
  "claude.fiveHour": "5 horas",
  "claude.sevenDay": "7 días",
  "claude.updated": "Actualizado {time}",

  "disk.title": "Disco y Red",
  "disk.download": "Descarga",
  "disk.upload": "Subida",
  "disk.noData": "Sin datos de disco.",
  "disk.of": "{used} de {total}",
  "disk.totalSuffix": "{total} en total",

  "weather.title": "Clima Ambiente",
  "weather.unavailable": "No disponible — sin conexión a internet, o falló la búsqueda de ubicación/clima.",
  "weather.feelsLike": "Sensación de {temp}",
  "weather.humidity": "{pct}% de humedad",
  "weather.footerHtml":
    'Vía una búsqueda pública de ubicación por IP y <a href="https://open-meteo.com" target="_blank" rel="noreferrer" class="underline underline-offset-2">Open-Meteo</a> — lo único aquí que sale de tu red.',
  "weather.veryCold": "Muy frío",
  "weather.cold": "Frío",
  "weather.pleasant": "Agradable",
  "weather.hot": "Caluroso",
  "weather.veryHot": "Muy caluroso",

  "processes.title": "Procesos Principales",
  "processes.byCpu": "por CPU",
  "processes.process": "Proceso",
  "processes.cpu": "CPU",
  "processes.memory": "Memoria",
  "processes.noData": "Sin datos de procesos.",

  "settings.title": "Configuración",
  "settings.close": "Cerrar",
  "settings.language": "Idioma",
  "settings.location": "Ubicación del clima",
  "settings.locationHint": "Anula la ubicación automática basada en IP.",
  "settings.locationPlaceholder": "ej.: Ciudad de México, México",
  "settings.locationUseAutomatic": "Usar ubicación automática",
  "settings.locationCurrent": "Actual: {city}",
  "settings.locationAutomatic": "Automática (basada en tu IP)",
  "settings.locationEmpty": "Sin resultados.",
  "settings.locationSearching": "Buscando…",

  "pairing.title": "Emparejar un dispositivo",
  "pairing.description": "Un dispositivo quiere conectarse. Escribe el código de 6 dígitos que aparece en su propia pantalla para confirmar que es tuyo.",
  "pairing.codeLabel": "Código mostrado en el dispositivo",
  "pairing.confirm": "Confirmar",
  "pairing.cancel": "Cancelar",
  "pairing.invalidCode": "Escribe el código de 6 dígitos que aparece en el dispositivo.",
  "pairing.rejected": "Ese código no coincidió — inténtalo de nuevo mirando la pantalla del dispositivo.",
};

const dictionaries = { en, pt, es };

let currentLanguage = DEFAULT_LANGUAGE;

export function setCurrentLanguage(lang) {
  currentLanguage = SUPPORTED_LANGUAGES.includes(lang) ? lang : DEFAULT_LANGUAGE;
}

export function getCurrentLanguage() {
  return currentLanguage;
}

/// Looks up `key` in the current language, falling back to English, then to
/// the key itself — so a missing translation shows as an ugly-but-visible
/// key rather than silently blank text.
export function t(key, vars) {
  const dict = dictionaries[currentLanguage] ?? dictionaries[DEFAULT_LANGUAGE];
  let text = dict[key] ?? dictionaries[DEFAULT_LANGUAGE][key] ?? key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.replaceAll(`{${name}}`, value);
    }
  }
  return text;
}

/// Applies the current language to every `[data-i18n]` (textContent) and
/// `[data-i18n-html]` (innerHTML — only for the few strings with an
/// embedded link) element in the document.
export function applyTranslations() {
  for (const el of document.querySelectorAll("[data-i18n]")) {
    el.textContent = t(el.getAttribute("data-i18n"));
  }
  for (const el of document.querySelectorAll("[data-i18n-html]")) {
    el.innerHTML = t(el.getAttribute("data-i18n-html"));
  }
  for (const el of document.querySelectorAll("[data-i18n-title]")) {
    el.title = t(el.getAttribute("data-i18n-title"));
  }
  for (const el of document.querySelectorAll("[data-i18n-placeholder]")) {
    el.placeholder = t(el.getAttribute("data-i18n-placeholder"));
  }
  document.documentElement.lang = currentLanguage;
}
