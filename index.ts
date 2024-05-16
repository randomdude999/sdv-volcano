import wasm_init, { main_update, GameSettings } from './pkg/sdv_volcano.js';

async function main() {
    await wasm_init();

    const update = () => {
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
        settings.days_played = Math.max(+days_played_el.value, 0);
        settings.special_charm = special_charm_el.checked;
        const res = main_update(settings);
        document.getElementById("temp").innerText = res;
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
