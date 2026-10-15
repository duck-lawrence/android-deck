const { invoke } = window.__TAURI__.core;
const $ = (sel) => document.querySelector(sel);

const state = {
  settings: { adbPath: "adb", scrcpyPath: "", defaultScrcpyArgs: [] },
  devices: [],
  connected: [],
  pollTimer: null,
};

const DEFAULT_CONNECT_PORT = 5555;

/* ---------------------- Helpers ---------------------- */

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function splitArgs(text) {
  return String(text || "")
    .split(/\s+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function endpoint(device) {
  const port = Number(device.connectPort) || DEFAULT_CONNECT_PORT;
  return `${device.ip}:${port}`;
}

function pairEndpoint(device) {
  if (!device.pairPort) return null;
  const ip = device.pairIp || device.ip;
  return `${ip}:${device.pairPort}`;
}

let toastTimer;
function toast(message, kind = "info") {
  const el = $("#toast");
  el.textContent = message;
  el.className = `toast ${kind}`;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.add("hidden"), 4000);
}

/* ---------------------- Tabs ---------------------- */

function switchTab(name) {
  document
    .querySelectorAll(".tab")
    .forEach((b) => b.classList.toggle("active", b.dataset.tab === name));
  document
    .querySelectorAll(".tab-panel")
    .forEach((p) => p.classList.toggle("active", p.id === `tab-${name}`));
  if (name === "live") refreshLive();
}

document.querySelectorAll(".tab").forEach((btn) => {
  btn.addEventListener("click", () => switchTab(btn.dataset.tab));
});

/* ---------------------- Settings ---------------------- */

function readSettingsForm() {
  return {
    adbPath: $("#adb-path").value.trim() || "adb",
    scrcpyPath: $("#scrcpy-path").value.trim(),
    defaultScrcpyArgs: splitArgs($("#default-args").value),
  };
}

async function persistSettings() {
  const settings = readSettingsForm();
  await invoke("save_settings", { settings });
  state.settings = settings;
  return settings;
}

async function loadSettings() {
  state.settings = await invoke("get_settings");
  $("#adb-path").value = state.settings.adbPath || "";
  $("#scrcpy-path").value = state.settings.scrcpyPath || "";
  $("#default-args").value = (state.settings.defaultScrcpyArgs || []).join(" ");
}

$("#settings-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  try {
    await persistSettings();
    toast("Settings saved", "ok");
  } catch (err) {
    toast(String(err), "err");
  }
});

$("#check-tools").addEventListener("click", async () => {
  const box = $("#tool-status");
  box.classList.remove("hidden");
  box.textContent = "Checking...";
  try {
    await persistSettings();
    const status = await invoke("check_tools");
    const line = (ok, label, detail) =>
      `${ok ? "[OK]   " : "[FAIL] "}${label}: ${detail}`;
    box.textContent = [
      line(status.adbOk, "adb", status.adbMessage),
      line(status.scrcpyOk, "scrcpy", status.scrcpyMessage),
    ].join("\n");
  } catch (err) {
    box.textContent = String(err);
  }
});

/* ---------------------- Auto-detect / Browse ---------------------- */

function fillCandidates(kind, list) {
  const select = $(`#${kind}-candidates`);
  select.innerHTML = "";
  if (list.length <= 1) {
    select.classList.add("hidden");
    return;
  }
  for (const item of list) {
    const option = document.createElement("option");
    option.value = item.path;
    option.textContent = `${item.path}  ·  ${item.source}`;
    select.appendChild(option);
  }
  select.classList.remove("hidden");
}

async function detectTools({ autoSave = false, silent = false } = {}) {
  let result;
  try {
    result = await invoke("detect_tools");
  } catch (err) {
    if (!silent) toast(String(err), "err");
    return null;
  }

  fillCandidates("adb", result.adb);
  fillCandidates("scrcpy", result.scrcpy);

  const adbFound = result.adb.length > 0;
  const scrcpyFound = result.scrcpy.length > 0;

  $("#adb-path").value = adbFound ? result.adb[0].path : "";
  $("#adb-hint").textContent = adbFound
    ? `Detected (${result.adb[0].source}).`
    : 'Not found — leaving it empty uses "adb" from PATH.';

  $("#scrcpy-path").value = scrcpyFound ? result.scrcpy[0].path : "";
  $("#scrcpy-hint").textContent = scrcpyFound
    ? `Detected (${result.scrcpy[0].source}).`
    : "Not found — please Browse... or enter the path manually.";

  if (autoSave) {
    try {
      await persistSettings();
    } catch (err) {
      toast(String(err), "err");
    }
  }

  if (!silent) {
    toast(
      `Detected — adb: ${adbFound ? "yes" : "no"}, scrcpy: ${scrcpyFound ? "yes" : "no"}`,
      scrcpyFound ? "ok" : "info",
    );
  }

  return { adbFound, scrcpyFound };
}

$("#detect-tools").addEventListener("click", () => detectTools({ autoSave: true }));

