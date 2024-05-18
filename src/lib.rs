use std::fmt::Write;
use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

mod map_data;
mod rng;

// this is part of stdlib in nightly
fn f64_next_up(x: f64) -> f64 {
    // this version only works for finite positive floats
    f64::from_bits(x.to_bits() + 1)
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MapTile {
    Floor = 0,
    Lava = 1,
    Wall = 2,
    Enter = 3,
    Exit = 4,
    SetPiece = 5,
    SwitchLocation = 6,
    MonsterSpawn = 7,
}

#[wasm_bindgen]
#[derive(Copy, Clone, Default)]
pub struct GameSettings {
    pub seed: i32,
    pub legacy_rng: bool,
    pub has_caldera: bool,
    pub post_1_6_4: bool,
    pub cracked_golden_coconut: bool,
    pub special_charm: bool,
    pub days_played: u32,
    pub max_luck_lvl: u32,
}
#[wasm_bindgen]
impl GameSettings {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Clone)]
struct Tilemap([[MapTile; 64]; 64]);
impl Index<(i32, i32)> for Tilemap {
    type Output = MapTile;
    fn index(&self, index: (i32, i32)) -> &Self::Output {
        &self.0[index.1 as usize][index.0 as usize]
    }
}
impl IndexMut<(i32, i32)> for Tilemap {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Self::Output {
        &mut self.0[index.1 as usize][index.0 as usize]
    }
}

impl Tilemap {
    fn load(layout_id: u32, flip_x: bool) -> Self {
        let layout_sz: usize = 64 * 64;
        let layout_off = layout_sz * layout_id as usize;
        let base = &map_data::LAYOUTS[layout_off..layout_off + layout_sz];
        let out: [[MapTile; 64]; 64] = if flip_x {
            std::array::from_fn(|y| {
                let mut row =
                    std::array::from_fn(|x| unsafe { std::mem::transmute(base[y * 64 + x]) });
                row.reverse();
                row
            })
        } else {
            std::array::from_fn(|y| {
                std::array::from_fn(|x| unsafe { std::mem::transmute(base[y * 64 + x]) })
            })
        };
        Tilemap(out)
    }
}

struct DungeonFloorState {
    rng: rng::DotnetRng,
    map: Tilemap,
    set_pieces: Vec<(i32, i32, i32)>,
    settings: GameSettings,
    level: i32,
    layout_id: u32,
    min_luck: f64,
    max_luck: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
enum CommonChest {
    CinderShards,
    GoldenCoconut,
    TaroTuber,
    PineappleSeeds,
    ProtectionRing,
    SoulSapperRing,
    DwarfSword,
    DwarfHammer,
    DwarfDagger,
}

impl Display for CommonChest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CinderShards => "Cinder Shard (3)",
            Self::GoldenCoconut => "Golden Coconut",
            Self::TaroTuber => "Taro Tuber (8)",
            Self::PineappleSeeds => "Pineapple Seeds (5)",
            Self::ProtectionRing => "Protection Ring",
            Self::SoulSapperRing => "Soul Sapper Ring",
            Self::DwarfSword => "Dwarf Sword",
            Self::DwarfHammer => "Dwarf Hammer",
            Self::DwarfDagger => "Dwarf Dagger",
        })
    }
}

