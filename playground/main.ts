import init, { State, version } from "@cooklang/cooklang-ts";

declare global {
  interface Window {
    ace: any;
  }
}

async function run(): Promise<void> {
  await init();

  const editor = window.ace.edit("editor", {
    wrap: true,
    printMargin: false,
    fontSize: 16,
    fontFamily: "Jetbrains Mono",
    placeholder: "Write your recipe here",
  });
  
  const input = window.sessionStorage.getItem("input") ?? "Write your @recipe here!";
  editor.setValue(input);
  
  const output = document.getElementById("output") as HTMLPreElement;
  const errors = document.getElementById("errors") as HTMLPreElement;
  const errorsDetails = document.getElementById("errors-details") as HTMLDetailsElement;
  const parserSelect = document.getElementById("parserSelect") as HTMLSelectElement;
  const jsonCheckbox = document.getElementById("json") as HTMLInputElement;
  const servings = document.getElementById("servings") as HTMLInputElement;
  const loadUnits = document.getElementById("loadUnits") as HTMLInputElement;
  const versionElement = document.getElementById("version") as HTMLPreElement;
  
  if (versionElement) {
    versionElement.textContent = version();
  }

  const state = new State();

  const search = new URLSearchParams(window.location.search);
  if (search.has("json")) {
    jsonCheckbox.checked = search.get("json") === "true";
  }
  if (search.has("loadUnits")) {
    const load = search.get("loadUnits") === "true";
    state.load_units = load;
  }
  loadUnits.checked = state.load_units;
  if (search.has("extensions")) {
    state.extensions = Number(search.get("extensions"));
  }
  let mode = search.get("mode") || localStorage.getItem("mode");
  if (mode !== null) {
    parserSelect.value = mode;
    setMode(mode);
  }

  function parse(): void {
    const input = editor.getValue();
    window.sessionStorage.setItem("input", input);
    switch (parserSelect.value) {
      case "full": {
        const { value, error } = state.parse_full(input, jsonCheckbox.checked);
        output.textContent = value;
        errors.innerHTML = error;
        break;
      }
      case "events": {
        const events = state.parse_events(input);
        output.textContent = events;
        errors.innerHTML = "";
        break;
      }
      case "ast": {
        const { value, error } = state.parse_ast(input, jsonCheckbox.checked);
        output.textContent = value;
        errors.innerHTML = error;
        break;
      }
      case "render": {
        const { value, error } = state.parse_render(input, servings.value.length === 0 ? null : servings.valueAsNumber);
        output.innerHTML = value;
        errors.innerHTML = error;
        break;
      }
      case "stdmeta": {
        const { value, error } = state.std_metadata(input);
        output.innerHTML = value;
        errors.innerHTML = error;
        break;
      }
    }
    errorsDetails.open = errors.childElementCount !== 0;
  }

  editor.on("change", debounce(parse, 100));
  parserSelect.addEventListener("change", (ev) => setMode((ev.target as HTMLSelectElement).value));
  jsonCheckbox.addEventListener("change", (ev) => {
    const params = new URLSearchParams(window.location.search);
    const target = ev.target as HTMLInputElement;
    if (target.checked) {
      params.set("json", "true");
    } else {
      params.delete("json");
    }
    window.history.replaceState(
      null,
      "",
      window.location.pathname + "?" + params.toString()
    );
    parse();
  });
  
  loadUnits.addEventListener("change", (ev) => {
    const params = new URLSearchParams(window.location.search);
    const target = ev.target as HTMLInputElement;
    state.load_units = !!target.checked;
    if (target.checked) {
      params.delete("loadUnits");
    } else {
      params.set("loadUnits", "false");
    }
    window.history.replaceState(
      null,
      "",
      window.location.pathname + "?" + params.toString()
    );
    parse();
  });
  
  servings.addEventListener("change", () => parse());

  const extensionsContainer = document.getElementById("extensions-container") as HTMLDivElement;

  const extensions: [string, number][] = [
    ["COMPONENT_MODIFIERS", 1 << 1],
    ["COMPONENT_ALIAS", 1 << 3],
    ["ADVANCED_UNITS", 1 << 5],
    ["MODES", 1 << 6],
    ["INLINE_QUANTITIES", 1 << 7],
    ["RANGE_VALUES", 1 << 9],
    ["TIMER_REQUIRES_TIME", 1 << 10],
    ["INTERMEDIATE_PREPARATIONS", 1 << 11 | 1 << 1]
  ];

  extensions.forEach(([e, bits]) => {
    const elem = document.createElement("input");
    elem.setAttribute("type", "checkbox");
    elem.setAttribute("id", e);
    elem.setAttribute("data-ext-bits", bits.toString());
    elem.checked = (state.extensions & bits) === bits;
    const label = document.createElement("label");
    label.setAttribute("for", e);
    label.textContent = e;
    const container = document.createElement("div");
    container.appendChild(elem);
    container.appendChild(label);
    extensionsContainer.appendChild(container);

    elem.addEventListener("change", updateExtensions);
  });
  
  function updateExtensions(): void {
    let e = 0;
    document.querySelectorAll("[data-ext-bits]:checked").forEach(elem => {
      const bits = Number((elem as HTMLElement).getAttribute("data-ext-bits"));
      e |= bits;
    });
    console.log(e);
    state.extensions = e;

    const params = new URLSearchParams(window.location.search);
    params.set("extensions", e.toString());
    window.history.replaceState(
      null,
      "",
      window.location.pathname + "?" + params.toString()
    );
    parse();
  }

  function setMode(mode: string): void {
    const params = new URLSearchParams(window.location.search);
    params.set("mode", mode);
    window.history.replaceState(
      null,
      "",
      window.location.pathname + "?" + params.toString()
    );
    const jsonContainer = document.getElementById("jsoncontainer") as HTMLDivElement;
    const servingsContainer = document.getElementById("servingscontainer") as HTMLDivElement;
    jsonContainer.hidden = mode === "render" || mode === "events";
    servingsContainer.hidden = mode !== "render";
    localStorage.setItem("mode", mode);
    parse();
  }

  editor.focus();
  parse();
}

function debounce(fn: () => void, delay: number): () => void {
  let timer: number | null = null;
  let first = true;
  return () => {
    if (first) {
      fn();
      first = false;
    } else {
      if (timer !== null) {
        clearTimeout(timer);
      }
      timer = setTimeout(fn, delay);
    }
  };
}

run();