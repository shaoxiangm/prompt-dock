export {};

const palette = {
  blue: "蓝色", green: "绿色", amber: "琥珀色",
  purple: "紫色", rose: "玫红色", teal: "青色",
};
type Color = keyof typeof palette;
interface Prompt { id: string; title: string; content: string; color: Color }
interface MenuStatus { autostart: boolean | null; warning: string | null }
interface EditorState { selectedId: string | null; newId: string | null; drafts: Prompt[] }
type Change =
  | { kind: "save"; prompt: Prompt }
  | { kind: "delete"; id: string }
  | { kind: "autostart"; enabled: boolean };

declare global {
  interface Window {
    __TAURI__?: {
      core: { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> };
      event: {
        listen<T>(event: string, handler: (event: { payload: T }) => void): Promise<() => void>;
      };
    };
  }
}

const iconPaths: Record<string, string> = {
  "start-work": "M3 17h18M6 13a6 6 0 0 1 12 0M12 3v3M4.2 6.5l2 2m13.6-2-2 2",
  "end-work": "M3 17h18M6 13a6 6 0 0 1 12 0M7 21h10M12 3v4m-2-2 2 2 2-2",
  "simulation-debug": "M3 12h4l2-6 4 12 2-6h6",
  "code-review": "m8 6-6 6 6 6m8-12 6 6-6 6m-2-14-4 16",
};
const genericIcon = "M6 4h12a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-6l-4 4v-4H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Zm2 5h8m-8 3h5";

function element<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

const tiles = element<HTMLDivElement>("tiles");
const add = element<HTMLButtonElement>("add-prompt");
const editor = element<HTMLFormElement>("editor");
const empty = element<HTMLParagraphElement>("empty-editor");
const title = element<HTMLInputElement>("prompt-title");
const content = element<HTMLTextAreaElement>("prompt-content");
const colors = element<HTMLFieldSetElement>("colors");
const save = element<HTMLButtonElement>("save-prompt");
const copy = element<HTMLButtonElement>("copy-prompt");
const remove = element<HTMLButtonElement>("delete-prompt");
const dialog = element<HTMLDialogElement>("delete-dialog");
const theme = element<HTMLInputElement>("theme-toggle");
const autostart = element<HTMLInputElement>("autostart-toggle");
const copyFeedback = element<HTMLSpanElement>("copy-feedback");
const feedback = element<HTMLElement>("feedback");

let prompts: Prompt[] = [];
let selectedId: string | undefined;
let newId: string | undefined;
let pendingEdit = false;
let pendingAutostart = false;
let autostartEnabled: boolean | null = null;
let copyTimer: number | undefined;
const drafts = new Map<string, Prompt>();

function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const tauri = window.__TAURI__;
  if (!tauri) return Promise.reject("请在 Prompt Dock 桌面窗口中打开指令库。");
  return tauri.core.invoke<T>(command, args);
}

function notify(message: string, error = false) {
  feedback.textContent = message;
  feedback.dataset.error = String(error);
}

function updateTheme() {
  document.documentElement.dataset.theme = theme.checked ? "dark" : "light";
  element("theme-label").textContent = theme.checked ? "白天模式" : "夜间模式";
}

theme.checked = localStorage.getItem("prompt-dock-theme") === "dark";
updateTheme();
theme.addEventListener("change", () => {
  updateTheme();
  localStorage.setItem("prompt-dock-theme", theme.checked ? "dark" : "light");
});

for (const [color, name] of Object.entries(palette)) {
  const swatch = document.createElement("label");
  swatch.className = "swatch";
  swatch.dataset.color = color;
  swatch.title = name;
  const option = document.createElement("input");
  option.type = "radio";
  option.name = "color";
  option.value = color;
  option.setAttribute("aria-label", name);
  swatch.append(option, document.createElement("span"));
  colors.append(swatch);
}