impl CommonChest {
    fn generate(seed: i32, settings: GameSettings) -> Self {
        let mut rng = rng::DotnetRng::new(seed);
        rng.next(); // one roll used for rare/normal check
        let ind = loop {
            let ind = rng.next_range(7);
            if ind == 1 && !settings.cracked_golden_coconut {
                continue;
            }
            break ind;
        };
        match ind {
            0 => Self::CinderShards,
            1 => Self::GoldenCoconut,
            2 => Self::TaroTuber,
            3 => Self::PineappleSeeds,
            4 => Self::ProtectionRing,
            5 => Self::SoulSapperRing,
            6 => {
                [Self::DwarfSword, Self::DwarfHammer, Self::DwarfDagger][rng.next_range(3) as usize]
            }
            _ => unreachable!(),
        }
    }
    fn get_icon(&self) -> &'static str {
        match self {
            CommonChest::CinderShards => "cinder_shard",
            CommonChest::GoldenCoconut => "golden_coconut",
            CommonChest::TaroTuber => "taro_tuber",
            CommonChest::PineappleSeeds => "pineapple_seeds",
            CommonChest::ProtectionRing => "protection_ring",
            CommonChest::SoulSapperRing => "soul_sapper_ring",
            CommonChest::DwarfSword => "dwarf_sword",
            CommonChest::DwarfHammer => "dwarf_hammer",
            CommonChest::DwarfDagger => "dwarf_dagger",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
enum RareChest {
    CinderShards,
    MermaidBoots,
    DragonscaleBoots,
    GoldenCoconuts,
    PhoenixRing,
    HotJavaRing,
    DragontoothCutlass,
    DragontoothClub,
    DragontoothShiv,
    DeluxePirateHat,
    OstrichEgg,
}

impl Display for RareChest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CinderShards => "Cinder Shard (10)",
            Self::MermaidBoots => "Mermaid Boots",
            Self::DragonscaleBoots => "Dragonscale Boots",
            Self::GoldenCoconuts => "Golden Coconut (3)",
            Self::PhoenixRing => "Phoenix Ring",
            Self::HotJavaRing => "Hot Java Ring",
            Self::DragontoothCutlass => "Dragontooth Cutlass",
            Self::DragontoothClub => "Dragontooth Club",
            Self::DragontoothShiv => "Dragontooth Shiv",
            Self::DeluxePirateHat => "Deluxe Pirate Hat",
            Self::OstrichEgg => "Ostrich Egg",
        })
    }
}

impl RareChest {
    fn generate(seed: i32, settings: GameSettings) -> Self {
        let mut rng = rng::DotnetRng::new(seed);
        rng.next(); // one roll used for rare/normal check
        let ind = loop {
            let ind = rng.next_range(9);
            if ind == 3 && !settings.cracked_golden_coconut {
                continue;
            }
            break ind;
        };
        match ind {
            0 => Self::CinderShards,
            1 => Self::MermaidBoots,
            2 => Self::DragonscaleBoots,
            3 => Self::GoldenCoconuts,
            4 => Self::PhoenixRing,
            5 => Self::HotJavaRing,
            6 => [
                Self::DragontoothCutlass,
                Self::DragontoothClub,
                Self::DragontoothShiv,
            ][rng.next_range(3) as usize],
            7 => Self::DeluxePirateHat,
            8 => Self::OstrichEgg,
            _ => unreachable!(),
        }
    }

    fn get_icon(&self) -> &'static str {
        match self {
            RareChest::CinderShards => "cinder_shard",
            RareChest::MermaidBoots => "mermaid_boots",
            RareChest::DragonscaleBoots => "dragonscale_boots",
            RareChest::GoldenCoconuts => "golden_coconut",
            RareChest::PhoenixRing => "phoenix_ring",
            RareChest::HotJavaRing => "hot_java_ring",
            RareChest::DragontoothCutlass => "dragontooth_cutlass",
            RareChest::DragontoothClub => "dragontooth_club",
            RareChest::DragontoothShiv => "dragontooth_shiv",
            RareChest::DeluxePirateHat => "deluxe_pirate_hat",
            RareChest::OstrichEgg => "ostrich_egg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Goodie {
    DragonTooth,
    CommonChest(CommonChest),
    RareChest(RareChest),
    ChanceChest {
        minluck: f64,
        common: CommonChest,
        rare: RareChest,
    },
}

impl Display for Goodie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Goodie::DragonTooth => write!(f, "Dragon Tooth"),
            Goodie::CommonChest(c) => write!(f, "common chest: {}", c),
            Goodie::RareChest(c) => write!(f, "rare chest: {}", c),
            Goodie::ChanceChest {
                minluck,
                common,
                rare,
            } => {
                write!(
                    f,
                    "luck boost > {:.4}: rare: {}, else common: {}",
                    minluck, rare, common
                )
            }
        }
    }
}

