use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone)]
pub enum SetPieceFeature {
    Rng,
    Tooth,
    Chest,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum MapTile {
    Floor = 0,
    Lava = 1,
    Wall = 2,
    Enter = 3,
    Exit = 4,
    SetPiece = 5,
    SwitchLocation = 6,
    MonsterSpawn = 7,
}

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut out_sizes = File::create(out_dir.join("set_piece_sizes.rs")).unwrap();
    let mut out_events = File::create(out_dir.join("set_piece_events.rs")).unwrap();
    let mut out_layouts = File::create(out_dir.join("layouts.bin")).unwrap();
    writeln!(out_sizes, "match set_size {{").unwrap();
    writeln!(
        out_events,
        "pub fn get_piece_events() -> HashMap<(i32, i32, i32), &'static [SetPieceFeature]> {{
use SetPieceFeature::*;
["
    )
    .unwrap();

    for set_size in [3, 4, 8, 16, 32] {
        let fname = format!("game_data/Volcano_SetPieces_{}.tmx", set_size);
        let map = &tiled::Loader::new().load_tmx_map(&fname).unwrap();
        println!("cargo::rerun-if-changed={}", fname);
        let paths_layer = map
            .layers()
            .find(|x| x.name == "Paths" && matches!(x.layer_type(), tiled::LayerType::Tiles(_)))
            .unwrap()
            .as_tile_layer()
            .unwrap();
        let num_cols = paths_layer.width().unwrap() as i32 / set_size;
        let num_rows = paths_layer.height().unwrap() as i32 / set_size;
        writeln!(out_sizes, "    {set_size} => ({num_rows}, {num_cols}),").unwrap();
        for selected_col in 0..num_cols {
            for selected_row in 0..num_rows {
                let mut events = vec![];
                for setx in 0..set_size {
                    // this really shouldn't be ..=, but that's what the game does
                    for sety in 0..=set_size {
                        let src_x = selected_col * set_size + setx;
                        let src_y = selected_row * set_size + sety;
                        let tile = paths_layer.get_tile(src_x, src_y);
                        if let Some(tile) = tile {
                            match tile.id() {
                                234..=239 => {
                                    // possible gate location, random
                                    events.push(SetPieceFeature::Rng);
                                }
                                250 => panic!("set piece contained switch for gate #0"),
                                // possible setpiece switch location - not worth to track
                                251..=255 => {}
                                330 => {}
                                331 => {}
                                332 => {
                                    events.push(SetPieceFeature::Chest);
                                }
                                // wall - not even used ingame??
                                333 => {}
                                334 => {
                                    // barrel
                                    events.push(SetPieceFeature::Rng);
                                }
                                335 => {
                                    events.push(SetPieceFeature::Tooth);
                                }
                                // spiker spawn point
                                346 => {}
                                _ => {
                                    panic!("unknown tile on path layer: {}", tile.id());
                                }
                            }
                        }
                    }
                }
                if !events.is_empty() {
                    writeln!(
                        out_events,
                        "    (({}, {}, {}), &{:?} as &[_]),",
                        set_size, selected_row, selected_col, events
                    )
                    .unwrap();
                }
            }
        }
    }
    writeln!(out_events, "].into_iter().collect() }}").unwrap();
    writeln!(out_sizes, "    _ => panic!(\"invalid set size\"),\n}}").unwrap();

    println!("cargo::rerun-if-changed=game_data/Layouts.png");
    let decoder = png::Decoder::new(File::open("game_data/Layouts.png").unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert!(
        info.color_type == png::ColorType::Rgba,
        "Layouts.png has unexpected color type!"
    );
    assert!(
        info.bit_depth == png::BitDepth::Eight,
        "Layouts.png has unexpected bit depth!"
    );
    let layout_rows = info.height / 64;
    let layout_cols = info.width / 64;
    let mut layouts = Vec::new();
    for layout_y in 0..layout_rows {
        for layout_x in 0..layout_cols {
            let this: [[MapTile; 64]; 64] = std::array::from_fn(|y| {
                std::array::from_fn(|x| {
                    let px_x = (layout_x as usize) * 64 + x;
                    let px_y = (layout_y as usize) * 64 + y;
                    let px_offset = info.line_size * px_y + 4 * px_x;
                    let color = &buf[px_offset..px_offset + 3];
                    match color {
                        [255, 0, 0] => MapTile::Exit,
                        [0, 255, 0] => MapTile::Enter,
                        [0, 0, 255] => MapTile::Lava,
                        [255, 255, 0] => MapTile::SetPiece,
                        [128, 128, 128] => MapTile::SwitchLocation,
                        [0, 255, 255] => MapTile::MonsterSpawn,
                        [0, 0, 0] => MapTile::Wall,
                        [255, 255, 255] => MapTile::Floor,
                        _ => panic!("unexpected color in Layouts.png! {:?}", color),
                    }
                })
            });
            layouts.push(this);
        }
    }
    while layouts.last() == Some(&[[MapTile::Wall; 64]; 64]) {
        layouts.pop();
    }
    let bytes: Vec<u8> = layouts
        .iter()
        .flatten()
        .flatten()
        .map(|&x| x as u8)
        .collect();
    out_layouts.write_all(&bytes).unwrap();
}
