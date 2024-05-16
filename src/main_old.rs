use std::{
    collections::{HashMap, HashSet}, hash::Hasher, ops::{Index, IndexMut}, sync::OnceLock
};

struct DotnetRng {
    state: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotnetRng {
    fn new(seed: i32) -> Self {
        // Reference:
        // https://github.com/dotnet/runtime/blob/a45853c4751b5f532cdb38e3db5d4324b5ca878a/src/libraries/System.Private.CoreLib/src/System/Random.Net5CompatImpl.cs#L258
        let mut state = [0_i32; 56];
        let mut mj = 161803398 - seed.saturating_abs();
        state[55] = mj;
        let mut mk = 1_i32;
        let mut ii = 0;
        for _i in 1..55 {
            // this should be a 31 instead lmao
            ii = (ii + 21) % 55;
            state[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += i32::MAX;
            }
            mj = state[ii];
        }
        for _k in 1..5 {
            for i in 1..56 {
                let n = (i + 30) % 55;
                state[i] = state[i].wrapping_sub(state[1 + n]);
                if state[i] < 0 {
                    state[i] += i32::MAX;
                };
            }
        }
        DotnetRng {
            state,
            inextp: 21,
            inext: 0,
        }
    }
    fn next(&mut self) -> i32 {
        self.inext = (self.inext % 55) + 1;
        self.inextp = (self.inextp % 55) + 1;
        let mut result = self.state[self.inext].wrapping_sub(self.state[self.inextp]);
        // the dotnet random api is Very High Quality
        // then again, which part of this implementation isn't?
        if result == i32::MAX {
            result -= 1;
        }
        if result < 0 {
            result += i32::MAX;
        }
        // also extremely funny how they write this, instead of the unmangled result,
        // back into the state array
        self.state[self.inext] = result;
        result
    }
    fn next_f64(&mut self) -> f64 {
        self.next() as f64 * (1.0 / i32::MAX as f64)
    }
    fn next_range(&mut self, max: i32) -> i32 {
        (self.next_f64() * max as f64) as i32
    }
}

fn stardew_hashcode(data: &[u8]) -> i32 {
    let mut hasher = twox_hash::XxHash32::with_seed(0);
    hasher.write(data);
    // XxHash32's finish() always returns a valid u32. stardew casts this to i32 assuming 2's
    // complement. rust's `as` also assumes 2's complement.
    (hasher.finish() as u32) as i32
}

fn stardew_seed_mix_legacy(values: &[f64]) -> i32 {
    debug_assert!(values.len() <= 5);
    values.iter().map(|x| x % 2147483647.0).sum::<f64>() as i32
}
fn stardew_seed_mix_new(values: &[f64]) -> i32 {
    debug_assert!(values.len() <= 5);
    let vals: Vec<i32> = values.iter().map(|x| (x % 2147483647.0) as i32).collect();
    let mut h = [0_i32; 5];
    h[0..vals.len()].copy_from_slice(&vals);
    let bytes: Vec<u8> = h.iter().flat_map(|x| x.to_le_bytes()).collect();
    stardew_hashcode(&bytes)
}
fn stardew_seed_mix(legacy_rng: bool, values: &[f64]) -> i32 {
    if legacy_rng {
        stardew_seed_mix_legacy(values)
    } else {
        stardew_seed_mix_new(values)
    }
}

// this is part of stdlib in nightly
fn f64_next_up(x: f64) -> f64 {
    // this version only works for finite positive floats
    f64::from_bits(x.to_bits() + 1)
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum MapTile {
    Floor,
    Lava,
    Wall,
    Enter,
    Exit,
    SetPiece,
    SwitchLocation,
    MonsterSpawn,
    SpikerSpawn,
}

#[derive(Copy, Clone)]
struct GameSettings {
    seed: i32,
    legacy_rng: bool,
    has_caldera: bool,
    cracked_golden_coconut: bool,
    days_played: u32,
    max_luck_lvl: u32,
}

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

struct DungeonFloorState {
    rng: DotnetRng,
    map: Tilemap,
    set_pieces: Vec<(i32, i32, i32)>,
    gate_locations: HashMap<i32, Vec<(i32, i32)>>,
    switch_locations: HashMap<i32, Vec<(i32, i32)>>,
    start_pos: Option<(i32, i32)>,
    end_pos: Option<(i32, i32)>,
    settings: GameSettings,
    level: i32,
    layout_id: u32,
}
struct ComputedMap {
    // put all the interesting shit here?
    chests: Vec<String>,
}

impl DungeonFloorState {
    fn new(settings: GameSettings, level: i32, layout_id: u32) -> Self {
        let gen_seed = stardew_seed_mix(
            settings.legacy_rng,
            &[
                (settings.days_played * (level + 1) as u32) as f64,
                (level * 5152) as f64,
                (settings.seed / 2) as f64,
            ],
        );
        let mut gen_random =
            DotnetRng::new(stardew_seed_mix(settings.legacy_rng, &[gen_seed as f64]));
        gen_random.next();
        Self {
            rng: gen_random,
            map: Tilemap([[MapTile::Floor; 64]; 64]),
            set_pieces: vec![],
            gate_locations: HashMap::new(),
            switch_locations: HashMap::new(),
            start_pos: None,
            end_pos: None,
            level,
            layout_id,
            settings,
        }
    }

    fn is_monster_level(&self) -> bool {
        (35..=37).contains(&self.layout_id)
    }

    fn is_mushroom_level(&self) -> bool {
        (32..=34).contains(&self.layout_id)
    }

    fn load_map(&mut self) -> ComputedMap {
        self.load_map_tiles();
        self.load_set_pieces();
        self.sync_rng_for_walls();
        // TODO: technically probably need to do some part of CreateEntrance or CreateExit for dirt
        // tiles to have the correct buildings layer info
        self.generate_dirt();
        self.create_dwarf_gates();
        ComputedMap { chests: vec![] }
    }

    fn load_map_tiles(&mut self) {
        let mut flip_x = self.rng.next_range(2) == 1;
        if self.layout_id == 0 || self.layout_id == 31 {
            flip_x = false;
        }

        // floor tile type generation (we don't care about the result, but need RNG to sync)
        for _x in 0..64 {
            for _y in 0..64 {
                if self.rng.next_f64() < 0.30000001192092896 {
                    self.rng.next();
                    self.rng.next();
                }
            }
        }
        // load layout images
        // TODO: don't bundle a whole png decoder for this
        static LAYOUTS: OnceLock<image::DynamicImage> = OnceLock::new();
        let img = LAYOUTS.get_or_init(|| image::open("Layouts.png").unwrap());

        // extract this floor's layout
        let total_cols = img.width() / 64;
        let offset_x = self.layout_id % total_cols;
        let offset_y = self.layout_id / total_cols;
        let mut this_one = img.crop_imm(offset_x * 64, offset_y * 64, 64, 64);
        if flip_x {
            this_one = this_one.fliph();
        }

        for (x, y, &color) in this_one.to_rgb8().enumerate_pixels() {
            self.map[(x as i32, y as i32)] = match color.0 {
                [255, 0, 0] => {
                    if self.end_pos.is_some() {
                        println!("warning: buggy map? multiple exits");
                    }
                    self.end_pos = Some((x as i32, y as i32));
                    MapTile::Exit
                }
                [0, 255, 0] => {
                    if self.end_pos.is_some() {
                        println!("warning: buggy map? multiple enters");
                    }
                    self.start_pos = Some((x as i32, y as i32));
                    MapTile::Enter
                }
                [0, 0, 255] => MapTile::Lava,
                [255, 255, 0] => MapTile::SetPiece,
                [128, 128, 128] => MapTile::SwitchLocation,
                [0, 255, 255] => MapTile::MonsterSpawn,
                [0, 0, 0] => MapTile::Wall,
                [255, 255, 255] => MapTile::Floor,
                _ => panic!("unexpected color in Layouts.png! {:?}", color.0),
            }
        }
        if self.end_pos.is_none() {
            panic!("buggy map: no exit");
        }
        if self.start_pos.is_none() {
            panic!("buggy map: no entrance");
        }
    }

    fn load_set_pieces(&mut self) {
        let mut possible_switch_locations: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
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
                            "warning: buggy map? layout={} at x={} y={}",
                            self.layout_id, x, y
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
                if self.map[(x, y)] == MapTile::SwitchLocation {
                    possible_switch_locations.entry(0).or_default().push((x, y));
                }
            }
        }

        for (x, y, set_size) in self.set_pieces.iter().cloned() {
            static LAYOUTS: OnceLock<HashMap<i32, tiled::Map>> = OnceLock::new();
            let all_layouts = LAYOUTS.get_or_init(|| {
                HashMap::from_iter([3, 4, 8, 16, 32].into_iter().map(|x| {
                    (
                        x,
                        tiled::Loader::new()
                            .load_tmx_map(format!("Volcano_SetPieces_{}.tmx", x))
                            .unwrap(),
                    )
                }))
            });
            let paths_layer = all_layouts[&set_size]
                .layers()
                .find(|x| x.name == "Paths" && matches!(x.layer_type(), tiled::LayerType::Tiles(_)))
                .unwrap()
                .as_tile_layer()
                .unwrap();
            let num_cols = paths_layer.width().unwrap() as i32 / set_size;
            let num_rows = paths_layer.height().unwrap() as i32 / set_size;
            let selected_col = self.rng.next_range(num_cols);
            let selected_row = self.rng.next_range(num_rows);
            if buggy {
                println!(
                    "layout {}: x={} y={} sz={} selected row {}, col {}",
                    self.layout_id, x, y, set_size, selected_row, selected_col
                );
            }
            for setx in 0..set_size {
                // this really shouldn't be ..=, but that's what the game does
                for sety in 0..=set_size {
                    let src_x = selected_col * set_size + setx;
                    let src_y = selected_row * set_size + sety;
                    let dst_x = x + setx;
                    let dst_y = y + sety;
                    let tile = paths_layer.get_tile(src_x as i32, src_y as i32);
                    if let Some(tile) = tile {
                        if sety == set_size {
                            //println!("layout {}: buggy paths at {},{}, tile {}", layout, dst_x, dst_y, tile.id());
                        }
                        match tile.id() {
                            234..=239 => {
                                // possible gate location, random
                                let chance = if let Some(h) = tile.get_tile() {
                                    let chance_prop = h.properties.get("Chance");
                                    if let Some(tiled::PropertyValue::FloatValue(f)) = chance_prop {
                                        *f
                                    } else {
                                        1.0
                                    }
                                } else {
                                    1.0
                                };
                                if self.rng.next_f64() < chance as f64 {
                                    self.gate_locations.entry((tile.id() - 234) as i32).or_default().push((dst_x, dst_y));
                                }
                            }
                            // possible switch location
                            250..=255 => {
                                possible_switch_locations
                                    .entry((tile.id() - 250) as i32)
                                    .or_default()
                                    .push((dst_x, dst_y));
                            }
                            330 => self.map[(dst_x, dst_y)] = MapTile::MonsterSpawn,
                            331 => self.map[(dst_x, dst_y)] = MapTile::Lava,
                            332 => {
                                println!("chest at {},{} on level {}", dst_x, dst_y, self.level);
                                let chest_seed = stardew_seed_mix(
                                    self.settings.legacy_rng,
                                    &[self.rng.next() as f64],
                                );
                                let mut chest_rng = DotnetRng::new(chest_seed);
                                // roll < (0.1 or 0.5) + luckboost
                                // roll - (0.1 or 0.5) < luckboost
                                let chest_roll =
                                    chest_rng.next_f64() - if self.level == 9 { 0.5 } else { 0.1 };
                                println!("  luckboost needed for rare: {:.3}", chest_roll);
                                chest_content(chest_seed, true);
                                chest_content(chest_seed, false);
                            }
                            // not even used ingame??
                            333 => self.map[(dst_x, dst_y)] = MapTile::Wall,
                            334 => {
                                if self.rng.next_f64() < 0.5 {
                                    // barrel
                                }
                            }
                            335 => {
                                if self.rng.next_f64() < 0.5 {
                                    println!(
                                        "dragon tooth at {},{} on level {}",
                                        dst_x, dst_y, self.level
                                    );
                                }
                            }
                            346 => self.map[(dst_x, dst_y)] = MapTile::SpikerSpawn,
                            _ => {
                                println!("unknown tile on path layer: {}", tile.id());
                            }
                        }
                    }
                }
            }
        }
    }