fn format_icon(name: &str) -> String {
    format!("<img src=\"icons/{}.png\" class=icon>", name)
}

impl Goodie {
    fn to_html(&self) -> String {
        match self {
            Goodie::DragonTooth => format!("{} Dragon Tooth", format_icon("dragon_tooth")),
            Goodie::CommonChest(c) => {
                format!(
                    "{} {} {}",
                    format_icon("common_chest"),
                    format_icon(c.get_icon()),
                    c
                )
            }
            Goodie::RareChest(c) => {
                format!(
                    "{} {} {}",
                    format_icon("rare_chest"),
                    format_icon(c.get_icon()),
                    c
                )
            }
            // shouldn't ever be turned into html
            Goodie::ChanceChest { .. } => self.to_string(),
        }
    }
}

impl DungeonFloorState {
    fn new(
        settings: GameSettings,
        level: i32,
        layout_id: u32,
        min_luck: f64,
        max_luck: f64,
    ) -> Self {
        let lvl_mod = if settings.post_1_6_4 {
            level + 1
        } else {
            level
        };
        let gen_seed = rng::stardew_seed_mix(
            settings.legacy_rng,
            &[
                (settings.days_played * lvl_mod as u32) as f64,
                (level * 5152) as f64,
                (settings.seed / 2) as f64,
            ],
        );
        let mut gen_random = rng::DotnetRng::new(rng::stardew_seed_mix(
            settings.legacy_rng,
            &[gen_seed as f64],
        ));
        gen_random.next();
        let mut flip_x = gen_random.next_range(2) == 1;
        if layout_id == 0 || layout_id == 31 {
            flip_x = false;
        }
        Self {
            rng: gen_random,
            map: Tilemap::load(layout_id, flip_x),
            set_pieces: vec![],
            level,
            layout_id,
            settings,
            min_luck,
            max_luck,
        }
    }

    fn load_map(&mut self) -> Vec<Goodie> {
        self.load_map_tiles();
        self.load_set_pieces()
    }

    fn get_tiles(&mut self) -> Tilemap {
        self.map.clone()
    }

    fn load_map_tiles(&mut self) {
        // floor tile type generation (we don't care about the result, but need RNG to sync)
        for _x in 0..64 {
            for _y in 0..64 {
                if self.rng.next_f64() < 0.3_f32 as f64 {
                    self.rng.next();
                    self.rng.next();
                }
            }
        }
    }

