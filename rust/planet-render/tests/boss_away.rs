//! `boss_away`: while the widget shows a saga boss out on the desktop, the planet is drawn without his sprite; his bar
//! and name stay.

use planet_core::data::VILLAINS;
use planet_core::{Game, DT};
use planet_render::scene::{Scene, View};
use planet_render::Framebuffer;

#[test]
fn an_away_saga_boss_leaves_only_his_sprite_off_the_planet() {
    let mut g = Game::new(7);
    g.start();
    // to the first saga raid's fight, past the scouter's intro
    let mut n = 0;
    while !g.tl.boss.as_ref().is_some_and(|b| b.saga && b.intro.is_none() && g.t >= b.go + 2.0) {
        g.frame(g.t + DT, DT);
        n += 1;
        assert!(n < 30 * 900, "no saga raid in 15 minutes");
    }
    let name = g.tl.boss.as_ref().unwrap().name.clone();
    let spr = planet_core::sprite::get(&format!("VILLAINS.{}.spr", VILLAINS.iter().position(|v| v.0 == name).unwrap()));
    let (mut fb, mut sc, view) = (Framebuffer::new(200, 134), Scene::new(), View { scale: 1.0, bare: true, clouds: None, cloud_amt: 2.0 });
    let mut draw = |g: &mut Game, away: bool| {
        g.boss_away = away;
        fb.buf.fill(0); // floating, nothing is drawn under the scene
        sc.draw(&mut fb, g, &view, 0.0);
        fb.buf.clone()
    };
    let (here, away, back) = (draw(&mut g, false), draw(&mut g, true), draw(&mut g, false));
    // his box, where he stands: no time passes between the draws, so he doesn't move
    let b = g.tl.boss.clone().unwrap();
    let (x0, x1) = ((b.x - spr.ax * b.k) as usize, (b.x + (spr.w as f64 - spr.ax) * b.k) as usize);
    let (y0, y1) = ((b.y - spr.h as f64 * b.k).max(0.0) as usize, b.y as usize);
    let boxed = |a: &[u32], c: &[u32]| (y0..y1).flat_map(|y| (x0..x1).map(move |x| y * 200 + x)).filter(|&i| a[i] != c[i]).count();
    let (gone, moved) = (boxed(&here, &away), boxed(&here, &back));
    assert!(gone > 200 && gone > 10 * moved, "his sprite is gone away: {gone} pixels (and {moved} between two draws of him)");
    // and nothing else (his outline aside): his bar, his name and the rest of the scene are drawn as they were
    let inside = |i: usize| (x0.saturating_sub(2)..x1 + 2).contains(&(i % 200)) && (y0.saturating_sub(2)..y1 + 2).contains(&(i / 200));
    assert_eq!((0..here.len()).filter(|&i| !inside(i) && here[i] != away[i]).count(), 0);
}