["adb", "scrcpy"].forEach((kind) => {
  $(`#${kind}-candidates`).addEventListener("change", (event) => {
    $(`#${kind}-path`).value = event.target.value;
  });
});

document.querySelectorAll(".browse").forEach((button) => {
  button.addEventListener("click", async () => {
    const kind = button.dataset.browse;
    try {
      const path = await invoke("pick_tool_file", { kind });
      if (path) {
        $(`#${kind}-path`).value = path;
        $(`#${kind}-hint`).textContent = "Selected manually.";
      }
    } catch (err) {
      toast(String(err), "err");
    }
  });
});


/* ---------------------- Devices ---------------------- */

async function loadDevices() {
  try {
    state.devices = await invoke("list_devices");
  } catch (err) {
    toast(String(err), "err");
    state.devices = [];
  }
  renderDevices();
}

function findConnected(device) {
  const ep = endpoint(device);
  return state.connected.find((c) => c.serial === ep);
}

function statusBadge(device) {
  const conn = findConnected(device);
  if (!conn) return '<span class="badge off">Offline</span>';
  if (conn.status === "device") return '<span class="badge on">Online</span>';
  return `<span class="badge warn">${escapeHtml(conn.status)}</span>`;
}

function setCardBusy(card, busy) {
  if (!card) return;
  card.querySelectorAll("button, input").forEach((el) => {
    if (busy) {
      if (el.dataset.wasDisabled === undefined) {
        el.dataset.wasDisabled = el.disabled ? "1" : "0";
      }
      el.disabled = true;
    } else {
      el.disabled = el.dataset.wasDisabled === "1";
      delete el.dataset.wasDisabled;
    }
  });
}

function renderDevices() {
  const list = $("#device-list");
  const empty = $("#device-empty");
  list.innerHTML = "";
  empty.classList.toggle("hidden", state.devices.length > 0);

  for (const device of state.devices) {
    const conn = findConnected(device);
    const online = conn && conn.status === "device";
    const pair = pairEndpoint(device);
    const port = Number(device.connectPort) || DEFAULT_CONNECT_PORT;

    const card = document.createElement("div");
    card.className = "device-card";
    card.innerHTML = `
      <div class="device-info">
        <h3>${escapeHtml(device.name || "Unnamed")} ${statusBadge(device)}</h3>
        <p class="muted">${escapeHtml(device.ip)}:<input
          type="number" min="1" max="65535" class="port-input" data-port
          value="${port}"
          title="Connect port — changes each time Wireless debugging is toggled or the phone reboots"
        />${conn && conn.model ? " · " + escapeHtml(conn.model) : ""}</p>
        <p class="muted small">Pairing: ${
          pair ? escapeHtml(pair) : "not configured"
        }</p>
        ${
          conn
            ? ""
            : '<p class="muted small">Saved — check the current port on the phone, then click Connect.</p>'
        }
      </div>
      <div class="device-actions">
        <button class="btn primary" data-act="connect">Connect</button>
        <button class="btn" data-act="scrcpy" ${online ? "" : "disabled"}>Open scrcpy</button>
        <button class="btn" data-act="disconnect" ${conn ? "" : "disabled"}>Disconnect</button>
        <button class="btn" data-act="edit">Edit</button>
        <button class="btn danger" data-act="delete">Delete</button>
      </div>`;

    const portInput = card.querySelector("[data-port]");

    card.querySelectorAll("[data-act]").forEach((btn) => {
      btn.addEventListener("click", () =>
        handleDeviceAction(btn.dataset.act, device, card, portInput),
      );
    });
    list.appendChild(card);
  }
}

async function handleDeviceAction(action, device, card, portInput) {
  if (action === "edit") return openDeviceDialog(device);

  if (action === "delete") {
    if (!confirm(`Delete device "${device.name}"?`)) return;
    try {
      setCardBusy(card, true);
      await invoke("delete_device", { id: device.id });
      toast("Device deleted", "ok");
      await loadDevices();
    } catch (err) {
      toast(String(err), "err");
      setCardBusy(card, false);
    }
    return;
  }

  setCardBusy(card, true);
  try {
    const nextPort = portInput
      ? Number(portInput.value) || device.connectPort
      : device.connectPort;
    if (portInput && nextPort !== device.connectPort) {
      const saved = await invoke("save_device", {
        device: { ...device, connectPort: nextPort },
      });
      Object.assign(device, saved);
    }

    if (action === "connect") {
      await invoke("adb_connect", { hostPort: endpoint(device) });
      toast("Connect command sent", "ok");
    } else if (action === "disconnect") {
      await invoke("adb_disconnect", { hostPort: endpoint(device) });
      toast("Disconnected", "ok");
    } else if (action === "scrcpy") {
      return launchScrcpy(device);
    }

    await refreshLive();
  } catch (err) {
    toast(String(err), "err");
  } finally {
    setCardBusy(card, false);
  }
}

async function launchScrcpy(device) {
  const args =
    device.scrcpyArgs && device.scrcpyArgs.length
      ? device.scrcpyArgs
      : state.settings.defaultScrcpyArgs || [];
  try {
    await invoke("launch_scrcpy", { serial: endpoint(device), args });
    toast("scrcpy launched", "ok");
  } catch (err) {
    toast(String(err), "err");
  }
}