    fn load_set_pieces(&mut self) -> Vec<Goodie> {
        let mut buggy = false;
        for x in 0_i32..64 {
            for y in 0_i32..64 {
                if self.map[(x, y)] == MapTile::SetPiece {
                    let mut j = 0_i32;
                    while j < 64
                        && self.map[(x + j, y)] == MapTile::SetPiece
                        && self.map[(x, y + j)] == MapTile::SetPiece
                    {
                        j += 1;
                    }
                    for y in y..y + j {
                        for x in x..x + j {
                            self.map[(x, y)] = MapTile::Floor;
                        }
                    }
                    if !matches!(j, 32 | 16 | 8 | 4 | 3) {
                        println!(
                            "warning: buggy map? layout={} at x={} y={}, size={}",
                            self.layout_id, x, y, j
                        );
                        buggy = true;
                    }
                    let realj = match j {
                        32.. => 32,
                        16.. => 16,
                        8.. => 8,
                        4.. => 4,
                        _ => 3,
                    };
                    self.set_pieces.push((x, y, realj));
                }
            }
        }
        // re-add the real set piece locations for rendering
        for &(x, y, j) in &self.set_pieces {
            for y in y..y + j {
                for x in x..x + j {
                    self.map[(x, y)] = MapTile::SetPiece;
                }
            }
        }
        let mut goodies = vec![];

        for (x, y, set_size) in self.set_pieces.iter().cloned() {
            let (num_rows, num_cols) = map_data::get_piece_sizes(set_size);
            let selected_col = self.rng.next_range(num_cols);
            let selected_row = self.rng.next_range(num_rows);
            if buggy {
                println!(
                    "layout {}: x={} y={} sz={} selected row {}, col {}",
                    self.layout_id, x, y, set_size, selected_row, selected_col
                );
            }
            let events = map_data::get_piece_events()
                .get(&(set_size, selected_row, selected_col))
                .copied();
            let events = events.unwrap_or(&[]);
            for &ev in events {
                match ev {
                    map_data::SetPieceFeature::Rng => {
                        self.rng.next();
                    }
                    map_data::SetPieceFeature::Tooth => {
                        if self.rng.next_f64() < 0.5 {
                            goodies.push(Goodie::DragonTooth);
                        }
                    }
                    map_data::SetPieceFeature::Chest => {
                        // TODO: does not go through seedmix in 1.5
                        // (though, legacy seedmix with 1 arg is mostly identity anyways...)
                        let chest_seed = rng::stardew_seed_mix(
                            self.settings.legacy_rng,
                            &[self.rng.next() as f64],
                        );
                        let mut chest_rng = rng::DotnetRng::new(chest_seed);
                        // roll < (0.1 or 0.5) + luckboost
                        // roll - (0.1 or 0.5) < luckboost
                        // roll - (0.1 or 0.5) < luckmult-1
                        // roll - (0.1 or 0.5) + 1 < luckmult
                        // (though that technically rounds different..)
                        let chest_roll =
                            chest_rng.next_f64() - if self.level == 9 { 0.5 } else { 0.1 } + 1.;
                        if chest_roll < self.min_luck {
                            // only rare
                            goodies.push(Goodie::RareChest(RareChest::generate(
                                chest_seed,
                                self.settings,
                            )));
                        } else if chest_roll >= self.max_luck {
                            // only common
                            goodies.push(Goodie::CommonChest(CommonChest::generate(
                                chest_seed,
                                self.settings,
                            )));
                        } else {
                            // both possible
                            goodies.push(Goodie::ChanceChest {
                                minluck: chest_roll,
                                common: CommonChest::generate(chest_seed, self.settings),
                                rare: RareChest::generate(chest_seed, self.settings),
                            });
                        }
                    }
                }
            }
        }
        goodies
    }
}

