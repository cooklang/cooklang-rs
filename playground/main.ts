import {Parser, version, CooklangParser} from "@cooklang/cooklang";
import {defineCooklangMode} from "./ace-cooklang";
import {examples, defaultExample} from "./examples";
import {renderPreview, esc} from "./preview";

declare global {
    interface Window {
        ace: any;
    }
}

type Mode = "preview" | "full" | "events" | "ast" | "stdmeta";
const MODES: Mode[] = ["preview", "full", "events", "ast", "stdmeta"];

// Old links (cooklang.org uses ?mode=render) keep working.
function normaliseMode(mode: string | null): Mode | null {
    if (mode === "render" || mode === "render2") return "preview";
    return MODES.includes(mode as Mode) ? (mode as Mode) : null;
}

const EXTENSIONS: [string, string, number][] = [
    ["COMPONENT_MODIFIERS", "Component modifiers (@&, @?, @-)", 1 << 1],
    ["COMPONENT_ALIAS", "Aliases (@name|alias)", 1 << 3],
    ["ADVANCED_UNITS", "Advanced units", 1 << 5],
    ["MODES", "Modes ([mode]: …)", 1 << 6],
    ["INLINE_QUANTITIES", "Inline quantities", 1 << 7],
    ["RANGE_VALUES", "Ranges {2-3}", 1 << 9],
    ["TIMER_REQUIRES_TIME", "Timers require a time", 1 << 10],
    ["INTERMEDIATE_PREPARATIONS", "Intermediate preparations", (1 << 11) | (1 << 1)],
];