    fn sync_rng_for_walls(&mut self) {
        // first call:
        // black, 0, 4, 4, 4, start_in_wall, delegate: set(chartreuse), corner hack
        // match = black (MapTile::Wall)
        // source_x = 0
        // source_y = 4
        // wall_height = 4
        // random_wall_variants = 4
        // start in wall
        // insufficient delegate: set(temporary)
        // corner hack: yes
        // first half of function:
        for _pass in 0..2 {
            for x in 0..64 {
                for y in 0..=64 {
                    if y == 64 || self.map[(x, y)] != MapTile::Wall {
                        // random wall variant generation
                        if self.rng.next_f64() < 0.5 {
                            self.rng.next();
                        }
                    }
                }
            }
        }
        // second half:
        for _x in 0..64 {
            for _y in 0..64 {
                self.rng.next();
            }
        }
        // second call (fix up the chartreuse): only randomness is a single call for every tile.
        for _x in 0..64 {
            for _y in 0..64 {
                self.rng.next();
            }
        }
        // wait this is fucking it? lmfao

        // i think this is for lava lights or something
        for x in 0..64 {
            for y in 0..64 {
                if self.map[(x, y)] == MapTile::Lava {
                    self.rng.next();
                }
            }
        }
    }

    fn generate_dirt(&mut self) {
        if self.level == 5 {
            return;
        }
        let mut dirt_tiles = HashSet::new();
        for _j in 0..8 {
            let mut center_x = self.rng.next_range(64);
            let mut center_y = self.rng.next_range(64);
            let travel_distance = 2 + self.rng.next_range(6);
            let mut radius = 1 + self.rng.next_range(2);
            let mut dir_x = if self.rng.next_range(2) != 0 { 1 } else { -1 };
            let mut dir_y = if self.rng.next_range(2) != 0 { 1 } else { -1 };
            let x_oriented = self.rng.next_range(2) == 0;
            for _k in 0..travel_distance {
                for x in center_x - radius..center_x + radius {
                    for y in center_y - radius..center_y + radius {
                        if self.map[(x, y)] == MapTile::Floor {
                            dirt_tiles.insert((x, y));
                        }
                    }
                }
                if x_oriented {
                    dir_y += if self.rng.next_range(2) != 0 { 1 } else { -1 };
                } else {
                    dir_x += if self.rng.next_range(2) != 0 { 1 } else { -1 };
                }
                center_x += dir_x;
                center_y += dir_y;
                radius += if self.rng.next_range(2) != 0 { 1 } else { -1 };
                radius = radius.clamp(1, 4);
            }
        }
        for _i in 0..2 {
            // erode invalid dirt
            // fuck.... it checks against *anything* on the buildings layer lol
            // ok for now actually just ignoring it
            // (i think only dwarf gates can end up there? and those are only added later)
            let mut new_dirt = HashSet::new();
            'nextdirt: for &(x, y) in dirt_tiles.iter() {
                for (setx, sety, setsz) in self.set_pieces.iter().cloned() {
                    if (setx..setx+setsz).contains(&x) && (sety..sety+setsz).contains(&y) {
                        continue 'nextdirt;
                    }
                }
                // should also check that tile x,y is empty on Buildings

                new_dirt.insert((x, y));
            }
        }
        for _p in dirt_tiles.iter() {
            if self.rng.next_f64() < 0.015 {
                // println!("duggy");
            }
        }
    }

    fn create_dwarf_gates(&mut self) {
        let gate_prob = if self.is_monster_level() { 1.0 } else { 0.2 };
        if self.level == 9 || self.rng.next_f64() < gate_prob {
            if self.switch_locations.get(&0).is_some_and(|x| x.len() > 0) {
                self.gate_locations.entry(0).or_default().push(self.end_pos.unwrap());
            }
        }
        for (ind, possible) in self.gate_locations.iter() {
            if possible.len() > 0 && self.switch_locations.get(ind).is_some_and(|x| x.len() > 0) {
                let _pt = possible[self.rng.next_range(possible.len() as i32) as usize];
                // dwarf gate at pt
                let _gate_seed = self.rng.next();
            }
        }
    }

    fn print_map(&self) {
        for row in self.map.0 {
            for tile in row {
                let c = match tile {
                    MapTile::Floor => ' ',
                    MapTile::Lava => '~',
                    MapTile::Wall => '#',
                    MapTile::Enter => 'I',
                    MapTile::Exit => 'O',
                    MapTile::SetPiece => 'X',
                    MapTile::SwitchLocation => '?',
                    MapTile::MonsterSpawn => 'M',
                    MapTile::SpikerSpawn => 'M',
                };
                print!("{}", c);
            }
            println!();
        }
        println!();
    }
}