fn compute_volcano_layouts(settings: GameSettings) -> Vec<(f64, f64, [u32; 10])> {
    fn compute_inner(
        settings: GameSettings,
        prev: &[u32],
        minluck: f64,
        maxluck: f64,
    ) -> Vec<(f64, f64, [u32; 10])> {
        let level = prev.len();
        let lvl_mod = if settings.post_1_6_4 {
            level + 1
        } else {
            level
        };
        let gen_seed = rng::stardew_seed_mix(
            settings.legacy_rng,
            &[
                (settings.days_played * lvl_mod as u32) as f64,
                (level * 5152) as f64,
                (settings.seed / 2) as f64,
            ],
        );
        let mut lvlbuf = prev.to_vec();
        if level == 0 {
            lvlbuf.push(0);
            return compute_inner(settings, &lvlbuf, minluck, maxluck);
        }
        if level == 5 {
            lvlbuf.push(31);
            return compute_inner(settings, &lvlbuf, minluck, maxluck);
        }
        if level == 9 {
            lvlbuf.push(30);
            // minluck and maxluck are luckMult, which is 1+luckBoost
            return vec![(minluck, maxluck, lvlbuf.try_into().unwrap())];
        }
        let mut valid_layouts: Vec<u32> = (1..30).collect();
        let mut layout_random = rng::DotnetRng::new(rng::stardew_seed_mix(
            settings.legacy_rng,
            &[gen_seed as f64],
        ));
        if level > 1 {
            let special_rng = layout_random.next_f64();
            let special_possible = prev.iter().all(|&x| x < 32);
            if special_possible {
                if special_rng < minluck * 0.5 {
                    // even with the worst possible luck, we still add the special floors
                    valid_layouts.extend(32..38);
                } else if !(special_rng < maxluck * 0.5) {
                    // even with best luck, we do not add the special floors
                } else {
                    // bifurcate!
                    let midpoint = special_rng / 0.5;
                    assert!(minluck < midpoint && midpoint < maxluck);
                    // in there: rng = maxluck * 0.5, so we go to the !< case
                    let mut res1 = compute_inner(settings, prev, minluck, midpoint);
                    // in there: rng ~= minluck * 0.5, but minluck is slightly increased,
                    // so rng < minluck * 0.5
                    let res2 = compute_inner(settings, prev, f64_next_up(midpoint), maxluck);

                    res1.extend(res2);
                    return res1;
                }
            }
        }
        if level > 0 && settings.post_1_6_4 && settings.has_caldera {
            if layout_random.next_f64() < 0.75 {
                valid_layouts.extend(38..58);
            }
        }
        let prev_level = prev[level - 1];
        if let Some(i) = valid_layouts.iter().position(|&x| x == prev_level) {
            valid_layouts.remove(i);
        }
        let the_layout =
            valid_layouts[layout_random.next_range(valid_layouts.len() as i32) as usize];
        lvlbuf.push(the_layout);
        return compute_inner(settings, &lvlbuf, minluck, maxluck);
    }
    // these values are *technically* not exact due to rounding (special charm especially)
    // but we only show them with 4 significant digits anyways
    let mut minluck = -0.1;
    let mut base_maxluck = 0.1;
    if settings.special_charm {
        minluck += 0.025_f32 as f64;
        base_maxluck += 0.025_f32 as f64;
    }
    return compute_inner(
        settings,
        &[],
        1. + minluck / 2.,
        1. + base_maxluck / 2. + 0.035 * (settings.max_luck_lvl as f64),
    );
}

#[allow(unused_macros)]
#[cfg(target_family = "wasm")]
macro_rules! console_log {
    ( $( $x:expr ),* ) => {
        web_sys::console::log_1(&format!( $($x),* ).into())
    };
}
#[allow(unused_macros)]
#[cfg(not(target_family = "wasm"))]
macro_rules! console_log {
    ( $( $x:expr ),* ) => {
        println!( $($x),* )
    };
}

// (minluck, maxluck, item)
type ProbabilityRange<T> = Vec<(f64, f64, T)>;