// Recipe text in the URL hash (#r=…) so a shared link carries the recipe.
function encodeRecipe(text: string): string {
    let bin = "";
    new TextEncoder().encode(text).forEach((b) => (bin += String.fromCharCode(b)));
    return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function decodeRecipe(value: string): string | null {
    try {
        const bin = atob(value.replace(/-/g, "+").replace(/_/g, "/"));
        return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
    } catch {
        return null;
    }
}

function setParam(key: string, value: string | null): void {
    const params = new URLSearchParams(window.location.search);
    if (value === null) params.delete(key);
    else params.set(key, value);
    const query = params.toString();
    window.history.replaceState(null, "", window.location.pathname + (query ? "?" + query : "") + window.location.hash);
}

function initialInput(): string {
    const hash = new URLSearchParams(window.location.hash.slice(1)).get("r");
    const fromHash = hash ? decodeRecipe(hash) : null;
    if (fromHash !== null) return fromHash;
    try {
        const saved = window.sessionStorage.getItem("input");
        if (saved !== null) return saved;
    } catch {}
    return defaultExample.source;
}

async function run(): Promise<void> {
    defineCooklangMode();
    const editor = window.ace.edit("editor", {
        mode: "ace/mode/cooklang",
        wrap: true,
        showPrintMargin: false,
        fontSize: 14,
        fontFamily: "JetBrains Mono, ui-monospace, monospace",
        placeholder: "Write your recipe here",
        highlightActiveLine: false,
        tabSize: 2,
    });
    editor.renderer.setScrollMargin(12, 12);
    editor.renderer.setPadding(12);
    editor.setValue(initialInput(), -1);

    const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
    const output = $<HTMLDivElement>("output");
    const errors = $<HTMLPreElement>("errors");
    const status = $<HTMLDetailsElement>("status");
    const statusSummary = $<HTMLElement>("statusSummary");
    const jsonCheckbox = $<HTMLInputElement>("json");
    const jsonContainer = $<HTMLLabelElement>("jsoncontainer");
    const loadUnits = $<HTMLInputElement>("loadUnits");
    const exampleSelect = $<HTMLSelectElement>("exampleSelect");
    const settingsBtn = $<HTMLButtonElement>("settingsBtn");
    const settings = $<HTMLDivElement>("settings");
    const shareBtn = $<HTMLButtonElement>("shareBtn");
    const shareLabel = $<HTMLSpanElement>("shareLabel");
    const workspace = document.querySelector(".workspace") as HTMLElement;

    $<HTMLSpanElement>("version").textContent = version();

    const parser = new Parser();
    const parser2 = new CooklangParser();
    let mode: Mode = "preview";
    let targetServings: number | null = null;

    // ---- URL / stored state
    const search = new URLSearchParams(window.location.search);
    jsonCheckbox.checked = search.get("json") === "true";
    if (search.has("loadUnits")) {
        const load = search.get("loadUnits") === "true";
        parser.load_units = load;
        parser2.units = load;
    }
    loadUnits.checked = parser.load_units;
    if (search.has("extensions")) {
        parser.extensions = Number(search.get("extensions"));
        parser2.extensions = Number(search.get("extensions"));
    }
    let storedMode: string | null = null;
    try { storedMode = localStorage.getItem("mode"); } catch {}
    mode = normaliseMode(search.get("mode")) ?? normaliseMode(storedMode) ?? "preview";

    // ---- Examples
    for (const ex of examples) {
        const opt = document.createElement("option");
        opt.value = ex.id;
        opt.textContent = ex.label;
        exampleSelect.appendChild(opt);
    }
    exampleSelect.addEventListener("change", () => {
        const ex = examples.find((e) => e.id === exampleSelect.value);
        if (!ex) return;
        targetServings = null;
        editor.setValue(ex.source, -1);
        exampleSelect.value = "";
        editor.focus();
    });

    // ---- Status bar
    function setStatus(report: string): void {
        const text = report.replace(/<[^>]*>/g, "").trim();
        errors.innerHTML = report;
        status.classList.remove("ok", "warn", "error");
        if (!text) {
            statusSummary.textContent = "✓ No issues";
            status.classList.add("ok");
            status.open = false;
            return;
        }
        const hasError = /\berror\b/i.test(text);
        status.classList.add(hasError ? "error" : "warn");
        statusSummary.textContent = hasError ? "✕ Errors — click to see them" : "⚠ Warnings — click to see them";
        if (hasError) status.open = true;
    }

    function code(content: string, html = false): string {
        return `<pre class="code-out">${html ? content : esc(content)}</pre>`;
    }

    // ---- Parse + render
    function parse(): void {
        const input = editor.getValue();
        try { window.sessionStorage.setItem("input", input); } catch {}
        try {
            render(input);
        } catch (err) {
            // A Rust panic surfaces as a wasm RuntimeError. Say so instead of going blank.
            output.innerHTML = `<div class="crash"><strong>The parser crashed on this recipe.</strong><p>Other tabs may still work. Please <a href="https://github.com/cooklang/cooklang-rs/issues">report it</a> with a Share link.</p><pre>${esc(String(err))}</pre></div>`;
            setStatus("");
            statusSummary.textContent = "✕ Parser crashed";
            status.classList.remove("ok");
            status.classList.add("error");
        }
    }

    function render(input: string): void {
        switch (mode) {
            case "preview": {
                // parse() takes a scale factor; the stepper works in servings.
                let [recipe, report] = parser2.parse(input, null);
                const base = typeof recipe.servings === "number" && recipe.servings > 0 ? recipe.servings : 1;
                if (targetServings !== null && targetServings !== base) {
                    [recipe, report] = parser2.parse(input, targetServings / base);
                }
                output.innerHTML = renderPreview(recipe, {servings: targetServings});
                setStatus(report);
                break;
            }
            case "full": {
                const {value, error} = parser.parse_full(input, jsonCheckbox.checked);
                output.innerHTML = code(value);
                setStatus(error);
                break;
            }
            case "events": {
                output.innerHTML = code(parser.parse_events(input));
                setStatus("");
                break;
            }
            case "ast": {
                const {value, error} = parser.parse_ast(input, jsonCheckbox.checked);
                output.innerHTML = code(value);
                setStatus(error);
                break;
            }
            case "stdmeta": {
                const {value, error} = parser.std_metadata(input);
                output.innerHTML = code(value, true);
                setStatus(error);
                break;
            }
        }
    }

    // Servings stepper lives inside the rendered preview.
    output.addEventListener("click", (ev) => {
        const btn = (ev.target as HTMLElement).closest("[data-servings-delta]") as HTMLElement | null;
        if (!btn) return;
        const [recipe] = parser2.parse(editor.getValue(), null);
        const base = targetServings ?? (typeof recipe.servings === "number" && recipe.servings > 0 ? recipe.servings : 1);
        targetServings = Math.max(1, base + Number(btn.dataset.servingsDelta));
        parse();
    });

    // ---- Tabs
    const tabs = Array.from(document.querySelectorAll<HTMLButtonElement>("[data-mode]"));
    function setMode(next: Mode): void {
        mode = next;
        tabs.forEach((t) => t.setAttribute("aria-selected", String(t.dataset.mode === mode)));
        jsonContainer.hidden = !(mode === "full" || mode === "ast");
        setParam("mode", mode === "preview" ? "render" : mode);
        try { localStorage.setItem("mode", mode); } catch {}
        parse();
    }
    tabs.forEach((t) => t.addEventListener("click", () => setMode(t.dataset.mode as Mode)));

    // ---- Mobile: Edit | Preview switch
    const paneButtons = Array.from(document.querySelectorAll<HTMLButtonElement>("[data-pane]"));
    paneButtons.forEach((b) =>
        b.addEventListener("click", () => {
            workspace.dataset.mobilePane = b.dataset.pane;
            paneButtons.forEach((p) => p.setAttribute("aria-selected", String(p === b)));
            if (b.dataset.pane === "edit") editor.resize();
        })
    );

    // ---- Settings popover
    settingsBtn.addEventListener("click", (ev) => {
        ev.stopPropagation();
        settings.hidden = !settings.hidden;
        settingsBtn.setAttribute("aria-expanded", String(!settings.hidden));
    });
    document.addEventListener("click", (ev) => {
        if (!settings.hidden && !settings.contains(ev.target as Node)) {
            settings.hidden = true;
            settingsBtn.setAttribute("aria-expanded", "false");
        }
    });

    jsonCheckbox.addEventListener("change", () => {
        setParam("json", jsonCheckbox.checked ? "true" : null);
        parse();
    });
    loadUnits.addEventListener("change", () => {
        parser.load_units = loadUnits.checked;
        parser2.units = loadUnits.checked;
        setParam("loadUnits", loadUnits.checked ? null : "false");
        parse();
    });

    const extensionsContainer = $<HTMLDivElement>("extensions-container");
    EXTENSIONS.forEach(([id, label, bits]) => {
        const wrap = document.createElement("label");
        wrap.className = "check";
        wrap.title = id;
        const box = document.createElement("input");
        box.type = "checkbox";
        box.id = id;
        box.dataset.extBits = String(bits);
        box.checked = (parser.extensions & bits) === bits;
        box.addEventListener("change", updateExtensions);
        wrap.append(box, " " + label);
        extensionsContainer.appendChild(wrap);
    });
    function updateExtensions(): void {
        let e = 0;
        document.querySelectorAll<HTMLInputElement>("[data-ext-bits]:checked").forEach((el) => {
            e |= Number(el.dataset.extBits);
        });
        parser.extensions = e;
        parser2.extensions = e;
        setParam("extensions", String(e));
        parse();
    }

    // ---- Share
    shareBtn.addEventListener("click", async () => {
        const url = new URL(window.location.href);
        url.hash = "r=" + encodeRecipe(editor.getValue());
        window.history.replaceState(null, "", url.toString());
        try {
            await navigator.clipboard.writeText(url.toString());
            shareLabel.textContent = "Link copied";
        } catch {
            shareLabel.textContent = "Link in address bar";
        }
        setTimeout(() => (shareLabel.textContent = "Share"), 2000);
    });

    editor.on("change", debounce(parse, 120));
    setMode(mode);
    editor.focus();
}

function debounce(fn: () => void, delay: number): () => void {
    let timer: number | undefined;
    return () => {
        clearTimeout(timer);
        timer = window.setTimeout(fn, delay);
    };
}

run();
