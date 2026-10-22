//! Cutscene tests against the user's own Flashback data. Skipped unless `OPENFLASHBACK_DATA`
//! names the directory with the original files.

use of_engine::cutscene::{Cmd, Op, Player, Pol, CUTSCENE_BASE};
use of_engine::DataDir;

fn data() -> Option<DataDir> {
    let dir = std::env::var("OPENFLASHBACK_DATA").ok()?;
    Some(DataDir::open(dir).expect("OPENFLASHBACK_DATA has no Flashback data"))
}

#[test]
fn every_script_plays_to_its_end() {
    let Some(data) = data() else { return };
    let mut scripts = 0;
    for entry in std::fs::read_dir(data.root()).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let Some(stem) = name.strip_suffix(".cmd") else {
            continue;
        };
        let cmd = Cmd::parse(std::fs::read(entry.path()).unwrap());
        let pol = Pol::parse(data.read(&format!("{stem}.pol")).unwrap()).unwrap();
        for sub in 0..cmd.len() {
            let ops = cmd.ops(sub);
            assert!(!ops.is_empty(), "{stem}.cmd sub-script {sub} is empty");
            for op in &ops {
                if let Op::Shape { shape, .. } = op {
                    assert!((*shape as usize) < pol.shape_count(), "{stem}: no shape {shape}");
                }
            }
            let mut player = Player::new(pol.clone(), ops);
            let mut frames = 0;
            while player.next_frame().is_some() {
                frames += 1;
                assert!(frames < 100_000, "{stem}.cmd sub-script {sub} does not end");
            }
            scripts += 1;
        }
    }
    assert!(scripts >= 60, "only {scripts} cutscene scripts found");
}

#[test]
fn logos_show_the_publishers_names() {
    let Some(data) = data() else { return };
    let pol = Pol::parse(data.read("logos.pol").unwrap()).unwrap();
    let cmd = Cmd::parse(data.read("logos.cmd").unwrap());
    let mut player = Player::new(pol, cmd.ops(0));
    player.next_frame().unwrap();
    let drawn = player.screen.pixels.iter().filter(|&&c| c > CUTSCENE_BASE).count();
    assert!(drawn > 1000, "first logo frame has only {drawn} drawn pixels");
    let lit = player.full_palette().0[CUTSCENE_BASE as usize + 1..CUTSCENE_BASE as usize + 16]
        .iter()
        .any(|c| c.0 > 100 || c.1 > 100 || c.2 > 100);
    assert!(lit, "first logo frame palette is dark");
}

#[test]
fn cutscene_texts_and_captions_are_read() {
    use of_engine::cutscene::CAPTION_COLOURS;
    use of_engine::text::load_cutscene_text;
    let Some(data) = data() else { return };
    let (font, texts) = load_cutscene_text(&data).unwrap();
    assert_eq!(texts.len(), 117);
    assert_eq!(texts.get(50).unwrap(), &["THE KIDNAPPING!".to_string()]);
    assert!(texts.get(1).unwrap().len() > 1, "bars break texts into lines");

    // The memory cutscene shows its first caption under the window.
    let pol = Pol::parse(data.read("memo.pol").unwrap()).unwrap();
    let cmd = Cmd::parse(data.read("memo.cmd").unwrap());
    let mut player = Player::new(pol, cmd.ops(0)).with_text(font, texts);
    let below = |p: &Player| {
        p.screen.pixels[180 * 256..]
            .iter()
            .filter(|&&c| c == CAPTION_COLOURS[2])
            .count()
    };
    let mut seen = false;
    while player.next_frame().is_some() {
        if below(&player) > 50 {
            seen = true;
            break;
        }
    }
    assert!(seen, "no caption under the window");
}