fn do_dungeon(
    settings: GameSettings,
) -> (
    [ProbabilityRange<u32>; 10],
    [ProbabilityRange<Vec<Goodie>>; 10],
) {
    let mut layouts_poss = [(); 10].map(|_| ProbabilityRange::<u32>::new());
    let mut loots_poss = [(); 10].map(|_| ProbabilityRange::<Vec<Goodie>>::new());
    for (minluck, maxluck, lvls) in compute_volcano_layouts(settings) {
        for (i, &x) in lvls.iter().enumerate() {
            if layouts_poss[i]
                .last()
                .is_some_and(|y| y.2 == x && f64_next_up(y.1) == minluck)
            {
                layouts_poss[i].last_mut().unwrap().1 = maxluck;
            } else {
                layouts_poss[i].push((minluck, maxluck, x));
            }
            let mut h = DungeonFloorState::new(settings, i as i32, x, minluck, maxluck);
            let loot = h.load_map();
            fn handle_loot(
                minluck: f64,
                maxluck: f64,
                loot: Vec<Goodie>,
                loots_poss: &mut Vec<(f64, f64, Vec<Goodie>)>,
            ) {
                assert!(minluck <= maxluck);
                if let Some(ind) = loot
                    .iter()
                    .position(|x| matches!(x, Goodie::ChanceChest { .. }))
                {
                    let Goodie::ChanceChest {
                        minluck: chestluck,
                        common,
                        rare,
                    } = loot[ind]
                    else {
                        unreachable!();
                    };
                    let mut alt_loot = loot;
                    // we're always narrowing the [minluck, maxluck] range here, hence the
                    // .min() / .max() to make sure we keep ourselves in that range
                    if chestluck >= minluck {
                        alt_loot[ind] = Goodie::CommonChest(common);
                        handle_loot(
                            minluck,
                            chestluck.min(maxluck),
                            alt_loot.clone(),
                            loots_poss,
                        );
                    }
                    if chestluck < maxluck {
                        alt_loot[ind] = Goodie::RareChest(rare);
                        handle_loot(
                            f64_next_up(chestluck).max(minluck),
                            maxluck,
                            alt_loot,
                            loots_poss,
                        );
                    }
                    // we always hit at least one of those branches
                } else {
                    // final loot set
                    if loots_poss
                        .last()
                        .is_some_and(|x| x.2 == loot && f64_next_up(x.1) == minluck)
                    {
                        loots_poss.last_mut().unwrap().1 = maxluck;
                    } else {
                        loots_poss.push((minluck, maxluck, loot));
                    }
                }
            }
            handle_loot(minluck, maxluck, loot, &mut loots_poss[i]);
        }
    }
    (layouts_poss, loots_poss)
}

fn display_luck(luck: f64) -> f64 {
    // all computations are in luckMult
    // luckMult = 1 + luck_lvl * 0.035 + daily_luck / 2
    // scale to "adjusted daily luck" or something?
    (luck - 1.) * 2.
}

fn is_mushroom_floor(layout: u32) -> bool {
    layout >= 32 && layout <= 34
}
fn is_monster_floor(layout: u32) -> bool {
    layout >= 35 && layout <= 37
}