/* ---------------------- Device dialog ---------------------- */

function openDeviceDialog(device) {
  const isEdit = Boolean(device);
  $("#device-dialog-title").textContent = isEdit ? "Edit device" : "Add device";
  $("#device-id").value = device?.id || "";
  $("#device-name").value = device?.name || "";
  $("#device-ip").value = device?.ip || "";
  $("#device-connect-port").value = device?.connectPort || DEFAULT_CONNECT_PORT;
  $("#device-pair-ip").value = device?.pairIp || "";
  $("#device-pair-port").value = device?.pairPort || "";
  $("#device-pair-code").value = "";
  $("#device-args").value = (device?.scrcpyArgs || []).join(" ");
  $("#device-dialog").showModal();
}

$("#add-device").addEventListener("click", () => openDeviceDialog(null));
$("#device-cancel").addEventListener("click", () => $("#device-dialog").close());

$("#device-form").addEventListener("submit", async (event) => {
  event.preventDefault();

  const ip = $("#device-ip").value.trim();
  const name = $("#device-name").value.trim();
  if (!name || !ip) {
    toast("Please enter a name and an IP address", "err");
    return;
  }

  const device = {
    id: $("#device-id").value,
    name,
    ip,
    connectPort: Number($("#device-connect-port").value) || DEFAULT_CONNECT_PORT,
    pairIp: $("#device-pair-ip").value.trim() || ip,
    pairPort: $("#device-pair-port").value
      ? Number($("#device-pair-port").value)
      : null,
    scrcpyArgs: splitArgs($("#device-args").value),
  };
  const code = $("#device-pair-code").value.trim();

  try {
    await invoke("save_device", { device });
    $("#device-dialog").close();
    toast("Device saved", "ok");
    await loadDevices();

    const pair = pairEndpoint(device);
    if (code && pair) {
      try {
        await invoke("adb_pair", { hostPort: pair, code });
        toast("Paired successfully", "ok");
      } catch (err) {
        toast(`Pairing failed: ${err}`, "err");
        return refreshLive();
      }
    }

    try {
      await invoke("adb_connect", { hostPort: endpoint(device) });
      toast("Device connected", "ok");
    } catch (err) {
      toast(`Connect failed: ${err}`, "err");
    }
    await refreshLive();
  } catch (err) {
    toast(String(err), "err");
  }
});

/* ---------------------- Live view ---------------------- */

let liveChain = Promise.resolve();
let liveBusy = false;

function refreshLive() {
  if (liveBusy) return liveChain;
  liveBusy = true;
  liveChain = liveChain
    .then(doRefreshLive)
    .catch(() => {})
    .finally(() => {
      liveBusy = false;
    });
  return liveChain;
}

async function doRefreshLive() {
  try {
    state.connected = await invoke("list_connected");
  } catch (err) {
    state.connected = [];
    toast(String(err), "err");
  }
  renderConnected();
  renderDevices();
}

function renderConnected() {
  const body = $("#live-body");
  body.innerHTML = "";
  $("#live-empty").classList.toggle("hidden", state.connected.length > 0);

  for (const conn of state.connected) {
    const kind = conn.serial.includes(":") ? "Wi-Fi" : "USB";
    const badge =
      conn.status === "device"
        ? '<span class="badge on">Online</span>'
        : `<span class="badge warn">${escapeHtml(conn.status)}</span>`;
    const tr = document.createElement("tr");
    tr.innerHTML = `
      <td>${escapeHtml(conn.serial)}</td>
      <td>${escapeHtml(conn.model || "—")}</td>
      <td>${kind}</td>
      <td>${badge}</td>`;
    body.appendChild(tr);
  }
}

$("#refresh-live").addEventListener("click", refreshLive);
$("#refresh-devices").addEventListener("click", refreshLive);

$("#kill-server").addEventListener("click", async () => {
  try {
    await invoke("adb_kill_server");
    toast("adb server stopped", "ok");
    await refreshLive();
  } catch (err) {
    toast(String(err), "err");
  }
});

$("#auto-refresh").addEventListener("change", (event) => {
  if (event.target.checked) startPolling();
  else stopPolling();
});

function startPolling() {
  stopPolling();
  state.pollTimer = setInterval(refreshLive, 3000);
}

function stopPolling() {
  if (state.pollTimer) clearInterval(state.pollTimer);
  state.pollTimer = null;
}

/* ---------------------- Init ---------------------- */

async function init() {
  await loadSettings();

  const configured = Boolean(state.settings.adbPath || state.settings.scrcpyPath);
  if (!configured) {
    const result = await detectTools({ autoSave: true, silent: true });
    if (result && (result.adbFound || result.scrcpyFound)) {
      toast("Auto-detected tool paths — review them in Settings", "ok");
    } else {
      toast("Could not auto-detect adb/scrcpy — set the paths in Settings", "info");
    }
  }

  await loadDevices();
  await refreshLive();
  startPolling();
}

init();
