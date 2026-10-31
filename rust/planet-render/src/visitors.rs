//! Everything game.js draws after the hero and before the particles: friends and visitors on the planet, each
//! world's residents, the boss and the ginyu force, ki blasts, in game.js's order. Screen spots are written back.

use planet_core::data::Upg;
use planet_core::geom::{add, ap, cross, mul, norm, V3};
use planet_core::jsmath::{atan2, cos, hypot, sin};
use planet_core::mech::Ally;
use planet_core::num::js_round;
use planet_core::sprite::{self, Sprite};
use planet_core::Game;

use crate::scene::{blit, hash, text, text_width, tone, Ctx, Scene};
use crate::{h2, pk};
use planet_core::fx::Particle;

/// What the hero's draw leaves for the rest: where he is and whether he can be seen.
pub struct HeroAt {
    pub x: f64,
    pub y: f64,
    pub ang: f64,
    pub flip: bool,
    pub lift: f64,
    pub depth: f64,
    pub seen: bool,
}

/// `standAng`: npcs stand up on the face of the planet, tipping out only near the rim.
fn stand_ang(v: V3) -> f64 {
    atan2(v[0], v[1]) * 1f64.min((v[0] * v[0] + v[1] * v[1]) * 1.6)
}

const POLE: [u32; 4] = [0x7a4a2a, 0xa86a3a, 0xcc8e52, 0xe8b880];
const POLE_RING: [u32; 4] = [0x8a1a12, 0xb82a1c, 0xe0503a, 0xf88a6a];

impl Scene {
    /// a sprite stood on a point of the planet (`npc`)
    fn npc(c: &mut Ctx, spr: &Sprite, p: V3, flip: bool, k: f64) {
        let v = ap(&c.m, p);
        if v[2] > 0.05 {
            let (x, y) = (c.cx + v[0] * c.r, c.cy - v[1] * c.r + c.bob);
            c.blit_z(spr, x, y, stand_ang(v), flip, 0.0, v[2] * c.r + 6.0, k);
        }
    }

    /// `decal`: a sprite lying flat on the surface, foreshortened with the planet's curve.
    fn decal(c: &mut Ctx, spr: &Sprite, v: V3, k: f64) {
        let mut t1 = cross([0.0, 1.0, 0.0], v);
        let l1 = hypot(&t1);
        t1 = if l1 < 0.05 { [1.0, 0.0, 0.0] } else { mul(t1, 1.0 / l1) };
        let t2 = cross(t1, v);
        let (ax, ay, bx, by) = (t1[0] * k, -t1[1] * k, t2[0] * k, -t2[1] * k);
        let det = ax * by - bx * ay;
        if det.abs() < 0.05 {
            return;
        }
        let (ox, oy, z0) = (c.cx + v[0] * c.r, c.cy - v[1] * c.r + c.bob, v[2] * c.r);
        let rr = hypot(&[spr.w as f64, spr.h as f64]) * k / 2.0 + 1.0;
        let mut y = (oy - rr).floor();
        while y <= oy + rr {
            if y >= 0.0 && y < c.h {
                let mut x = (ox - rr).floor();
                while x <= ox + rr {
                    if x >= 0.0 && x < c.w {
                        let (dx, dy) = (x + 0.5 - ox, y + 0.5 - oy);
                        let u = (dx * by - dy * bx) / det;
                        let w = (ax * dy - ay * dx) / det;
                        let i = (u + spr.w as f64 / 2.0).floor();
                        let j = (w + spr.h as f64 / 2.0).floor();
                        if i >= 0.0 && j >= 0.0 && i < spr.w as f64 && j < spr.h as f64 {
                            let col = spr.col[j as usize * spr.w + i as usize];
                            if col >= 0 {
                                let kk = y as usize * c.fb.w + x as usize;
                                let z = z0 + (t1[2] * u + t2[2] * w) * k;
                                if !(c.fb.zb[kk] as f64 > z + 3.0) {
                                    c.fb.buf[kk] = pk(col as u32, 255);
                                }
                            }
                        }
                    }
                    x += 1.0;
                }
            }
            y += 1.0;
        }
    }