fn chest_content(seed: i32, rare: bool) {
    // TODO: golden coconut check
    let mut rng = DotnetRng::new(seed);
    rng.next(); // one roll used for rare/normal check
    if rare {
        let ind = rng.next_range(9);
        let res = match ind {
            0 => "10 cinder shards",
            1 => "mermaid boots",
            2 => "dragonscale boots",
            3 => "3 golden coconuts",
            4 => "phoenix ring",
            5 => "hot java ring",
            6 => [
                "\x1b[1;31mdragontooth cutlass",
                "\x1b[1;31mdragontooth club",
                "\x1b[1;31mdragontooth shiv",
            ][rng.next_range(3) as usize],
            7 => "deluxe pirate hat",
            8 => "ostrich egg",
            _ => unreachable!(),
        };
        println!("  rare drop: {}\x1b[0m", res);
    } else {
        let ind = rng.next_range(7);
        let res = match ind {
            0 => "3 cinder shards",
            1 => "1 golden coconut",
            2 => "8 taro tuber",
            3 => "5 pineapple seeds",
            4 => "protection ring",
            5 => "soul sapper ring",
            6 => ["dwarf sword", "dwarf hammer", "dwarf dagger"][rng.next_range(3) as usize],
            _ => unreachable!(),
        };
        println!("  common drop: {}", res);
    }
}