function renderTiles() {
  for (const tile of tiles.querySelectorAll(".prompt-tile")) tile.remove();
  const fragment = document.createDocumentFragment();
  for (const prompt of prompts) {
    const tile = document.createElement("button");
    tile.type = "button";
    tile.className = "tile prompt-tile";
    tile.dataset.color = prompt.color;
    tile.title = prompt.title;
    tile.setAttribute("aria-pressed", String(prompt.id === selectedId));
    const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("aria-hidden", "true");
    const path = document.createElementNS(icon.namespaceURI, "path");
    path.setAttribute("d", Object.hasOwn(iconPaths, prompt.id) ? iconPaths[prompt.id] : genericIcon);
    icon.append(path);
    const name = document.createElement("span");
    name.className = "tile-title";
    name.textContent = prompt.title;
    tile.append(icon, name);
    tile.addEventListener("click", () => {
      select(prompt.id);
      tiles.querySelector<HTMLButtonElement>('.prompt-tile[aria-pressed="true"]')?.focus();
    });
    fragment.append(tile);
  }
  tiles.insertBefore(fragment, add);
  add.setAttribute("aria-pressed", String(newId !== undefined && selectedId === newId));
}

function currentPrompt(): Prompt | undefined {
  if (!selectedId) return;
  return drafts.get(selectedId) ?? prompts.find((prompt) => prompt.id === selectedId);
}

function select(id?: string) {
  selectedId = id;
  const prompt = currentPrompt();
  editor.hidden = !prompt;
  empty.hidden = Boolean(prompt);
  empty.textContent = "点击「新建指令」添加提示词。";
  if (prompt) {
    title.value = prompt.title;
    content.value = prompt.content;
    colors.querySelector<HTMLInputElement>(`input[value="${prompt.color}"]`)!.checked = true;
  }
  remove.textContent = id !== undefined && id === newId ? "丢弃草稿" : "删除指令";
  window.clearTimeout(copyTimer);
  copyFeedback.textContent = "";
  renderTiles();
}

function draftFromFields(): Prompt | undefined {
  if (!selectedId) return;
  const color = colors.querySelector<HTMLInputElement>("input:checked")?.value as Color | undefined;
  return { id: selectedId, title: title.value, content: content.value, color: color ?? "blue" };
}

editor.addEventListener("input", () => {
  const prompt = draftFromFields();
  if (prompt) drafts.set(prompt.id, prompt);
});

add.addEventListener("click", () => {
  if (!newId) {
    const bytes = crypto.getRandomValues(new Uint8Array(16));
    newId = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
    drafts.set(newId, { id: newId, title: "", content: "", color: "blue" });
  }
  select(newId);
  title.focus();
});

function applyMenuStatus(status: MenuStatus) {
  autostartEnabled = status.autostart;
  autostart.checked = autostartEnabled ?? false;
  autostart.disabled = pendingAutostart || autostartEnabled === null;
  if (status.warning) notify(status.warning, true);
}

function sync(change?: Change) {
  return invoke<MenuStatus>("sync_tray_menu", change ? { change } : {});
}

function setEditingBusy(busy: boolean) {
  pendingEdit = busy;
  save.disabled = busy;
  remove.disabled = busy;
  title.readOnly = busy;
  content.readOnly = busy;
  colors.disabled = busy;
}

editor.addEventListener("submit", async (event) => {
  event.preventDefault();
  const prompt = draftFromFields();
  if (!prompt || pendingEdit) return;
  prompt.title = prompt.title.trim();
  notify("");
  setEditingBusy(true);
  try {
    const status = await sync({ kind: "save", prompt });
    const index = prompts.findIndex((item) => item.id === prompt.id);
    if (index < 0) prompts.push(prompt);
    else prompts[index] = prompt;
    drafts.delete(prompt.id);
    if (newId === prompt.id) newId = undefined;
    if (selectedId === prompt.id) title.value = prompt.title;
    if (!editor.hidden) remove.textContent = selectedId && selectedId === newId ? "丢弃草稿" : "删除指令";
    renderTiles();
    notify("已保存");
    applyMenuStatus(status);
  } catch (error) {
    notify(String(error), true);
  } finally {
    setEditingBusy(false);
  }
});