    pub(crate) fn draw_visitors(&mut self, c: &mut Ctx, g: &mut Game, hero: &HeroAt, dt: f64) {
        let t = c.t;
        let (cx, cy, r, bob) = (c.cx, c.cy, c.r, c.bob);
        let m = c.m;
        let world = c.world;
        // the fusion dance's beat ring
        if let Some(d) = g.tl.dance.clone() {
            if d.done == 0.0 && hero.seen && d.i < 3 {
                let b = d.beats[d.i];
                let rr = 6.0 + 0f64.max(b - t) * 26.0;
                let (hx, hy) = (hero.x + sin(hero.ang) * 14.0, hero.y - cos(hero.ang) * 14.0);
                let mut a = 0.0;
                while a < 6.28 {
                    c.fb.px(hx + cos(a) * rr, hy + sin(a) * rr, if (b - t).abs() < 0.32 { 0xfff27a } else { 0xffffff }, 1.0);
                    a += 0.12;
                }
                text(c.fb, ["fu", "sion", "ha!"][d.i], hx, hy - rr - 8.0, if t - d.ok < 0.3 { 0xfff27a } else { 0xffffff }, 1.0);
            }
        }
        if let Some(boom) = g.tl.cling {
            c.blit_z(
                sprite::get("SAIBA"),
                hero.x - cos(hero.ang) * 5.0,
                hero.y - sin(hero.ang) * 5.0,
                hero.ang,
                !hero.flip,
                hero.lift + 6.0 + if boom - t < 1.0 && (t * 16.0).floor() as i64 % 2 != 0 { 1.0 } else { 0.0 },
                hero.depth + 1.0,
                0.9,
            );
        }
        // piccolo meditates above the planet
        if g.s.lv(Upg::Piccolo) > 0 {
            let v = ap(&m, mul(g.frame3.cap_pt(0.6, 0.9), 1.28));
            g.tl.piccolo_at = if v[2] > -0.2 { Some((cx + v[0] * r, cy - v[1] * r + bob + sin(t * 1.6) * 2.0)) } else { None };
            if let Some((x, y)) = g.tl.piccolo_at.filter(|_| !crate::mech::held(g, Ally::Piccolo)) {
                c.blit_z(sprite::get("PICCOLO"), x, y, 0.0, false, 0.0, v[2] * r + 8.0, 0.95);
            }
        } else {
            g.tl.piccolo_at = None;
        }
        if let Some(car) = g.tl.car.as_mut() {
            let v = ap(&m, g.frame3.road_pt(car.phi));
            let ahead = ap(&m, g.frame3.road_pt(car.phi + 0.05));
            let ax = cx + ahead[0] * r;
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                car.sx = Some((sx, sy));
                c.blit_z(sprite::get("KAICAR"), sx, sy, atan2(v[0], v[1]), ax > sx, 0.0, v[2] * r + 6.0, 0.9);
            } else {
                car.sx = None;
            }
        }
        if let Some(kai) = g.tl.kai.as_mut() {
            let v = ap(&m, kai.c);
            let up = 1f64.min((t - kai.t0) / 0.4) * 1f64.min((kai.until - t) / 0.4);
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                kai.sx = Some((sx, sy));
                c.blit_z(sprite::get("KINGKAI"), sx, sy, atan2(v[0], v[1]), false, (up - 1.0) * 18.0, v[2] * r + 6.0, 0.85);
            } else {
                kai.sx = None;
            }
        }
        if let Some(mut d) = g.tl.date.clone() {
            let v = ap(&m, d.c);
            let v2 = ap(&m, d.c2);
            let up = 1f64.min((t - d.t0) / 0.4) * 1f64.min((d.until - t) / 0.4);
            if v[2] > 0.05 {
                c.blit_z(sprite::get("A18"), cx + v[0] * r, cy - v[1] * r + bob, atan2(v[0], v[1]), true, (up - 1.0) * 18.0, v[2] * r + 6.0, 1.0);
            }
            if d.kr && v2[2] > 0.05 {
                let (sx, sy) = (cx + v2[0] * r, cy - v2[1] * r + bob);
                d.sx2 = Some((sx, sy));
                c.blit_z(sprite::get("KRILLIN"), sx, sy, atan2(v2[0], v2[1]), false, (up - 1.0) * 18.0 + sin(t * 9.0).abs() * 1.5, v2[2] * r + 6.0, 1.0);
                if !d.paid && g.rng.random() < 0.25 {
                    let x = sx + (g.rng.random() - 0.5) * 8.0;
                    g.fx.push(Particle::new(x, sy - 20.0, &[0xffd0e0, 0xff7aa8, 0xe0306a]).v(0.0, -24.0).life(0.7));
                }
            } else {
                d.sx2 = None;
            }
            g.tl.date = Some(d);
        }
        if let Some(mut a) = g.tl.a16.clone() {
            let v = ap(&m, a.c);
            let up = 1f64.min((t - a.t0) / 0.4) * 1f64.min((a.until - t) / 0.4);
            if v[2] > 0.05 {
                let (x, y, ang) = (cx + v[0] * r, cy - v[1] * r + bob, atan2(v[0], v[1]));
                c.blit_z(sprite::get("A16"), x, y, ang, false, (up - 1.0) * 18.0, v[2] * r + 6.0, 0.95);
                for b in a.birds.iter_mut() {
                    let fly = if b.free != 0.0 { (t - b.free) * 60.0 } else { 0.0 };
                    let bx = x + (b.i as f64 - 1.0) * 9.0 + fly * 0.6;
                    let by = y - 24.0 - (b.i % 2) as f64 * 6.0 - fly;
                    b.sx = if b.free != 0.0 { None } else { Some((bx, by)) };
                    if by > -10.0 {
                        let spr = sprite::get(["BIRD_A", "BIRD_B", "BIRD_C", "BIRD_D"][b.kind as usize]);
                        blit(c.fb, spr, bx - 3.0, by - 3.0 + if b.free != 0.0 { 0.0 } else { sin(t * 3.0 + b.i as f64).abs() * 1.0 }, b.i % 2 == 1);
                    }
                }
            } else {
                for b in a.birds.iter_mut() {
                    b.sx = None;
                }
            }
            g.tl.a16 = Some(a);
        }
        for gd in g.tl.gadgets.iter_mut() {
            let v = ap(&m, gd.c);
            let up = 1f64.min((t - gd.t0) / 0.3);
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                gd.sx = Some((sx, sy));
                let spr = sprite::get(["GADGET_A", "GADGET_B", "GADGET_C", "GADGET_D"][gd.kind as usize]);
                c.blit_z(spr, sx, sy, atan2(v[0], v[1]), false, (1.0 - up) * 30.0 + sin(t * 3.0 + gd.t0).abs() * 2.0, v[2] * r + 6.0, 1.0);
            } else {
                gd.sx = None;
            }
        }
        if world == 6 {
            if let Some((house, roshi)) = self.built.kh {
                let v = ap(&m, house);
                if v[2] > 0.05 {
                    c.blit_z(sprite::get("KAMEHOUSE"), cx + v[0] * r, cy - v[1] * r + bob, atan2(v[0], v[1]), false, 0.0, v[2] * r + 6.0, 1.0);
                }
                Self::npc(c, sprite::get("ROSHI"), roshi, (t / 3.0).floor() as i64 % 2 == 0, 0.95);
            }
        }
        if world == 8 {
            let fa = self.built.fut_andro.clone();
            for (i, p) in fa.iter().enumerate() {
                let v = ap(&m, *p);
                if v[2] > 0.05 {
                    c.blit_z(
                        sprite::get(if i > 0 { "A18" } else { "A17" }),
                        cx + v[0] * r,
                        cy - v[1] * r + bob,
                        stand_ang(v),
                        i == 0,
                        0.0,
                        v[2] * r + 6.0,
                        1.0,
                    );
                }
            }
            let v = ap(&m, g.frame3.cap_pt(1.3, 3.25));
            if v[2] > 0.05 {
                c.blit_z(sprite::get("FUSED.futureGohan"), cx + v[0] * r, cy - v[1] * r + bob, stand_ang(v), false, 0.0, v[2] * r + 6.0, 0.85);
            }
            for p in self.built.smokes.clone() {
                let v = ap(&m, p);
                if (v[2] > 0.1 || hypot(&[v[0], v[1]]) > 1.02) && g.rng.random() < dt * 9.0 {
                    let x = cx + v[0] * r + (g.rng.random() - 0.5) * 3.0;
                    let vx = 2.0 + g.rng.random() * 3.0;
                    let vy = -11.0 - g.rng.random() * 5.0;
                    g.fx.push(Particle::new(x, cy - v[1] * r + bob, &[0x5a5e64, 0x4a4e54, 0x3a3e44, 0x2e3238]).v(vx, vy).life(3.4).size(2.0, 4.0));
                }
            }
            for i in 0..90 {
                let i = i as f64;
                let y = (hash(i * 5.3) * c.h + t * (150.0 + hash(i * 3.1) * 60.0)) % (c.h + 8.0) - 4.0;
                let x = ((hash(i * 1.7) * c.w - y * 0.3) % c.w + c.w) % c.w;
                for k in 0..4 {
                    let k = k as f64;
                    c.fb.px(js_round(x + k * 0.3), js_round(y - k), 0xc8d4e0, 0.45);
                }
            }
        }
        if crate::tones::is_w2(world) {
            self.draw_w2(c, g, dt);
        }
        if world == 5 {
            let v = ap(&m, g.frame3.cap_pt(0.55, 3.2));
            if v[2] > 0.05 {
                c.blit_z(
                    sprite::get("BULMA"),
                    cx + v[0] * r,
                    cy - v[1] * r + bob,
                    atan2(v[0], v[1]),
                    (t / 2.0).floor() as i64 % 2 == 0,
                    0.0,
                    v[2] * r + 6.0,
                    1.0,
                );
            }
        }
        if world == 7 {
            self.draw_paozu(c, g);
        }
        // the arena's stands in front: not ported yet
        let whis_at = g.frame3.cap_pt(0.5, 3.3);
        if g.tl.bz.is_some() {
            Self::npc(c, sprite::get("NPCS.whis"), whis_at, (t / 3.0).floor() as i64 % 2 == 0, 1.0);
        }
        if world == 7 {
            Self::npc(c, sprite::get("NPCS.chichi"), g.frame3.cap_pt(0.45, 3.2), true, 1.0);
        }
        if world == 6 {
            Self::npc(c, sprite::get("NPCS.turtle"), g.frame3.cap_pt(0.5, 5.2), false, 1.0);
            Self::npc(c, sprite::get("NPCS.oolong"), g.frame3.cap_pt(0.62, 4.1), (t / 2.5).floor() as i64 % 2 == 0, 1.0);
        }
        if let Some(bz) = g.tl.bz.clone() {
            let v = ap(&m, bz.c);
            if v[2] > 0.05 {
                let (x, y, ang) = (cx + v[0] * r, cy - v[1] * r + bob, atan2(v[0], v[1]));
                if bz.awake != 0.0 {
                    let jx = x + (g.rng.random() - 0.5) * 2.0;
                    c.blit_z(sprite::get("BEERUS"), jx, y, ang, false, 0.0, v[2] * r + 6.0, 0.9);
                    if g.rng.random() < 0.5 {
                        let px_ = x + (g.rng.random() - 0.5) * 16.0;
                        let py = y - 10.0 - g.rng.random() * 20.0;
                        g.fx.push(Particle::new(px_, py, &[0xffffff, 0xd9c8ff, 0x7a4ae0]).v(0.0, -30.0).life(0.5).size(1.0, 1.0));
                    }
                } else {
                    c.blit_z(sprite::get("BEERUS"), x, y, ang + 1.45, false, -12.0, v[2] * r + 6.0, 0.9);
                    let z = (t * 0.8) % 1.0;
                    text(
                        c.fb,
                        if z < 0.33 {
                            "z"
                        } else if z < 0.66 {
                            "zz"
                        } else {
                            "zzz"
                        },
                        x + 6.0,
                        y - 22.0 - z * 6.0,
                        0x5e44a0,
                        1.0,
                    );
                }
            }
        }
        if let Some(b) = g.tl.bean.clone() {
            if let Some((sx, sy)) = b.sx {
                let v = ap(&m, b.c);
                let hop = 3.0 + sin(t * 3.0).abs() * 3.0;
                c.blit_z(sprite::get("SENZU"), sx, sy, atan2(v[0], v[1]), false, hop, v[2] * r + 6.0, 1.5);
                if g.rng.random() < 0.12 {
                    let x = sx + (g.rng.random() - 0.5) * 14.0;
                    let y = sy - hop - 8.0 - g.rng.random() * 8.0;
                    g.fx.push(Particle::new(x, y, &[0xffffff, 0xdaf8b4]).v(0.0, -12.0).life(0.5));
                }
            }
        }
        if let Some(p) = g.tl.pud.as_mut() {
            let v = ap(&m, p.c);
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                p.sx = Some((sx, sy));
                c.blit_z(sprite::get("PUDDING"), sx, sy, atan2(v[0], v[1]), false, sin(t * 4.0).abs() * 3.0, v[2] * r + 6.0, 1.1);
            } else {
                p.sx = None;
            }
        }
        if let Some(ro) = g.tl.roshi.as_mut() {
            let v = ap(&m, ro.c);
            let up = 1f64.min((t - ro.t0) / 0.4) * 1f64.min((ro.until - t) / 0.4);
            if v[2] > 0.05 {
                let ang = atan2(v[0], v[1]);
                let (max, fired) = (ro.flag, ro.n);
                let down = fired != 0.0 && t - fired >= 0.7;
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                ro.sx = Some((sx, sy));
                let kf: i64 = if max != 0.0 && (fired == 0.0 || t - fired < 0.7) {
                    if fired != 0.0 {
                        2
                    } else if t - max < 0.55 {
                        0
                    } else {
                        1
                    }
                } else {
                    -1
                };
                let (rx, ry) = (cos(ang), sin(ang));
                if kf >= 0 {
                    c.blit_z(
                        sprite::get(["ROSHI_KAME.0", "ROSHI_KAME.1", "ROSHI_KAME.2"][kf as usize]),
                        sx + rx * 4.0,
                        sy + ry * 4.0,
                        ang,
                        false,
                        (up - 1.0) * 18.0,
                        v[2] * r + 6.0,
                        0.95,
                    );
                } else {
                    c.blit_z(sprite::get("ROSHI"), sx, sy, ang + if down { 1.45 } else { 0.0 }, false, (up - 1.0) * 18.0, v[2] * r + 6.0, 0.95);
                }
                if fired != 0.0 && t - fired < 0.6 {
                    let (ux, uy) = (sin(ang), -cos(ang));
                    let (bx, by) = (sx + rx * 24.0 + ux * 12.0, sy + ry * 24.0 + uy * 12.0);
                    let ll = c.w + c.h;
                    let mut d = 0.0;
                    while d < ll {
                        for w in -3..=3 {
                            let w = w as f64;
                            c.fb.px(bx + rx * d + ux * w, by + ry * d + uy * w, if w.abs() < 2.0 { 0xffffff } else { 0x6fc8ff }, 1.0);
                        }
                        d += 0.5;
                    }
                }
            } else {
                ro.sx = None;
            }
        }
        if let Some(k) = g.tl.kril.as_mut() {
            if k.dead == 0.0 {
                let v = ap(&m, k.c);
                let up = 1f64.min((t - k.t0) / 0.4);
                if v[2] > 0.05 {
                    let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                    k.sx = Some((sx, sy));
                    c.blit_z(sprite::get("KRILLIN"), sx, sy, atan2(v[0], v[1]), false, (up - 1.0) * 18.0 + sin(t * 6.0).abs() * 2.0, v[2] * r + 6.0, 1.0);
                    let a = (t - (k.boom - 0.35)) / 0.35;
                    if a > 0.0 {
                        let mut i = 0.0;
                        while i <= 1.0 {
                            if i < a {
                                let x = c.w + 10.0 + (sx - c.w - 10.0) * i;
                                let y = -10.0 + (sy - 8.0 + 10.0) * i;
                                for w in -1..=1 {
                                    c.fb.px(x, y + w as f64, if w != 0 { 0xc89aff } else { 0xffffff }, 1.0);
                                }
                            }
                            i += 0.01;
                        }
                    }
                } else {
                    k.sx = None;
                }
            }
        }
        for (key, spr, lift_k, flipper) in [(0u8, "ZENO", 24.0, 3.0), (1u8, "SATAN", 18.0, 0.0)] {
            let npc = if key == 0 { g.tl.zen.as_mut() } else { g.tl.satan.as_mut() };
            let Some(n) = npc else { continue };
            if key == 1 && t <= n.t0 {
                continue;
            }
            let v = ap(&m, n.c);
            let up = 1f64.min((t - n.t0) / if key == 0 { 0.6 } else { 0.3 });
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                n.sx = Some((sx, sy));
                let (flip, hop) = if key == 0 {
                    ((t / flipper).floor() as i64 % 2 == 0, sin(t * 2.5).abs() * 2.0)
                } else {
                    ((t * 2.0).floor() as i64 % 2 == 0, sin(t * 5.0).abs() * 3.0)
                };
                c.blit_z(sprite::get(spr), sx, sy, atan2(v[0], v[1]), flip, (up - 1.0) * lift_k + hop, v[2] * r + 6.0, 1.0);
            } else {
                n.sx = None;
            }
        }
        if let Some(s) = g.tl.ship.clone() {
            let v = ap(&m, s.c);
            let land = 1f64.min((t - s.t0) / 1.2);
            let lift = if s.left != 0.0 { planet_core::jsmath::pow((t - s.left) / 1.5, 2.0) } else { 0.0 };
            if v[2] > 0.05 {
                c.blit_z(
                    sprite::get("SHIP"),
                    cx + v[0] * r,
                    cy - v[1] * r + bob,
                    atan2(v[0], v[1]),
                    false,
                    (1.0 - land) * 70.0 + lift * 90.0,
                    v[2] * r + 5.0,
                    0.9,
                );
            }
        }
        if let Some(y) = g.tl.yam.as_mut() {
            let v = ap(&m, y.c);
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob + 3.0);
                y.sx = Some((sx, sy));
                if y.boom != 0.0 {
                    c.blit_z(sprite::get("SAIBA"), sx, sy - 3.0, atan2(v[0], v[1]), false, sin(t * 14.0).abs() * 2.0, v[2] * r + 6.0, 1.1);
                } else {
                    Self::decal(c, sprite::get("YAMCHA"), v, 1.1);
                }
            } else {
                y.sx = None;
            }
        }
        if let Some(gr) = g.tl.greg.as_mut() {
            let k = 1f64.min((t - gr.hop_t) / 0.35);
            let from = gr.from.unwrap_or(gr.c);
            let p = norm(add(mul(from, 1.0 - k), gr.c, k));
            let v = ap(&m, p);
            if v[2] > 0.05 {
                let (sx, sy) = (cx + v[0] * r, cy - v[1] * r + bob);
                gr.sx = Some((sx, sy));
                c.blit_z(
                    sprite::get("GREGORY"),
                    sx,
                    sy,
                    atan2(v[0], v[1]),
                    k < 1.0 && from[0] > gr.c[0],
                    sin(k * std::f64::consts::PI) * 9.0,
                    v[2] * r + 4.0,
                    1.1,
                );
            } else {
                gr.sx = None;
            }
        }
        if g.s.krillin_ally && g.tl.kril.is_none() && !g.tl.date.as_ref().is_some_and(|d| d.kr) && !crate::mech::held(g, Ally::Krillin) {
            let kv = ap(&m, g.frame3.road_pt(g.phi - 0.5));
            let (kx, ky) = (cx + kv[0] * r, cy - kv[1] * r + bob);
            let ka = ap(&m, g.frame3.road_pt(g.phi - 0.45));
            c.blit_z(sprite::get("KRILLIN"), kx, ky, atan2(kv[0], kv[1]), cx + ka[0] * r < kx, sin(t * 9.0).abs() * 2.0, kv[2] * r + 5.0, 1.0);
        }
        if g.s.lv(Upg::Partner) > 0 && g.b.fuse_until <= t && !crate::mech::held(g, Ally::Bubbles) {
            let mv = ap(&m, g.frame3.road_pt(g.phi - 0.32));
            let (mx, my, ma) = (cx + mv[0] * r, cy - mv[1] * r + bob, atan2(mv[0], mv[1]));
            let kk = 2.6f64.min(1.0 + 0.16 * (g.s.lvf(Upg::Partner)).sqrt());
            g.tl.monkey_at = if mv[2] > 0.0 { Some((mx, my, kk)) } else { None };
            if g.tl.fuse_offer > t && g.rng.random() < 0.7 {
                let x = mx + (g.rng.random() - 0.5) * 12.0;
                let y = my - 4.0 - g.rng.random() * 10.0;
                g.fx.push(Particle::new(x, y, &[0xffffff, 0xffe08a, 0xff9a3c]).v(0.0, -20.0).life(0.5));
            }
            let fr = (t * 8.0).floor() as i64 % 2;
            c.blit_z(
                sprite::get(if fr == 0 { "MONKEY.0" } else { "MONKEY.1" }),
                mx,
                my,
                ma,
                hero.flip,
                if g.tl.fuse_offer > t { sin(t * 9.0).abs() * 3.0 } else { 0.0 },
                mv[2] * r + 5.0,
                1.1 * kk,
            );
        } else {
            g.tl.monkey_at = None;
        }
        if hero.seen && g.b.fuse_until > t && g.rng.random() < 0.6 {
            let (hx, hy) = g.hand_pos();
            let x = hx + (g.rng.random() - 0.5) * 20.0;
            let y = hy + (g.rng.random() - 0.5) * 20.0;
            g.fx.push(Particle::new(x, y, &[0xffffff, 0xffe08a, 0xff9a3c, 0x6fb0ff]).v(0.0, -26.0).life(0.5).size(1.0, 1.0));
        }
    }

    /// The boss over the planet's shoulder, with his health bar, name and universe badge.
    pub(crate) fn draw_boss(&mut self, c: &mut Ctx, g: &mut Game, dt: f64) {
        let t = c.t;
        let Some(mut b) = g.tl.boss.clone() else { return };
        let Some(spr) = boss_sprite(g, &b) else { return };
        let k = 0.9f64.max(c.r / 62.0);
        b.k = k;
        let ent = 1f64.min((t - b.t0) / 1.2);
        let out = if b.dead { 1f64.min((t - b.t1) / 1.2) } else { 0.0 };
        let arena = c.world == 9;
        let fled = |v: f64| if b.fled { v } else { 0.0 };
        b.x = c.cx + c.r * if arena { 1.3 } else { 1.45 } + (1.0 - ent) * c.w * 0.4 + fled(out * c.w * 0.5);
        b.y = c.cy - c.r * if arena { 0.4 } else { 0.75 } + sin(t * 2.0) * 3.0;
        // cell, the one boss taller than 40px, keeps his name and hp bar (his mechanic's gauge) on the canvas
        if spr.h > 40 {
            b.y = b.y.max(spr.h as f64 * k + 18.0);
        }
        b.y -= fled(out * 30.0);
        let name_w = text_width(&b.name) / 2.0 + 1.0;
        b.x = b.x.min(c.w - ((spr.w as f64 - spr.ax + 1.0) * k).max(name_w).max(10.0 * k + 1.0) - 3.0 + (1.0 - ent) * c.w * 0.4 + fled(out * c.w * 0.5));
        b.flash = 0f64.max(b.flash - dt);
        if !(b.dead && !b.fled) {
            let away = g.boss_away && b.saga && !b.uni;
            for j in (0..spr.h).filter(|_| !away) {
                for i in 0..spr.w {
                    let col = spr.col[j * spr.w + (spr.w - 1 - i)];
                    if col < 0 {
                        continue;
                    }
                    let mut yy = 0.0;
                    while yy < k {
                        let mut xx = 0.0;
                        while xx < k {
                            c.fb.px(
                                b.x + (i as f64 - spr.ax) * k + xx,
                                b.y + (j as f64 - spr.h as f64) * k + yy,
                                if b.flash > 0.0 { 0xffffff } else { col as u32 },
                                1.0,
                            );
                            xx += 1.0;
                        }
                        yy += 1.0;
                    }
                }
            }
            let bw = js_round(20.0 * k);
            let bx = js_round(b.x - bw / 2.0);
            let by = js_round(b.y - spr.h as f64 * k - 7.0);
            let f = 0f64.max((b.hp / b.max).to_f64());
            let mut x = -1.0;
            while x <= bw {
                for y in -1..=2 {
                    let y = y as f64;
                    let col = if x < 0.0 || x == bw || y < 0.0 || y == 2.0 {
                        0x1d1030
                    } else if x < bw * f {
                        if f > 0.3 {
                            0x34d873
                        } else {
                            0xe0473e
                        }
                    } else {
                        0xffffff
                    };
                    c.fb.px(bx + x, by + y, col, 1.0);
                }
                x += 1.0;
            }
            let nm = b.name.to_uppercase();
            let nx = js_round(b.x - text_width(&nm) / 2.0);
            for (ox, oy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                text(c.fb, &nm, nx + ox, by - 8.0 + oy, 0xffffff, 1.0);
            }
            text(c.fb, &nm, nx, by - 8.0, 0x1d1030, 1.0);
            if b.uni {
                let (badge, tc) = uni_badge(g);
                let bx2 = js_round(b.x - text_width(&badge) / 2.0);
                for (ox, oy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    text(c.fb, &badge, bx2 + ox, b.y + 3.0 + oy, 0x1d1030, 1.0);
                }
                text(c.fb, &badge, bx2, b.y + 3.0, tc, 1.0);
            }
        }
        g.tl.boss = Some(b);
    }

    /// The ginyu force (or androids 17 and 18) lined up over the planet.
    pub(crate) fn draw_ginyu(&mut self, c: &mut Ctx, g: &mut Game, dt: f64) {
        let t = c.t;
        let Some(mut gi) = g.tl.ginyu.clone() else { return };
        for m in gi.members.iter_mut() {
            let k = 0.9f64.max(c.r / 62.0);
            m.k = k;
            let i = m.i as f64;
            let slot = if gi.androids { ((m.i as i64 + ((t - gi.t0) / 2.5).floor() as i64) % 2) as f64 - 0.5 } else { i - 2.0 };
            let ent = 1f64.min((t - gi.t0) / 1.2 + i * -0.1);
            let out = if gi.done != 0.0 { 1f64.min((t - gi.done) / 1.2) } else { 0.0 };
            m.x = c.cx + slot * c.r * 0.55 + (1.0 - 0f64.max(ent)) * c.w * 0.6;
            m.y = c.cy - c.r * 1.12 - slot.abs() * -c.r * 0.08 + sin(t * 2.0 + i) * 2.0 - out * 40.0;
            m.flash = 0f64.max(m.flash - dt);
            if m.dead {
                continue;
            }
            let name = if gi.androids { ["A17", "A18"][m.i] } else { ["GINYU.0.spr", "GINYU.1.spr", "GINYU.2.spr", "GINYU.3.spr", "GINYU.4.spr"][m.i] };
            let spr = sprite::get(name);
            let mirror = (t * 2.0 + i) % 2.0 < 1.0;
            for j in 0..spr.h {
                for ii in 0..spr.w {
                    let col = spr.col[j * spr.w + if mirror { ii } else { spr.w - 1 - ii }];
                    if col < 0 {
                        continue;
                    }
                    let mut yy = 0.0;
                    while yy < k {
                        let mut xx = 0.0;
                        while xx < k {
                            c.fb.px(
                                m.x + (ii as f64 - spr.ax) * k + xx,
                                m.y + (j as f64 - spr.h as f64) * k + yy,
                                if m.flash > 0.0 { 0xffffff } else { col as u32 },
                                1.0,
                            );
                            xx += 1.0;
                        }
                        yy += 1.0;
                    }
                }
            }
            let bw = js_round(12.0 * k);
            let bx = js_round(m.x - bw / 2.0);
            let by = js_round(m.y - spr.h as f64 * k - 5.0);
            let f = 0f64.max((m.hp / m.max).to_f64());
            let mut x = -1.0;
            while x <= bw {
                for y in -1..=1 {
                    let y = y as f64;
                    c.fb.px(
                        bx + x,
                        by + y,
                        if x < 0.0 || x == bw || y != 0.0 {
                            0x1d1030
                        } else if x < bw * f {
                            0x34d873
                        } else {
                            0xffffff
                        },
                        1.0,
                    );
                }
                x += 1.0;
            }
        }
        g.tl.ginyu = Some(gi);
    }

    fn draw_w2(&mut self, c: &mut Ctx, g: &mut Game, dt: f64) {
        let t = c.t;
        if self.built.pole {
            let y0 = js_round(c.cy + c.r * 0.98 + c.bob);
            let hw = 3f64.max(js_round(c.r * 0.07));
            let mut y = y0;
            while y < c.h {
                let mut dx = -hw;
                while dx <= hw {
                    let x = js_round(c.cx) + dx;
                    let k = (y * c.w + x) as i64;
                    if x >= 0.0 && x < c.w && !(c.fb.zb[k as usize] as f64 > -1e8) {
                        let l = 0.62 - dx / hw * 0.4;
                        let ring = (y - y0) % 26.0 < 3.0;
                        let col = if dx.abs() == hw { 0x3a2014 } else { tone(if ring { &POLE_RING } else { &POLE }, l, x as i64, y as i64) };
                        c.fb.buf[k as usize] = pk(col, 255);
                    }
                    dx += 1.0;
                }
                y += 1.0;
            }
        }
        for rs in self.built.res.clone() {
            let mut p = rs.c;
            let mut vis = 1.0;
            if !rs.blink.is_empty() {
                let per = 4.2;
                let q = t / per + rs.ph;
                let f = (q % 1.0) * per;
                p = rs.blink[q.floor() as usize % rs.blink.len()];
                vis = 1f64.min(f / 0.4).min((per - f) / 0.4);
            }
            let v = ap(&c.m, p);
            if v[2] <= 0.05 {
                continue;
            }
            let spr = sprite::get(rs.spr);
            let (x, y) = (c.cx + v[0] * c.r, c.cy - v[1] * c.r + c.bob);
            let k = rs.k;
            let hop = if rs.hop != 0.0 && vis >= 1.0 { sin(t * rs.hop + rs.ph * 3.0).abs() * 1.5 } else { 0.0 };
            if vis > 0.45 {
                let flip = if rs.flip == 2 { (t / 3.0 + rs.ph).floor() as i64 % 2 == 0 } else { rs.flip != 0 };
                c.blit_z(spr, x, y, stand_ang(v), flip, hop, v[2] * c.r + 6.0, k);
            }
            if vis < 1.0 {
                for i in -4..=4 {
                    let hh = spr.h as f64 * k * (1.15 - (i as f64).abs() / 6.0);
                    let mut j = 0.0;
                    while j < hh {
                        if h2((i * 13) as f64 + (t * 24.0).floor(), j * 7.0 + i as f64) < (1.0 - vis) * 0.8 {
                            c.fb.px(x + i as f64 * 1.5, y - j, if (j as i64 + i) % 3 != 0 { 0xbff6ff } else { 0xffffff }, 0.85);
                        }
                        j += 1.0;
                    }
                }
            }
        }
        if c.world == 12 && g.rng.random() < dt * 4.0 {
            let a = g.rng.random() * 6.28;
            let rr = c.r * g.rng.random().sqrt();
            g.fx.push(Particle::new(c.cx + cos(a) * rr, c.cy + c.bob + sin(a) * rr, &[0xffffff, 0xbff6ff]).v(0.0, -8.0).life(1.4));
        }
    }

    fn draw_paozu(&mut self, c: &mut Ctx, g: &mut Game) {
        let t = c.t;
        for &(spr, th, a0, k, sway, hop) in &self.built.dinos.clone() {
            let ph = t * 0.12 + a0 * 3.0;
            let a = a0 + sin(ph) * sway;
            let v = ap(&c.m, g.frame3.cap_pt(th, a));
            if v[2] < 0.05 {
                continue;
            }
            let ah = ap(&c.m, g.frame3.cap_pt(th, a + 0.02 * cos(ph)));
            let (x, y) = (c.cx + v[0] * c.r, c.cy - v[1] * c.r + c.bob);
            c.blit_z(
                sprite::get(spr),
                x,
                y,
                atan2(v[0], v[1]) * 1f64.min((v[0] * v[0] + v[1] * v[1]) * 1.6),
                c.cx + ah[0] * c.r < x,
                sin(t * 5.0 + a0).abs() * hop,
                v[2] * c.r + 6.0,
                k,
            );
        }
        let (e1, e2, n) = (g.frame3.e1, g.frame3.e2, g.frame3.n);
        let ring = |a: f64| norm(add(add(mul(e1, cos(a)), e2, sin(a)), n, 0.55));
        let pa = t * 0.3;
        let pv = ap(&c.m, mul(ring(pa), 1.6));
        let pn = ap(&c.m, ring(pa + 0.05));
        let (px_, py) = (c.cx + pv[0] * c.r, c.cy - pv[1] * c.r + c.bob);
        c.blit_z(sprite::get("DINOS.d"), px_, py, 0.0, pn[0] * 1.6 * c.r + c.cx < px_, 0.0, pv[2] * c.r + 4.0, 0.42);
    }

    /// The door to the time chamber, piccolo's zap, ki blasts and the dragon-ball-less extras after the boss.
    pub(crate) fn draw_late(&mut self, c: &mut Ctx, g: &mut Game, dt: f64) {
        let t = c.t;
        if let Some(d) = g.tl.door.as_mut() {
            let v = ap(&c.m, d.c);
            let a = 1f64.min((t - d.t0) / 0.5);
            if v[2] > 0.05 {
                let ang = atan2(v[0], v[1]);
                let (sx, sy) = (c.cx + v[0] * c.r, c.cy - v[1] * c.r + c.bob);
                d.sx = Some((sx, sy));
                c.blit_z(sprite::get("DOOR"), sx, sy, ang, false, (a - 1.0) * 18.0, v[2] * c.r + 6.0, 1.1);
                if g.rng.random() < 0.3 {
                    let h = g.rng.random() * 18.0;
                    let x = sx + sin(ang) * h + (g.rng.random() - 0.5) * 10.0;
                    g.fx.push(Particle::new(x, sy - cos(ang) * h, &[0xffffff, 0xf6f4ee]).v(sin(ang) * 10.0, -cos(ang) * 10.0).life(0.6).size(1.0, 0.0));
                }
            } else {
                d.sx = None;
            }
        }
        if let Some((x0, y0, x1, y1, t0)) = g.tl.zap {
            let k = (t - t0) / 0.3;
            if k > 1.0 {
                g.tl.zap = None;
            } else {
                let (dx, dy) = (x1 - x0, y1 - y0);
                let l = hypot(&[dx, dy]);
                let l = if l == 0.0 { 1.0 } else { l };
                let (nx, ny) = (-dy / l, dx / l);
                let mut d = 0.0;
                while d < l {
                    let w = sin(d * 0.5) * 2.2;
                    c.fb.px(x0 + dx * d / l + nx * w, y0 + dy * d / l + ny * w, 0xc89aff, 1.0);
                    c.fb.px(x0 + dx * d / l, y0 + dy * d / l, 0xfff6a0, 1.0);
                    d += 0.6;
                }
            }
        }
        self.draw_ginyu(c, g, dt);
        // the time chamber: every frame leaves a ghost of the last, so everything trails afterimages
        if g.b.chamber_until > t {
            let e = 1f64.min((t - g.tl.chamber_t0) / 0.5).min((g.b.chamber_until - t) / 0.5);
            let f = 0.62 * e;
            let n = c.fb.buf.len();
            if self.echo.len() != n {
                self.echo = c.fb.buf.clone();
            }
            let mix = |a: u32, b: u32, q: f64| (a as f64 + (b as f64 - a as f64) * q) as i64 as i32 as u32 & 255;
            for k in 0..n {
                let (p, q0) = (c.fb.buf[k], self.echo[k]);
                let (al, a2) = (p >> 24, q0 >> 24);
                if al == 0 && a2 == 0 {
                    continue;
                }
                let q = if al != 0 { f * a2 as f64 / 255.0 } else { 1.0 };
                let r = mix(p & 255, q0 & 255, q);
                let gg = mix((p >> 8) & 255, (q0 >> 8) & 255, q);
                let b = mix((p >> 16) & 255, (q0 >> 16) & 255, q);
                let a = if al != 0 { al } else { ((a2 as f64 * f) as i64 as u32) & 255 };
                let v = pk((r << 16) | (gg << 8) | b, a);
                c.fb.buf[k] = v;
                self.echo[k] = v;
            }
        } else {
            self.echo.clear();
        }
        for kb in g.tl.kis.clone() {
            if kb.disc {
                for x in -5..=5 {
                    c.fb.px(kb.x + x as f64, kb.y, if (x as i64).abs() < 3 { 0xfff6c0 } else { 0xffd23a }, 1.0);
                }
                for x in -3..=3 {
                    c.fb.px(kb.x + x as f64, kb.y - 1.0, 0xffe880, 1.0);
                    c.fb.px(kb.x + x as f64, kb.y + 1.0, 0xffe880, 1.0);
                }
            } else {
                for y in -2..=2 {
                    for x in -2..=2 {
                        let d = hypot(&[x as f64, y as f64]);
                        if d <= 2.2 {
                            c.fb.px(kb.x + x as f64, kb.y + y as f64, if d < 1.2 { 0xffffff } else { 0x8fd8ff }, 1.0);
                        }
                    }
                }
            }
        }
    }

    /// Dr. gero (or 19) hanging on, draining you through a red line from your hands.
    pub(crate) fn draw_gero(&mut self, c: &mut Ctx, g: &mut Game) {
        let t = c.t;
        let Some(ge) = g.tl.gero.clone() else { return };
        let spr = sprite::get(if ge.name == "android 19" { "A19" } else { "A20" });
        let (hx, hy) = g.hand_pos();
        if t - ge.t0 > 1.2 {
            let mut i = 0.0;
            while i <= 1.0 {
                if (i * 50.0 + t * 20.0) % 4.0 < 2.0 {
                    c.fb.px(hx + (ge.x - hx) * i, hy + (ge.y - spr.h as f64 * 0.55 - hy) * i, 0xe0473e, 1.0);
                }
                i += 0.02;
            }
        }
        blit(c.fb, spr, ge.x - spr.w as f64 / 2.0, ge.y - spr.h as f64 + sin(t * 3.0) * 2.0, true);
    }
}