fn compute_volcano_layouts(settings: GameSettings) {
    fn compute_inner(settings: GameSettings, prev: &[u32], minluck: f64, maxluck: f64) {
        let level = prev.len();
        let gen_seed = stardew_seed_mix_new(&[
            (settings.days_played * (level + 1) as u32) as f64,
            (level * 5152) as f64,
            (settings.seed / 2) as f64,
        ]);
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
            println!("{:.4}..{:.4} -> {:?}", minluck, maxluck, &lvlbuf);
            for (i, &x) in lvlbuf.iter().enumerate() {
                //do_layout(seed, days, i as i32, x);
                let mut h = DungeonFloorState::new(settings, i as i32, x);
                let out = h.load_map();
                dbg!(out.chests);
            }
            return;
        }
        let mut valid_layouts: Vec<u32> = (1..30).collect();
        let mut layout_random = DotnetRng::new(stardew_seed_mix_new(&[gen_seed as f64]));
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
                    compute_inner(settings, prev, minluck, midpoint);
                    // in there: rng ~= minluck * 0.5, but minluck is slightly increased,
                    // so rng < minluck * 0.5
                    compute_inner(settings, prev, f64_next_up(midpoint), maxluck);
                    // TODO: if we roll a really low layout, these branches might end up choosing
                    // the same layout anyways, making the bifurcation unnecessary. but that's
                    // annoying to check for
                    return;
                }
            }
        }
        if level > 0 && settings.has_caldera {
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
        compute_inner(settings, &lvlbuf, minluck, maxluck);
    }
    // returns sets of layouts based on ranges of luckMult
    // attainable luckMult: 0.95 (or 0.9625 with charm) .. unbounded?
    // realistic max luck level: 5+1+1+2 = 9 (rock candy, luck rings, qi seasoned ginger ale)
    // max luckMult: 1.3775
    compute_inner(
        settings,
        &[],
        0.95,
        1.0625 + 0.035 * (settings.max_luck_lvl as f64),
    );
}

fn main() {
    stardew_seed_mix_legacy(&[0.; 5]);
    let mut total_days: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    loop {
        let total_seasons = (total_days - 1) / 28;
        let year = total_seasons / 4 + 1;
        let season = total_seasons % 4;
        let day = (total_days - 1) % 28 + 1;
        println!(
            "day: {} of {}, year {}",
            day,
            ["spring", "summer", "fall", "winter"][season as usize],
            year
        );
        let settings = GameSettings {
            seed: 361279468,
            legacy_rng: false,
            has_caldera: true,
            cracked_golden_coconut: true,
            days_played: total_days,
            max_luck_lvl: 5,
        };
        compute_volcano_layouts(settings);
        std::io::stdin().read_line(&mut String::new()).unwrap();
        total_days += 1;
    }
}
