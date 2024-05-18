import wasm_init, { GameSettings, main_update, render_map } from "./pkg/sdv_volcano.js";

function get_settings() {
    const get_el = (id: string) => document.getElementById(id) as HTMLInputElement;
    //document.getElementById("caldera_wrapper").classList.toggle("hidden", !post_1_6_4_el.checked);
    // ^ actually this just looks distracting
    const settings = new GameSettings();
    settings.seed = +get_el("seed").value;
    settings.legacy_rng = get_el("legacy_rng").checked;
    settings.post_1_6_4 = get_el("post_1_6_4").checked;
    settings.has_caldera = get_el("has_caldera").checked;
    settings.cracked_golden_coconut = get_el("cracked_coconut").checked;
    settings.max_luck_lvl = Math.max(+get_el("max_luck_lvl").value, 0);
    settings.days_played = Math.max(+get_el("days_played").value, 1);
    settings.special_charm = get_el("special_charm").checked;
    return settings;
}

async function main() {
    await wasm_init();

    const small_tiles = new Image();
    small_tiles.src = "icons/maptiles_8.png";
    const big_tiles = new Image();
    big_tiles.src = "icons/maptiles_16.png";
    const map_canvas = document.getElementById("map-canvas") as HTMLCanvasElement;
    const map_ctx = map_canvas.getContext("2d");
    const map_placeholder = document.getElementById("map-placeholder");
    const map_notes = document.getElementById("map-notes");

    let last_lvl = null;
    let last_layout = null;

    const reset_canvas = () => {
        last_layout = null;
        last_lvl = null;
        map_canvas.classList.add("hidden");
        map_placeholder.classList.remove("hidden");
        map_notes.innerHTML = "";
    };

    reset_canvas();

    const select_layout_handler = (lvl: number, layout: number) => {
        const use_big = (document.getElementById("big_tiles") as HTMLInputElement)
            .checked;
        const settings = get_settings();
        const tile_size = use_big ? 16 : 8;
        map_canvas.width = map_canvas.height = 64 * tile_size;
        const tiles_img = use_big ? big_tiles : small_tiles;
        const notes = render_map(settings, lvl, layout, map_ctx, tiles_img, tile_size);
        last_lvl = lvl;
        last_layout = layout;
        map_canvas.classList.remove("hidden");
        map_placeholder.classList.add("hidden");
        map_notes.innerHTML = notes;
    };

    document.getElementById("big_tiles").addEventListener("input", () => {
        if (typeof last_lvl == "number") {
            select_layout_handler(last_lvl, last_layout);
        }
    });

    const update = () => {
        const settings = get_settings();
        const res = main_update(settings);
        document.getElementById("temp").innerText = res;
        reset_canvas();
        for (const el of document.getElementsByClassName("layout-btn")) {
            el.addEventListener("click", (ev) => {
                const el = ev.target as HTMLElement;
                const lvl = +el.getAttribute("data-lvl");
                const layout = +el.getAttribute("data-layout");
                select_layout_handler(lvl, layout);
            });
        }
    };

    update();

    for (const el of document.getElementsByClassName("setting")) {
        el.addEventListener("input", update);
    }

    document.getElementById("spam")?.addEventListener("click", () => {
        for (let i = 0; i < 1000; i++) update();
    });
}

main();