/// The boss's sprite: his saga's, a movie villain's, nappa, or a universe fighter's current take.
pub(crate) fn boss_sprite(g: &Game, b: &planet_core::timeline::Boss) -> Option<&'static Sprite> {
    use planet_core::data::VILLAINS;
    let n = if b.name == "past vegeta" { "vegeta" } else { b.name.as_str() };
    if b.uni {
        let key = format!("TOP_FIGHTERS.{n}.{}", b.p);
        return sprite::baked().get(&key).or_else(|| if n == "jiren" { sprite::baked().get("VILLAINS.10.spr") } else { None });
    }
    if b.movie {
        return sprite::baked().get(&format!("MOVIE.{n}"));
    }
    if n == "nappa" && g.s.world == 2 {
        return Some(sprite::get("NAPPA"));
    }
    VILLAINS.iter().position(|v| v.0 == n).and_then(|i| sprite::baked().get(&format!("VILLAINS.{i}.spr")))
}

/// `uniBadge(S.uni)` (and BOSS on the last fight) in `uniColour`.
fn uni_badge(g: &Game) -> (String, u32) {
    use planet_core::data::{LADDER, UNI_STAGES};
    let u = g.s.uni as usize;
    let last = g.s.uni_stage as usize == UNI_STAGES.len() - 1;
    let (badge, tc) = if u < LADDER.len() {
        (format!("U{}", LADDER[u].0), LADDER[u].2)
    } else {
        let tint = [0x8a4ae0, 0xe8ecf4, 0xf2c84a];
        (format!("U-INF{}", if u > LADDER.len() { format!(" {}", u - LADDER.len() + 1) } else { String::new() }), tint[u % 3])
    };
    (badge + if last { " BOSS" } else { "" }, tc)
}

