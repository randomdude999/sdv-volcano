import wasm_init, { main_update, GameSettings, render_map } from './pkg/sdv_volcano.js';

function get_settings() {
    const seed_el = document.getElementById("seed") as HTMLInputElement;
    const legacy_rng_el = document.getElementById("legacy_rng") as HTMLInputElement;
    const has_caldera_el = document.getElementById("has_caldera") as HTMLInputElement;
    const post_1_6_4_el = document.getElementById("post_1_6_4") as HTMLInputElement;
    const cracked_coconut_el = document.getElementById("cracked_coconut") as HTMLInputElement;
    const special_charm_el = document.getElementById("special_charm") as HTMLInputElement;
    const max_luck_lvl_el = document.getElementById("max_luck_lvl") as HTMLInputElement;
    const days_played_el = document.getElementById("days_played") as HTMLInputElement;
    //document.getElementById("caldera_wrapper").classList.toggle("hidden", !post_1_6_4_el.checked);
    // ^ actually this just looks distracting
    const settings = new GameSettings();
    settings.seed = +seed_el.value;
    settings.legacy_rng = legacy_rng_el.checked;
    settings.post_1_6_4 = post_1_6_4_el.checked;
    settings.has_caldera = has_caldera_el.checked;
    settings.cracked_golden_coconut = cracked_coconut_el.checked;
    settings.max_luck_lvl = Math.max(+max_luck_lvl_el.value, 0);
    settings.days_played = Math.max(+days_played_el.value, 1);
    settings.special_charm = special_charm_el.checked;
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
    }

    reset_canvas();

    const select_layout_handler = (lvl: number, layout: number) => {
        const use_big = (document.getElementById("big_tiles") as HTMLInputElement).checked;
        const settings = get_settings();
        const tile_size = use_big ? 16 : 8;
        map_canvas.width = map_canvas.height = 64 * tile_size;
        const tiles_img = use_big ? big_tiles : small_tiles;
        const hints = render_map(settings, lvl, layout, map_ctx, tiles_img, tile_size);
        last_lvl = lvl;
        last_layout = layout;
        map_canvas.classList.remove("hidden");
        map_placeholder.classList.add("hidden");
        map_notes.innerHTML = hints;
    };

    document.getElementById("big_tiles").addEventListener("input", () => {
        if(typeof last_lvl == 'number') {
            select_layout_handler(last_lvl, last_layout);
        }
    });

    const update = () => {
        const settings = get_settings();
        const res = main_update(settings);
        document.getElementById("temp").innerText = res;
        reset_canvas();
        for(const el of document.getElementsByClassName("layout-btn")) {
            el.addEventListener("click", (ev) => {
                const el = ev.target as HTMLElement;
                const lvl = +el.getAttribute("data-lvl");
                const layout = +el.getAttribute("data-layout");
                select_layout_handler(lvl, layout);
            });
        }
    };

    update();

    //const el = document.getElementById("b");
    //el!.addEventListener("click", update);
    for(const el of document.getElementsByClassName("setting")) {
        el.addEventListener("input", update);
    }

    document.getElementById("spam")?.addEventListener("click", () => {
        for(let i = 0; i < 1000; i++) update();
    });
}

main();
