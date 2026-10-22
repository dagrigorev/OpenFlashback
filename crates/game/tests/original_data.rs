//! Tests against the user's own Flashback data. Skipped unless `OPENFLASHBACK_DATA` names the
//! directory with the original files (and `fb.exe`).

use of_engine::DataDir;
use of_game::conrad::{anims, FLOOR_STEP, FLOOR_TOP};
use of_game::{Game, Input};

fn data() -> Option<DataDir> {
    let dir = std::env::var("OPENFLASHBACK_DATA").ok()?;
    Some(DataDir::open(dir).expect("OPENFLASHBACK_DATA has no Flashback data"))
}

fn run(game: &mut Game, input: Input, ticks: usize) {
    for _ in 0..ticks {
        game.tick(input);
    }
}

const NONE: Input = Input {
    left: false,
    right: false,
    up: false,
    down: false,
    run: false,
};
const RIGHT: Input = Input { right: true, ..NONE };

#[test]
fn every_level_starts_with_conrad_on_the_ground() {
    let Some(data) = data() else { return };
    for level in 0..5 {
        let mut g = Game::new(&data, level).unwrap();
        run(&mut g, NONE, 90);
        let c = &g.conrad;
        assert!(!c.is_falling(), "level {} start: falling", level + 1);
        assert!(c.floor().is_some(), "level {} start: feet at y {}", level + 1, c.y);
        assert_eq!(c.anim(), anims::STAND, "level {} start", level + 1);
    }
}

#[test]
fn level1_wake_up_turn_and_walk_steps_one_cell_at_a_time() {
    let Some(data) = data() else { return };
    let mut g = Game::new(&data, 0).unwrap();
    assert_eq!((g.room(), g.conrad.x, g.conrad.y), (27, 32, FLOOR_TOP));
    run(&mut g, NONE, 40);
    let x0 = g.conrad.x;
    run(&mut g, RIGHT, 60);
    run(&mut g, NONE, 20);
    assert!(!g.conrad.facing_left);
    assert_eq!((g.conrad.x - x0) % 16, 0, "walked {} px", g.conrad.x - x0);
    assert!(g.conrad.x > x0);
}

#[test]
fn level1_gap_is_fallen_through_or_jumped_over() {
    let Some(data) = data() else { return };
    // Walking into the gap at column 6 drops Conrad to the middle floor.
    let mut g = Game::new(&data, 0).unwrap();
    run(&mut g, NONE, 30);
    run(&mut g, RIGHT, 70);
    run(&mut g, NONE, 40);
    assert_eq!(g.conrad.y, FLOOR_TOP + FLOOR_STEP);

    // A running jump from the edge clears it.
    let mut g = Game::new(&data, 0).unwrap();
    run(&mut g, NONE, 30);
    run(&mut g, RIGHT, 10);
    run(&mut g, Input { run: true, ..RIGHT }, 18);
    run(
        &mut g,
        Input {
            run: true,
            up: true,
            ..RIGHT
        },
        1,
    );
    run(&mut g, NONE, 60);
    assert_eq!(g.conrad.y, FLOOR_TOP);
    assert!(g.conrad.x >= 128, "landed at x {}", g.conrad.x);
}

#[test]
fn level1_climbs_up_to_a_ledge_and_back_down() {
    let Some(data) = data() else { return };
    let mut g = Game::new(&data, 0).unwrap();
    run(&mut g, NONE, 30);
    run(&mut g, RIGHT, 100);
    run(&mut g, NONE, 20);
    run(&mut g, RIGHT, 12);
    run(&mut g, NONE, 10);
    assert_eq!(g.conrad.y, FLOOR_TOP + FLOOR_STEP);
    run(&mut g, Input { up: true, ..NONE }, 1);
    run(&mut g, NONE, 80);
    assert_eq!(g.conrad.y, FLOOR_TOP, "pulled up onto the top floor");
}

/// Every animation chain Conrad's controller uses must exist in the original script, with the
/// controls it reacts to.
#[test]
fn controller_moves_follow_conrads_script() {
    use of_engine::obj::{input_condition, ObjectScripts};
    use of_engine::pge::parse_pge;
    let Some(data) = data() else { return };
    let scripts = ObjectScripts::parse(&data.read("level1.obj").unwrap());
    let node = parse_pge(&data.read("level1.pge").unwrap())[0].obj_node as usize;
    // (from, to, input opcode or 0 for any).
    let chains: [(u16, u16, u8); 14] = [
        (anims::STAND, anims::TURN, 2),
        (anims::TURN, anims::TURN_END, 0),
        (anims::TURN_END, anims::STAND, 0),
        (anims::STAND, anims::JUMP_UP, 1),
        (anims::JUMP_UP, anims::PULL_UP, 1),
        (anims::JUMP_UP, anims::JUMP_UP_LAND, 0),
        (anims::STAND, anims::RUN_START, 8),
        (anims::RUN_START, anims::RUN[3], 0),
        (anims::WALK[0], anims::WALK[1], 4),
        (anims::WALK[1], anims::WALK[0], 4),
        (anims::STAND, anims::CROUCH_DOWN, 3),
        (anims::CROUCH_DOWN, anims::CROUCHED, 0),
        (anims::STAND, anims::CLIMB_DOWN, 7),
        (anims::CLIMB_DOWN, anims::CLIMB_DOWN_LAND, 0),
    ];
    for (from, to, input) in chains {
        let found = scripts
            .from_state(node, from)
            .iter()
            .any(|t| t.next_state == to && (input == 0 || (t.cond1 == input && input_condition(input).is_some())));
        assert!(found, "script has no {from} -> {to} on input {input}");
    }
    // Pulling up onto a ledge moves exactly one floor up.
    let pull = scripts.from_state(node, anims::JUMP_UP);
    assert!(pull
        .iter()
        .any(|t| t.next_state == anims::PULL_UP && t.dy as i32 == -FLOOR_STEP));
}