/// `mvBlit`: a sprite scaled by k, standing on (x, y), optionally flashed white or faded.
fn mv_blit(c: &mut Ctx, spr: &Sprite, x: f64, y: f64, k: f64, flip: bool, white: bool, a: f64) {
    for j in 0..spr.h {
        for i in 0..spr.w {
            let col = spr.col[j * spr.w + if flip { spr.w - 1 - i } else { i }];
            if col < 0 {
                continue;
            }
            let mut yy = 0.0;
            while yy < k {
                let mut xx = 0.0;
                while xx < k {
                    c.fb.px(x + (i as f64 - spr.ax) * k + xx, y + (j as f64 - spr.h as f64) * k + yy, if white { 0xffffff } else { col as u32 }, a);
                    xx += 1.0;
                }
                yy += 1.0;
            }
        }
    }
}

impl Scene {
    /// `movieDraw`: the henchmen in a row (janemba's double mirrored on the left), the supernova, bujin's web, the
    /// mirrored space and the jelly wobble.
    pub(crate) fn draw_movie(&mut self, c: &mut Ctx, g: &mut Game, dt: f64) {
        let t = c.t;
        let (Some(mut m), Some(b)) = (g.tl.movie.clone(), g.tl.boss.clone()) else { return };
        let Some(bspr) = sprite::baked().get(&format!("MOVIE.{}", b.name)) else { return };
        let n = m.o.minions.len() as f64;
        let w = c.w;
        for (i, h) in m.minions.iter_mut().enumerate() {
            let spr = sprite::get(&format!("MOVIE.{}", h.name));
            if h.twin {
                h.k = b.k;
                let (x, y) = (c.cx - c.r * 1.45, c.cy - c.r * 0.75 + sin(t * 2.0 + 1.0) * 3.0);
                h.x = Some(((spr.w as f64 / 2.0 + 2.0) * h.k + 3.0).max((c.w - (spr.w as f64 / 2.0 + 2.0) * h.k - 3.0).min(x)));
                h.y = (spr.h as f64 * h.k + 18.0).max((c.h * 0.8).min(y));
            } else {
                h.k = 0.9f64.max(c.r / 62.0);
                let fi = i as f64;
                let slot = fi - (n - 1.0) / 2.0;
                let e = 0f64.max(1f64.min((t - m.t0) / 1.2 - fi * 0.1));
                let (tx, ty) = (c.cx + slot * c.r * 0.55, c.cy - c.r * 1.12 + slot.abs() * c.r * 0.08 + sin(t * 2.0 + fi) * 2.0);
                h.x = Some(tx + c.w * 0.6 * (1.0 - e));
                h.y = ty;
            }
            h.flash = 0f64.max(h.flash - dt);
            if h.dead {
                continue;
            }
            let hx = h.x.unwrap();
            let fi = if h.twin { 0.0 } else { i as f64 };
            mv_blit(c, spr, hx, h.y, h.k, if h.twin { false } else { (t * 2.0 + fi) % 2.0 < 1.0 }, h.flash > 0.0, 1.0);
            let bw = js_round(if h.twin { 20.0 } else { 12.0 } * h.k);
            let bx = js_round(hx - bw / 2.0);
            let by = js_round(h.y - spr.h as f64 * h.k - 5.0);
            let f = 0f64.max((h.hp / h.max).to_f64());
            let mut x = -1.0;
            while x <= bw {
                for y in -1..=1 {
                    let y = y as f64;
                    c.fb.px(
                        bx + x,
                        by + y,
                        if x < 0.0 || x == bw || y != 0.0 {
                            0x1d1030
                        } else if x < bw * f {
                            0x34d873
                        } else {
                            0xffffff
                        },
                        1.0,
                    );
                }
                x += 1.0;
            }
        }
        let over = m.over != 0.0;
        let web = m.web.is_some();
        let (flip_until, wob, nova) = (m.flip_until, m.wob, m.nova);
        let bujin = m.minions.iter().find(|h| h.name == "bujin").cloned();
        g.tl.movie = Some(m);
        if over {
            return;
        }
        if let Some((n0, n1)) = nova {
            let p = 1f64.min((t - n0) / (n1 - n0));
            let r = (3.0 + 11.0 * p) * b.k;
            let up = b.y - bspr.h as f64 * b.k - 18.0 - r;
            let ny = (r + 4.0).max(up);
            let nx = if up < r + 4.0 { b.x - bspr.w as f64 * b.k / 2.0 - r - 6.0 } else { b.x };
            let mut yy = -r - 3.0;
            while yy <= r + 3.0 {
                let mut xx = -r - 3.0;
                while xx <= r + 3.0 {
                    let d = hypot(&[xx, yy]);
                    if d <= r {
                        c.fb.px(
                            nx + xx,
                            ny + yy,
                            if d < r * 0.45 {
                                0xfff2d0
                            } else if d < r * 0.8 {
                                0xffb050
                            } else {
                                0xf06a20
                            },
                            1.0,
                        );
                    } else if d <= r + 3.0 && crate::bayer(js_round(nx + xx) as i64 as i32, js_round(ny + yy) as i64 as i32) < 0.5 {
                        c.fb.px(nx + xx, ny + yy, 0xffd0a0, 0.7);
                    }
                    xx += 1.0;
                }
                yy += 1.0;
            }
        }
        if web {
            let (hx, hy) = g.hand_pos();
            if let Some(bj) = bujin {
                let spr = sprite::get("MOVIE.bujin");
                let bx = bj.x.unwrap_or(0.0);
                let mut i = 0.0;
                while i <= 1.0 {
                    c.fb.px(
                        bx + (hx - bx) * i + sin(i * 30.0 + t * 8.0) * 2.0,
                        bj.y - spr.h as f64 * bj.k / 2.0 + (hy - bj.y + spr.h as f64 * bj.k / 2.0) * i,
                        if i * 60.0 % 4.0 < 2.0 { 0xff9af0 } else { 0x9ae84a },
                        1.0,
                    );
                    i += 0.015;
                }
            }
            let mut a = 0.0;
            while a < 6.28 {
                for r in [8.0, 14.0] {
                    c.fb.px(hx + cos(a) * r, hy + sin(a) * r, 0xff9af0, 0.8);
                }
                a += 0.05;
            }
            let mut a = 0.0;
            while a < 6.28 {
                for r in 0..14 {
                    let r = r as f64;
                    c.fb.px(hx + cos(a) * r, hy + sin(a) * r, 0xffc8f6, 0.8);
                }
                a += 0.785;
            }
        }
        if flip_until > t {
            for y in 0..c.fb.h {
                if (y >> 2) % 2 != 0 {
                    c.fb.px(w / 2.0, y as f64, 0xc070ff, 0.7);
                }
            }
            mv_blit(c, bspr, w - b.x, b.y, b.k, false, false, 0.45);
        }
        if wob > t {
            let amp = 3.0 * (wob - t);
            let wi = c.fb.w;
            for y in 0..c.fb.h {
                let s = js_round(sin(y as f64 * 0.12 + t * 14.0) * amp);
                if s == 0.0 {
                    continue;
                }
                let row: Vec<u32> = c.fb.buf[y * wi..(y + 1) * wi].to_vec();
                for x in 0..wi {
                    c.fb.buf[y * wi + x] = row[0f64.max((wi as f64 - 1.0).min(x as f64 - s)) as usize];
                }
            }
        }
    }
}