#[wasm_bindgen]
pub fn main_update(settings: GameSettings) -> String {
    console_error_panic_hook::set_once();
    let mut out = String::new();
    let (layouts, loots) = do_dungeon(settings);

    let total_seasons = (settings.days_played - 1) / 28;
    let year = total_seasons / 4 + 1;
    let season = total_seasons % 4;
    let day = (settings.days_played - 1) % 28 + 1;
    writeln!(
        out,
        "day: {} {day}, Y{year}",
        ["spring", "summer", "fall", "winter"][season as usize],
    )
    .unwrap();

    fn format_layout(level: usize, layout: u32) -> String {
        let displayname = if is_mushroom_floor(layout) {
            format!("{} {}", format_icon("magma_cap"), layout)
        } else if is_monster_floor(layout) {
            format!("{} {}", format_icon("monster_floor"), layout)
        } else {
            layout.to_string()
        };
        format!(
            "<button data-lvl=\"{}\" data-layout=\"{}\" class=\"layout-btn\">{}</button>",
            level, layout, displayname
        )
    }

    let layouts_disp = String::from_iter(layouts.iter().enumerate().map(|(lvl, this_layouts)| {
        let mut out = String::from("<td>");
        if this_layouts.len() == 1 {
            out += &format_layout(lvl, this_layouts[0].2);
        } else {
            let formatted: Vec<_> = this_layouts
                .iter()
                .map(|&(a, b, c)| {
                    format!(
                        "<span title=\"luck {:.4} to {:.4}\">{}</span>",
                        display_luck(a),
                        display_luck(b),
                        format_layout(lvl, c)
                    )
                })
                .collect();
            out += &formatted.join(" / ");
        }
        out += "</td>";
        out
    }));
    let mut layouts_full = String::from("<table><tr>");
    for i in 0..10 {
        write!(layouts_full, "<td>{}</td>", i).unwrap();
    }
    layouts_full += "</tr><tr>";
    layouts_full += &layouts_disp;
    layouts_full += "</tr></table>";

    let mut goodies_out = String::new();

    macro_rules! out {
        ( $( $x:expr ),* ) => {
            writeln!(goodies_out, $($x),*).unwrap()
        };
    }

    for (i, floor_loot) in loots.into_iter().enumerate() {
        if floor_loot.iter().all(|y| y.2.is_empty()) {
            continue;
        }

        out!("<div><b>floor {}:</b><ul>", i);
        for (minl, maxl, loot) in &floor_loot {
            if floor_loot.len() > 1 {
                out!(
                    "<li>luck {:.4} to {:.4}:</li>",
                    display_luck(*minl),
                    display_luck(*maxl)
                );
            }
            out!("<ul>");
            if loot.is_empty() {
                out!("<li>[nothing]</li>");
            }
            let num_dragon_teeth = loot
                .iter()
                .filter(|x| matches!(x, Goodie::DragonTooth))
                .count();
            if num_dragon_teeth > 1 {
                out!(
                    "<li>{} ({})</li>",
                    Goodie::DragonTooth.to_html(),
                    num_dragon_teeth
                );
            } else if num_dragon_teeth > 0 {
                out!("<li>{}</li>", Goodie::DragonTooth.to_html());
            }
            for l in loot {
                if *l != Goodie::DragonTooth {
                    out!("<li>{}</li>", l.to_html());
                }
            }
            out!("</ul>");
        }
        out!("</ul></div>");
    }
    let doc = web_sys::window().unwrap().document().unwrap();
    doc.get_element_by_id("goodies")
        .unwrap()
        .set_inner_html(&goodies_out);
    doc.get_element_by_id("map-sel")
        .unwrap()
        .set_inner_html(&layouts_full);

    return out;
}

#[wasm_bindgen]
pub fn render_map(
    settings: GameSettings,
    lvl: i32,
    layout: u32,
    canvas: CanvasRenderingContext2d,
    tile_img: HtmlImageElement,
    tile_sz: usize,
) -> String {
    // TODO: currently the map rendering does not depend on luck, so we can just use a dummy value
    // for it. might need to track it properly later tho
    let mut floor = DungeonFloorState::new(settings, lvl, layout, 0., 0.);
    floor.load_map();
    let mut has_buttons = false;
    let tiles = floor.get_tiles();
    for y in 0..64 {
        for x in 0..64 {
            let tile = tiles[(x, y)];
            if let MapTile::SwitchLocation = tile {
                has_buttons = true;
            }
            let tile_off = tile_sz * tile as u8 as usize;
            let tile_sz = tile_sz as f64;
            canvas
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &tile_img,
                    tile_off as f64,
                    0.0,
                    tile_sz,
                    tile_sz,
                    x as f64 * tile_sz,
                    y as f64 * tile_sz,
                    tile_sz,
                    tile_sz,
                )
                .unwrap();
        }
    }
    let mut out = String::new();
    if is_mushroom_floor(layout) {
        out += "Mushroom floor: there's lots of Magma Caps and False Magma Caps here.<br>";
    }
    if is_monster_floor(layout) {
        out += "Monster floor: there's lots of enemies and a guaranteed dwarf gate around the exit here.<br>";
    }
    if lvl != 9 {
        if has_buttons && !is_monster_floor(layout) {
            out += "This floor has a 20% chance of generating a dwarf gate around the exit.<br>";
        }
        if has_buttons {
            out += "When a dwarf gate generates, it'll randomly choose ";
            if is_monster_floor(layout) {
                out += "3";
            } else {
                out += "1 to 3";
            }
            out += " of the possible button positions and generate buttons there.<br>";
        }
    }
    out
}