copy.addEventListener("click", async () => {
  const id = selectedId;
  const text = content.value;
  notify("");
  if (!text.trim()) {
    notify("请输入提示词内容。", true);
    content.focus();
    return;
  }
  try {
    await invoke("plugin:clipboard-manager|write_text", { text });
    if (id !== selectedId) return;
    window.clearTimeout(copyTimer);
    copyFeedback.textContent = "已复制 ✓";
    copyTimer = window.setTimeout(() => { copyFeedback.textContent = ""; }, 1500);
  } catch (error) {
    notify(`无法写入剪贴板：${String(error)}`, true);
  }
});

let deleteId: string | undefined;
remove.addEventListener("click", () => {
  const prompt = currentPrompt();
  if (!prompt || pendingEdit) return;
  deleteId = prompt.id;
  const isNew = prompt.id === newId;
  element("delete-heading").textContent = isNew ? "丢弃草稿" : "删除指令";
  element("delete-message").textContent = isNew ? "丢弃这条尚未保存的指令？" : `删除「${prompt.title}」？`;
  dialog.querySelector("button[value=delete]")!.textContent = isNew ? "确认丢弃" : "确认删除";
  dialog.returnValue = "";
  dialog.showModal();
});

dialog.addEventListener("close", async () => {
  const id = deleteId;
  deleteId = undefined;
  if (dialog.returnValue !== "delete" || !id || pendingEdit) return;
  setEditingBusy(true);
  notify("");
  try {
    const status = id === newId ? undefined : await sync({ kind: "delete", id });
    const index = prompts.findIndex((item) => item.id === id);
    if (index !== -1) prompts.splice(index, 1);
    if (id === newId) newId = undefined;
    drafts.delete(id);
    if (selectedId === id) select(prompts[Math.max(0, index - 1)]?.id);
    else renderTiles();
    notify(status ? "已删除" : "已丢弃草稿");
    if (status) applyMenuStatus(status);
  } catch (error) {
    notify(String(error), true);
  } finally {
    setEditingBusy(false);
  }
});

autostart.addEventListener("change", async () => {
  const enabled = autostart.checked;
  pendingAutostart = true;
  autostart.disabled = true;
  notify("");
  try {
    applyMenuStatus(await sync({ kind: "autostart", enabled }));
  } catch (error) {
    autostart.checked = autostartEnabled ?? false;
    notify(String(error), true);
    try {
      applyMenuStatus(await sync());
    } catch {
      // Keep the last known state when the status query fails.
    }
  } finally {
    pendingAutostart = false;
    autostart.disabled = autostartEnabled === null;
  }
});

async function initialize() {
  try {
    const events = window.__TAURI__?.event;
    await events?.listen("editor-hidden", () => { void suspendEditor(); });
    await events?.listen<MenuStatus>("autostart-changed", ({ payload }) => applyMenuStatus(payload));
    await events?.listen<string>("tray-error", ({ payload }) => notify(payload, true));
    applyMenuStatus(await sync());
    prompts = await invoke<Prompt[]>("load_prompts");
    const workspace = await invoke<EditorState | null>("restore_editor");
    if (workspace) {
      for (const draft of workspace.drafts) drafts.set(draft.id, draft);
      newId = workspace.newId ?? undefined;
    }
    select(workspace?.selectedId ?? prompts[0]?.id);
    add.disabled = false;
  } catch (error) {
    empty.textContent = "无法加载指令库。";
    notify(String(error), true);
  } finally {
    tiles.setAttribute("aria-busy", "false");
  }
}

async function suspendEditor() {
  try {
    while (pendingEdit || pendingAutostart) {
      await new Promise<void>((done) => window.setTimeout(done, 30));
    }
    await invoke("suspend_editor", {
      workspace: {
        selectedId: selectedId ?? null,
        newId: newId ?? null,
        drafts: [...drafts.values()],
      },
    });
  } catch (error) {
    notify(String(error), true);
  }
}

window.addEventListener("focus", () => {
  if (!pendingAutostart && window.__TAURI__) {
    void sync().then(applyMenuStatus).catch((error) => notify(String(error), true));
  }
});
void initialize();
