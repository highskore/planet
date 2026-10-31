//! The simulation: game.js `frame(t, dt)` and the inputs that change state (training taps, the shop, kaioken, the tree,
//! ascending), with the economy as methods. Expressions keep game.js's operand order so floats round the same way.

use crate::big::Big;
use crate::data::*;
use crate::fx::Particle;
use crate::geom::{Cam, Frame};
use crate::hud::{Cue, Hud};
use crate::input::Input;
use crate::num::js_round;
use crate::rng::Lcg;
use crate::state::Save;
use crate::timeline::Tl;

/// Timed buffs. Each is the `st.t` it runs out at.
#[derive(Clone, Debug, Default)]
pub struct Buffs {
    pub car_until: f64,
    pub boost_until: f64,
    pub ape_until: f64,
    pub senzu_until: f64,
    pub kaio_until: f64,
    pub tired_until: f64,
    pub kaio_ready: f64,
    pub asc_ready: f64,
    pub daily_until: f64,
    pub fuse_until: f64,
    pub fused: Option<(f64, f64)>, // (x, until)
    pub laugh_until: f64,
    pub chamber_until: f64,
    pub rage: f64,
    pub rage_until: f64,
    pub chi_until: f64,
    pub roshi_until: f64,
    pub calm_until: f64,
    pub gadget_until: f64,
    pub whis_until: f64,
    pub pose_until: f64,
    /// west city gadgets: a robot that taps, a hover bike that sprints
    pub robot_until: f64,
    pub bike_until: f64,
    /// the power surge a tapped-through transformation earns
    pub surge_until: f64,
}

/// Shenron's timed wish buffs (`wishBuff`), by key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WishBuff {
    Fly,
    Fast,
    Taps,
    Idle,
    Laps,
    Crit,
    Combo,
}

pub struct Game {
    pub s: Save,
    pub rng: Lcg,
    pub forms: Vec<Form>,
    pub b: Buffs,
    pub wish_buff: [f64; 7],
    /// `st.t`: the time of the last frame
    pub t: f64,
    pub session: f64,
    pub playing: bool,
    pub phi: f64,
    pub laps: u32,
    pub last_lap: f64,
    pub sprint: f64,
    pub riding: f64,
    /// the `#snake-nimbus` preview hook (any hash with `nimbus`): snake way starts riding the nimbus
    pub snake_nimbus: bool,
    /// snake way's slot in the trip rotation: `SNAKE_WAY_ENABLED`, or on with the `#snake` preview hook
    pub snake_way: bool,
    pub acc: f64,
    pub auto_acc: f64,
    pub combo: f64,
    pub last_click: f64,
    pub shake: f64,
    /// prefers-reduced-motion (`st.calm`): the screen doesn't shake, and the shake's random draw isn't made
    pub calm: bool,
    /// the widget shows a saga boss out on your desktop for a while: the renderer leaves his sprite off the planet (his
    /// bar and name stay). Never saved; game.js has no such thing
    pub boss_away: bool,
    /// `ui.event` toasts (text, ms), oldest first; the HUD bridge drains them
    pub events: Vec<(String, f64)>,
    pub cam: Cam,
    pub frame3: Frame,
    /// the timeline: visitors, raids, trips, the dragon
    pub tl: Tl,
    /// the pointer: drag and spin, beam charge, the big bang
    pub inp: Input,
    /// sounds and title effects (`ui.sfx`, `ui.fx`, ...), oldest first; the HUD bridge drains them
    pub cues: Vec<Cue>,
    /// the HUD as game.js hands it to `ui.update`, rebuilt at that point of every frame while playing
    pub hud: Hud,
    /// `ui.shop`'s last argument: rebuilt where game.js calls `syncShop` (the HUD point of a frame, buys, the tree)
    pub shop: String,
    /// the wall clock at t = 0, in ms (achievements are stamped with it); the host sets it
    pub epoch_ms: f64,
    /// how high the hero is drawn off the road and whether he faces left: the renderer sets these
    pub lift_now: f64,
    pub flip_now: bool,
    /// particles (game.js `fx`): the simulation and the renderer both add, the renderer steps them
    pub fx: Vec<crate::fx::Particle>,
    /// the floating gain numbers (game.js `floats`): the simulation adds, the renderer ages them
    pub floats: Vec<crate::floats::Float>,
    /// `flashT`: the white flash after a transform, a trip, a big hit; the renderer runs it down
    pub flash_t: f64,
    /// whether the hero was in view the last time he was drawn (a full beam charge makes the screen tremble then)
    pub hero_seen: bool,
    /// planet-render draws this game: a full charge's tremble then comes from the drawing, as in game.js, not from
    /// `draw_step`'s guess from the last frame
    pub drawn_here: bool,
    /// the capsule houses as game.js last built the scenery (`scenery()`): a wish's free levels don't rebuild it
    pub scenery_house: u32,
    /// the universes the arena showed erased at game.js's last `topSync()` (a poked `S.uni` doesn't redraw it)
    pub top_uni: u32,
    /// the transformation scene, while one runs
    pub xf: Option<crate::xform::XForm>,
    /// a form a preview hook or a test poke sets up lands at once, without its scene
    pub xf_instant: bool,
    /// the boss mechanics off (the AFK sims' baseline)
    pub no_mechs: bool,
    /// a limit break or a milestone playing (limits.rs)
    pub mo: Option<crate::limits::Moment>,
    /// strain from play waiting for the next frame, the last tap that counted, and the strain per second (smoothed)
    pub strain_extra: f64,
    pub strain_tap_t: f64,
    pub strain_rate: f64,
    /// the next milestone, worked out once per milestone (limits.rs)
    pub next_mile: std::cell::RefCell<crate::limits::NextMile>,
    /// the ki ceiling, kept until what it's made of changes (limits.rs)
    pub ceil_memo: std::cell::Cell<Option<crate::limits::CeilMemo>>,
    pub floor_memo: std::cell::Cell<Option<(u32, Big)>>,
    /// the last big buy, while undo can still take it back (buy.rs)
    pub undo: Option<crate::buy::Undo>,
    pub undo_seq: u32,
    /// `(id, text)`: an undo offer the HUD hasn't shown yet
    pub undo_offer: Option<(u32, String)>,
    /// times something drained power (dr. gero): an undo can't rewind past one
    pub drains: u32,
}

impl Game {
    pub fn new(seed: u32) -> Self {
        let s = Save::new();
        let forms = ladder_of(s.hero, s.limits);
        let mut g = Game {
            s,
            rng: Lcg::new(seed),
            forms,
            b: Buffs { rage: 2.0, ..Default::default() },
            wish_buff: [0.0; 7],
            t: 0.0,
            session: 0.0,
            playing: false,
            phi: 0.0,
            laps: 0,
            last_lap: 0.0,
            sprint: 0.0,
            riding: 0.0,
            snake_nimbus: false,
            snake_way: SNAKE_WAY_ENABLED,
            acc: 0.0,
            auto_acc: 0.0,
            combo: 1.0,
            last_click: -9.0,
            shake: 0.0,
            calm: false,
            boss_away: false,
            events: Vec::new(),
            cam: Cam::new(200.0, 134.0), // the harness's 600×400 canvas in game.js pixels; the HUD bridge resizes it
            frame3: Frame::new(),
            tl: Tl::default(),
            inp: Input::default(),
            cues: Vec::new(),
            hud: Hud::default(),
            shop: String::new(),
            epoch_ms: 0.0,
            lift_now: 0.0,
            flip_now: false,
            fx: Vec::new(),
            floats: Vec::new(),
            flash_t: 0.0,
            hero_seen: false,
            drawn_here: false,
            scenery_house: 0,
            top_uni: 0,
            xf: None,
            xf_instant: false,
            no_mechs: false,
            mo: None,
            strain_extra: 0.0,
            strain_tap_t: -9.0,
            strain_rate: 1.0,
            next_mile: Default::default(),
            ceil_memo: Default::default(),
            floor_memo: Default::default(),
            undo: None,
            undo_seq: 0,
            undo_offer: None,
            drains: 0,
        };
        g.sync_shop();
        g
    }

    /// game.js `start()`; the daily bonus needs a date and waits for the save bridge.
    pub fn start(&mut self) {
        self.playing = true;
        self.sync_shop();
    }

    /// `syncShop()`: what `ui.shop` gets now.
    pub(crate) fn sync_shop(&mut self) {
        self.shop = self.shop_json();
    }

    // ── economy

    pub fn lvl(&self, node: usize) -> f64 {
        self.s.tree[node] as f64
    }

    pub fn ang(&self, a: usize) -> bool {
        self.s.angel[a]
    }

    pub fn wished(&self, k: WishBuff) -> bool {
        self.wish_buff[k as usize] > self.t
    }

    /// Producer milestone multiplier.
    pub fn ms(&self, u: Upg) -> Big {
        Self::ms_at(self.s.lv(u))
    }

    /// The producer milestones' doubling at level `l`.
    pub fn ms_at(l: u32) -> Big {
        let hit = MS.iter().filter(|&&n| l >= n).count() as f64;
        let past = ((l as f64 - 100.0) / 50.0).floor().max(0.0);
        Big::from(2f64).pow(hit + past)
    }

    pub fn seed_boost_of(&self, a: Big) -> Big {
        let b = 1.0 + a * (0.1 + 0.02 * self.lvl(N_SEEDX));
        if b <= SEED_SOFT {
            b
        } else {
            SEED_SOFT * (b / SEED_SOFT).pow(if self.s.limits >= 2 { crate::data::SEED_EXP_II } else { SEED_EXP })
        }
    }

    pub fn seed_boost(&self) -> Big {
        self.seed_boost_of(self.s.ascend)
    }

    pub fn combo_cap(&self) -> f64 {
        if self.lvl(N_PRIDE) > 0.0 {
            4.0
        } else {
            2.0
        }
    }

    pub fn tree_open(&self) -> bool {
        self.s.seeds > 0.0 || self.s.tree.iter().any(|&v| v > 0)
    }

    pub fn seeds_now(&self) -> Big {
        let p = self.s.pl / 1e4;
        (if self.ang(A_DIVINE) { p.pow(0.55) } else { p.sqrt() }).floor().max(1.0)
    }

    pub fn god_now(&self) -> u32 {
        (if self.s.gods > 0 { 0 } else { 1 }).max(self.s.uni) * if self.ang(A_PRIEST) { 2 } else { 1 }
    }

    pub fn zeno_ready(&self) -> bool {
        self.s.gods >= ZENO_GODS * (1 + self.s.zeno)
    }

    /// God ki's boost for `n` god ki: ×(1+n), or compounding once limit IV is broken (whichever is more).
    pub fn god_boost_of(&self, n: f64) -> Big {
        let lin = Big::from(1.0 + GOD_X * n);
        if self.s.limits >= 4 {
            lin.max(Big::from(crate::data::GOD_X_IV).pow(n))
        } else {
            lin
        }
    }

    /// A universe star's boost: ×1.5, or ×1.75 once limit V is broken.
    pub fn star_x(&self) -> f64 {
        if self.s.limits >= 5 {
            crate::data::STAR_X_V
        } else {
            STAR_X
        }
    }

    pub fn capped(&self, m: Big) -> Big {
        let c = self.ceiling();
        if m <= c {
            m
        } else {
            // through jsmath's log and exp, not the platform's pow: the same bits on every target, as the ceiling now moves
            // with play (glibc's and Apple's pow differed by an ulp here)
            c * Big::exp10((m / c).log10() * CEIL_EXP)
        }
    }

    pub fn late_boost(&self) -> Big {
        let stars = self.s.ustars as f64 * if self.ang(A_STARS) { 2.0 } else { 1.0 };
        Big::from(self.star_x()).pow(stars)
            * Big::from(MASTERY_X).pow(self.s.mastery as f64)
            * self.god_boost_of(self.s.god_total as f64)
            * Big::from(ZENO_X).pow(self.s.zeno as f64)
    }

    /// The engine the ceiling caps; forms sit outside it. Every limit broken is a floor under it, even right after god
    /// ki empties the engine.
    pub fn core(&self) -> Big {
        let w = self.s.wishes as f64;
        (Big::from(1.15f64).pow(self.s.lvf(Upg::Gravity)) * Big::from(1.25f64).pow(w) * (1.0 + 0.05 * self.lvl(N_WISH) * w) * self.seed_boost() * ECON)
            .max(if self.s.limits > 0 { 1.0 } else { 0.0 })
            * self.floor()
    }

    pub fn tail_lv(&self) -> f64 {
        let t = self.s.lvf(Upg::Tail);
        if t <= 10.0 {
            t
        } else {
            10.0 + 2.0 * (t - 10.0).sqrt()
        }
    }

    /// His bonus on top of ×1; only his size and time slow past lv 10.
    pub fn ape_x(&self) -> f64 {
        2.0 + 0.25 * self.s.lvf(Upg::Tail) + if self.tl.ape.giant { 1.0 } else { 0.0 }
    }

    /// Crit chance past 100%: each 10% of it is +20% crit damage.
    pub fn crit_over(&self) -> f64 {
        (self.s.lvf(Upg::Scouter) * 0.1 + self.lvl(N_CRIT) * 0.08 - 1.0).max(0.0)
    }

    pub fn crit_hit(&self) -> f64 {
        (2.0 + self.lvl(N_CRITX)) * (1.0 + 2.0 * self.crit_over())
    }

    /// A senzu bean's boost at a senzu tree level: ×1.5, +0.1 a level past lv 3 (when beans come round fast enough).
    pub fn senzu_x_at(lv: f64) -> f64 {
        (15.0 + (lv - 3.0).max(0.0)) / 10.0
    }

    pub fn senzu_x(&self) -> f64 {
        Self::senzu_x_at(self.s.lvf(Upg::Senzu))
    }

    /// A full charge's pay at a galick level: +5% a level.
    pub fn beam_x_at(lv: f64) -> f64 {
        (19.0 + lv.max(1.0)) / 20.0
    }

    /// A big bang's pay, in full beams, at a level: 1.3, +0.05 a level.
    pub fn bang_x_at(lv: f64) -> f64 {
        (25.0 + lv) / 20.0
    }

    pub fn kaio_x(&self) -> f64 {
        1.25 + 0.25 * self.s.lvf(Upg::Kaio)
    }

    /// Instant transmission's cooldown, from the end of a jump's trip: 5 min, down to 90s.
    pub fn itx_cd(&self) -> f64 {
        (300.0 / (1.0 + 0.1 * (self.s.lvf(Upg::Itx) - 1.0))).max(90.0)
    }

    /// How much sooner pods, the nimbus and trips come with instant transmission: up to 30%.
    pub fn soon(&self) -> f64 {
        (1.0 / (1.0 + 0.04 * self.s.lvf(Upg::Itx))).max(0.7)
    }

    pub fn chamber_x(&self) -> f64 {
        1.5 + 0.5 * self.s.lvf(Upg::Chamber)
    }

    /// Permanent boosts multiply; timed ones add their bonuses. `bare` leaves the timed ones out (windfalls, villain hp).
    pub fn mult(&self, bare: bool) -> Big {
        let t = self.t;
        let b = &self.b;
        let on = |until: f64, x: f64| if until > t { x } else { 0.0 };
        let bonus = if bare {
            1.0
        } else {
            1.0 + on(b.car_until, 0.5)
                + on(b.boost_until, 1.0)
                + on(b.ape_until, self.ape_x())
                + on(b.senzu_until, self.senzu_x() - 1.0)
                + if self.riding > 0.0 { 0.5 } else { 0.0 }
                + on(b.kaio_until, self.kaio_x() - 1.0)
                + on(b.daily_until, 0.25)
                + on(b.fuse_until, 1.0)
                + b.fused.map_or(0.0, |(x, until)| on(until, x - 1.0))
                + on(b.laugh_until, 0.25)
                + on(b.chamber_until, self.chamber_x() - 1.0)
                + on(b.rage_until, b.rage - 1.0)
                + on(b.chi_until, 0.5)
                + on(b.roshi_until, 0.5)
                + on(b.calm_until, 0.25)
                + on(b.gadget_until, 0.5)
                + on(b.surge_until, crate::xform::SURGE_X - 1.0)
        };
        let tired = if b.kaio_until <= t && b.tired_until > t { 0.75 } else { 1.0 };
        let awake = if self.tl.bz.as_ref().is_some_and(|b| b.awake != 0.0) { 0.25 } else { 1.0 };
        self.capped(self.core()) * self.forms[self.s.form].x * self.late_boost() * bonus * tired * awake
    }

    pub fn lap_value(&self) -> Big {
        30.0 * (1.0 + self.s.lvf(Upg::Boots) * self.ms(Upg::Boots))
    }

    pub fn passive(&self) -> Big {
        let s = &self.s;
        (s.lvf(Upg::Partner) * 4.0 * self.ms(Upg::Partner)
            + s.lvf(Upg::House) * 40.0 * self.ms(Upg::House)
            + s.lvf(Upg::Piccolo) * 60.0 * self.ms(Upg::Piccolo))
            * (1.0 + 0.5 * self.lvl(N_IDLE))
            * if self.wished(WishBuff::Idle) { 3.0 } else { 1.0 }
            * if self.lvl(N_ETERNAL) > 0.0 { 3.0 } else { 1.0 }
            * if s.krillin_ally { 1.5 } else { 1.0 }
    }

    /// Power a second, buffs included: what the HUD shows.
    pub fn rate(&self) -> Big {
        ((self.lap_value() / 7.0 + self.passive()) * self.mult(false)).round()
    }

    /// The rate without timed buffs: windfalls and villain hp are sized on it.
    pub fn wrate(&self) -> Big {
        ((self.lap_value() / 7.0 + self.passive()) * self.mult(true)).round()
    }

    pub fn unlocked(&self, u: Upg) -> bool {
        let best = self.s.best;
        match u {
            Upg::Boots | Upg::Partner | Upg::Gravity | Upg::Cloud => true,
            Upg::Scouter => best >= 150.0,
            Upg::Galick => best >= 400.0,
            Upg::Senzu => best >= 1000.0,
            Upg::House => best >= 2000.0,
            Upg::Armor => best >= 5000.0,
            Upg::Tail => self.s.ach.contains("ape"),
            Upg::Kaio => best >= 8000.0,
            Upg::Bigbang => best >= 20000.0,
            Upg::Piccolo => best >= 10000.0,
            Upg::Chamber => best >= 40000.0,
            Upg::Itx => self.s.ach.contains("yardrat"),
        }
    }

    pub fn ascend_unlocked(&self) -> bool {
        self.s.best >= 1e6 || self.tree_open()
    }

    pub fn cost(&self, u: Upg) -> Big {
        self.cost_at(u, self.s.lvf(u))
    }

    /// One buy's price at level `l`.
    pub fn cost_at(&self, u: Upg, l: f64) -> Big {
        let it = &SHOP[u as usize];
        (it.base * Big::from(it.k).pow(l) * (1.0 - 0.05 * self.lvl(N_DEAL))).floor()
    }

    /// Ascending spends all you hold, 1M at least.
    pub fn ascend_cost(&self) -> Big {
        self.s.pl.floor().max(1e6)
    }

    pub fn god_cost(&self) -> Big {
        self.s.pl.floor().max(2e14)
    }

    // ── state changes

    /// `ui.event(text)`: a toast for the default 2.6s.
    pub(crate) fn event(&mut self, s: impl Into<String>) {
        self.events.push((s.into(), 2600.0));
    }

    /// `ui.event(text, ms)`: a toast that stays up `ms` milliseconds.
    pub(crate) fn event_ms(&mut self, s: impl Into<String>, ms: f64) {
        self.events.push((s.into(), ms));
    }

    /// `ach(id)`: an achievement, once, stamped with the wall clock; its toast follows 0.9s later.
    pub(crate) fn ach(&mut self, id: &str) {
        if !self.playing || self.s.ach.contains(id) {
            return;
        }
        let Some(&(id, name, _)) = ACH.iter().find(|a| a.0 == id) else { return };
        self.s.ach.insert(id);
        self.s.ach_at.insert(id, self.now_ms());
        let unlocks = CHARS.iter().find(|c| c.3 == Some(id)).map_or(String::new(), |c| format!(" · {} unlocked", c.2));
        self.after(0.9, crate::timeline::Later::Event(format!("achievement — {name}{unlocks}"), 3200.0));
        self.cue(Cue::Sfx("coin"));
        self.cue(Cue::Achieved);
    }

    /// `Date.now()`: the host's wall clock at `t` (`epoch_ms` is the clock at t = 0).
    pub fn now_ms(&self) -> f64 {
        self.epoch_ms + (self.t * 1000.0).floor()
    }

    pub fn gain(&mut self, n: impl Into<Big>) {
        let n = n.into();
        let n = if n > 1.0 { n.round() } else { Big::from(1.0) };
        self.s.pl += n;
        if self.s.pl > self.s.best {
            self.s.best = self.s.pl;
        }
        if self.s.pl > self.s.run_best {
            self.s.run_best = self.s.pl;
        }
    }

    fn goal_need(&self, id: &str) -> bool {
        match id {
            "beam" => self.s.lv(Upg::Galick) > 0,
            "tour" => self.s.best >= 3000.0,
            _ => true,
        }
    }

    fn next_goal(&mut self) {
        let i = GOALS.iter().position(|g| g.id == self.s.goal).unwrap_or(0);
        for k in 1..=GOALS.len() {
            let g = &GOALS[(i + k) % GOALS.len()];
            if self.goal_need(g.id) {
                self.s.goal = g.id;
                self.s.goal_p = 0.0;
                return;
            }
        }
    }

    /// Training goals: progress, and a prize (and every fifth, a dragon ball) when one is done.
    pub fn goal(&mut self, id: &str, amount: f64, set: bool) {
        if id == "meteor" {
            if let Some(s) = self.tl.storm.as_mut() {
                s.hit += 1;
            }
        }
        if !self.playing || self.s.goal != id {
            return;
        }
        let g = GOALS.iter().find(|g| g.id == id).unwrap();
        self.s.goal_p = if set { self.s.goal_p.max(amount) } else { self.s.goal_p + amount };
        if self.s.goal_p >= g.n {
            self.s.goals_done += 1;
            let prize = (self.wrate() * 90.0 * (1.0 + (self.s.goals_done.min(20) as f64) * 0.15)).max(200.0);
            self.gain(prize);
            let mut msg = format!("training done: {} — +{}", g.text, crate::num::fmt_short(prize.round()));
            if self.s.goals_done.is_multiple_of(5) {
                self.s.orbs7 += 1;
                msg += " and a dragon ball";
                if self.s.orbs7 >= 7 {
                    self.s.orbs7 = 0;
                    self.after(1.5, crate::timeline::Later::Summon);
                }
            }
            self.event(msg);
            self.cue(Cue::Fx("buy"));
            self.next_goal();
        }
    }

    pub(crate) fn victory(&mut self) {
        self.b.pose_until = self.t + 1.6;
    }

    pub(crate) fn transform(&mut self, preview: bool) {
        self.transform_with(preview, "", 2600.0);
    }

    /// `transform`, with `more` added to its toast and the toast up for `ms`.
    pub(crate) fn transform_with(&mut self, preview: bool, more: &str, ms: f64) {
        self.flash_t = 1.0;
        self.s.form += 1;
        self.victory();
        if !preview {
            let f = self.s.form;
            if f >= 1 {
                self.ach("ss");
            }
            if f >= UI_FORM {
                self.ach("ui");
            }
            if f >= MUI_FORM {
                self.ach("mui");
            }
            if self.forms[f].lb > 0 {
                self.ach("lbform");
            }
            if let Some(&(_, a)) = LATE_ACH.iter().find(|l| l.0 == self.forms[f].name) {
                self.ach(a);
            }
            if f > self.s.form_top {
                self.s.form_top = f;
                self.s.form_top_name = self.forms[f].name;
            }
        }
        self.cue(Cue::Sfx("form"));
        let f = &self.forms[self.s.form];
        let msg = format!("{}!  ×{}{}{more}", f.name, f.x, if preview { " — a taste, for 60s" } else { "" });
        self.event_ms(msg, ms);
        self.cue(Cue::Form(self.s.form));
        let aura = self.forms[self.s.form].aura();
        let q = self.kid_screen(10.0);
        for _ in 0..90 {
            let vx = (self.rng.random() - 0.5) * 200.0;
            let vy = (self.rng.random() - 0.5) * 200.0;
            self.fx.push(Particle::new(q.x, q.y, &aura).v(vx, vy).drag(2.0).life(1.1));
        }
    }

    /// game.js `train()`: a tap just off the hero, so nothing on the planet takes it and it trains.
    pub fn train(&mut self) {
        let k = self.kid_screen(14.0);
        self.click(k.x + 999.0, k.y + 999.0);
    }

    /// The fall-through of `onClick`: a training tap.
    pub(crate) fn train_tap(&mut self) {
        let t = self.t;
        let cap = self.combo_cap();
        self.combo = if self.wished(WishBuff::Combo) {
            cap
        } else if t - self.last_click < 0.6 {
            (self.combo + 0.1).min(cap)
        } else {
            1.0
        };
        self.last_click = t;
        self.strain_tap();
        if self.combo >= 2.0 {
            self.goal("combo", 1.0, false);
        }
        self.sprint = (self.sprint + 0.5).min(1.2);
        let whis = self.b.whis_until > t;
        if let Some(bz) = self.tl.bz.as_mut().filter(|b| b.awake == 0.0) {
            bz.noise += if whis { 0.08 } else { 0.04 };
        }
        if self.tl.jump_h <= 0.0 {
            self.tl.jump_v = 120.0;
        }
        let crit = self.wished(WishBuff::Crit) || self.rng.random() < self.s.lvf(Upg::Scouter) * 0.1 + self.lvl(N_CRIT) * 0.08;
        let n = (1.0 + self.s.lvf(Upg::Boots) * self.ms(Upg::Boots))
            * self.mult(false)
            * self.combo
            * if crit { self.crit_hit() } else { 1.0 }
            * (1.0 + 0.5 * self.lvl(N_FIST))
            * if self.wished(WishBuff::Taps) { 3.0 } else { 1.0 };
        let k = self.kid_screen(14.0);
        self.gain_at(n, k.x, k.y - 10.0, crit);
        self.cue(Cue::Sfx("tap"));
        if crit {
            let k = self.kid_screen(14.0);
            for _ in 0..10 {
                let vx = (self.rng.random() - 0.5) * 90.0;
                let vy = -20.0 - self.rng.random() * 60.0;
                self.fx.push(Particle::new(k.x, k.y - 14.0, &[0xffffff, 0x9cf0b8, 0x34d873]).v(vx, vy).drag(2.0).life(0.5));
            }
        }
        self.cue(Cue::Bump);
    }

    /// Buys a shop item by its game.js key; false if it can't be afforded.
    pub fn buy(&mut self, key: &str) -> bool {
        match key {
            "ascend" => {
                if self.s.pl < self.ascend_cost() {
                    return false;
                }
                self.ascend();
                true
            }
            "godki" => {
                if self.s.pl < self.god_cost() {
                    return false;
                }
                self.god_reset();
                true
            }
            "zeno" => {
                self.zeno_erase();
                true
            }
            _ => self.buy_n(key, 1),
        }
    }

    /// The worlds instant transmission can reach: every one you've been but snake way and the arena.
    pub fn itx_worlds(&self) -> Vec<u32> {
        ITX_WORLDS.into_iter().filter(|w| self.s.been & 1 << w != 0).collect()
    }

    /// game.js `calm()`: nothing on (no fight, the dragon, the dance or a struggle). Not `self.calm`, reduced motion.
    pub fn calm(&self) -> bool {
        let tl = &self.tl;
        tl.tour.is_none()
            && tl.shen.is_none()
            && tl.boss.is_none()
            && tl.dance.is_none()
            && tl.struggle.is_none()
            && !tl.ginyu.as_ref().is_some_and(|g| g.done == 0.0)
            && tl.movie.is_none()
    }

    /// game.js `itxCan()`: owned, recharged, at home, and calm.
    pub fn itx_can(&self) -> bool {
        self.playing && self.s.lv(Upg::Itx) > 0 && self.t >= self.tl.itx_ready && self.s.world == self.home() && self.calm()
    }

    /// game.js `itxNo()`: why it won't go (said only once it's owned and the game is on).
    fn itx_no(&mut self) {
        if self.s.lv(Upg::Itx) == 0 || !self.playing {
            return;
        }
        if self.t < self.tl.itx_ready {
            let msg = format!("instant transmission recharges in {}s", (self.tl.itx_ready - self.t).ceil());
            self.event(msg);
        } else {
            self.event("instant transmission only works from home, with no fight on");
        }
    }

    /// game.js `itxOpen()` (the gesture or the button): the picker, or why it won't open.
    pub fn itx_open(&mut self) {
        if !self.itx_can() {
            self.itx_no();
            return;
        }
        let list = crate::json::arr(
            self.itx_worlds().into_iter().map(|w| crate::json::Obj::new().num("key", w as f64).str("label", crate::hud::WORLD_FULL[w as usize]).end()),
        );
        self.cue(crate::hud::Cue::ItxPick(list));
    }

    /// game.js `itxJump(w)`: that world's normal trip, starting now, on top of the natural ones (their timer and power
    /// mark stay); the cooldown starts when it ends.
    pub fn itx_jump(&mut self, w: u32) {
        if !self.itx_can() {
            self.itx_no();
            return;
        }
        if !self.itx_worlds().contains(&w) {
            return;
        }
        let from = self.kid_screen(0.0);
        self.tl.itx_fx = Some((from, self.t + 0.35)); // the arrival plays once the trip's flash fades
        self.trip(w);
        self.tl.itx_trip = true;
        self.ach("itx");
        self.cue(crate::hud::Cue::Sfx("itx"));
    }

    /// game.js `itxHome()`: a jumped trip is over, so the cooldown starts.
    pub(crate) fn itx_home(&mut self) {
        self.tl.itx_trip = false;
        self.tl.itx_ready = self.t + self.itx_cd();
        self.tl.itx_back = self.session;
    }

    pub fn kaioken(&mut self) {
        if !self.playing || self.s.lv(Upg::Kaio) == 0 || self.t < self.b.kaio_ready {
            return;
        }
        self.ach("kaioken");
        self.b.kaio_until = self.t + 15.0;
        self.b.tired_until = self.t + 25.0;
        self.b.kaio_ready = self.t + 45.0;
        self.shake = 0.5;
        self.flash_t = 0.4;
        let msg = format!("kaioken! ×{}", self.kaio_x());
        self.event(msg);
        let (x, y) = self.hand_pos();
        self.burst_at(x, y, &[0xffffff, 0xff9a8a, 0xe0332b], 40, 200.0);
    }

    fn branch_pts(&self, b: Branch) -> u32 {
        TREE.iter().enumerate().filter(|(_, n)| n.branch == b).map(|(i, _)| self.s.tree[i]).sum()
    }

    /// Uncapped nodes cost double every 5 ranks.
    pub fn rank_cost(n: &Node, r: u32) -> Big {
        TIER_COST[n.tier] as f64 * Big::from(2f64).pow((r / 5) as f64)
    }

    pub fn node_open(&self, i: usize) -> bool {
        self.branch_pts(TREE[i].branch) >= TIER_NEED[TREE[i].tier]
    }

    pub fn learn(&mut self, id: &str) {
        let Some(i) = node_ix(id) else { return };
        let n = &TREE[i];
        let c = Self::rank_cost(n, self.s.tree[i]);
        if !self.node_open(i) || self.s.tree[i] >= n.max || self.s.seeds < c {
            return;
        }
        self.s.seeds -= c;
        self.s.tree[i] += 1;
        self.cue(Cue::Sfx("buy"));
        self.sync_shop();
    }

    pub fn respec(&mut self) {
        for (i, n) in TREE.iter().enumerate() {
            for r in 0..self.s.tree[i] {
                self.s.seeds += Self::rank_cost(n, r);
            }
        }
        self.s.tree = [0; TREE.len()];
        self.sync_shop();
    }

    pub fn ascend(&mut self) {
        if self.t < self.b.asc_ready {
            return;
        }
        self.b.asc_ready = self.t + ASC_CD;
        let gain = self.seeds_now();
        let keep = (self.s.pl * 0.01 * self.lvl(N_HEAD)).floor();
        self.s.ascend += gain;
        self.s.seeds += gain;
        self.ach("ascend");
        self.s.pl = keep;
        self.s.run_best = keep;
        self.s.namek_at = Big::ZERO;
        self.s.foes.clear();
        let notes = self.ang(A_NOTES);
        for l in self.s.levels.iter_mut() {
            *l = if notes { (*l as f64 * 0.1).floor() as u32 } else { 0 };
        }
        self.s.form = if self.ang(A_GOLDEN) { 1 } else { 0 };
        self.tl.preview = None;
        self.cue(Cue::Form(self.s.form));
        self.scenery();
        let msg = format!(
            "ascended — +{} ki seed{} (×{} on everything), back to {}",
            crate::num::big_string(gain),
            if gain > 1.0 { "s" } else { "" },
            crate::num::fixed1(self.seed_boost()),
            self.forms[self.s.form].name
        );
        self.event_ms(msg, 4500.0);
        self.cue(Cue::Fx("super"));
        self.sync_shop();
    }

    /// God ki: seeds, the tree, forms and universes go; wishes, sagas and mastery stars stay.
    pub fn god_reset(&mut self) {
        if self.s.form < MUI_FORM || self.god_now() == 0 {
            return;
        }
        self.flash_t = 1.0;
        let g = self.god_now();
        self.s.godki += g;
        self.s.god_total += g;
        self.s.gods += 1;
        self.ach("godki");
        let s = &mut self.s;
        s.pl = Big::ZERO;
        s.run_best = Big::ZERO;
        s.namek_at = Big::ZERO;
        s.levels = [0; Upg::ALL.len()];
        s.seeds = Big::ZERO;
        s.ascend = Big::ZERO;
        s.tree = [0; TREE.len()];
        s.form = if s.angel[A_GOLDEN] { 1 } else { 0 };
        s.uni = 0;
        s.uni_stage = 0;
        s.ustars = 0;
        s.foes.clear();
        self.tl.preview = None;
        if let Some(b) = self.tl.boss.as_mut().filter(|b| b.uni && !b.dead) {
            b.dead = true;
            b.fled = true;
            b.t1 = self.t;
        }
        self.top_sync();
        self.cue(Cue::Form(self.s.form));
        self.scenery();
        let boost = crate::num::fmt_short(self.god_boost_of(self.s.god_total as f64));
        self.event_ms(format!("whis: \"ohoho, not bad.\" — +{g} god ki (×{boost} on everything). back to universe 9: spend god ki in menu → angels"), 5000.0);
        self.cue(Cue::Fx("super"));
        if self.zeno_ready() {
            self.after(5.2, crate::timeline::Later::Event("something tiny and very powerful is watching…".into(), 3500.0));
        }
        self.sync_shop();
    }

    /// game.js `scenery()`: the scenery is rebuilt from the houses owned now. Korin's tower (and the lookout's turn round
    /// it) only stands under the lookout.
    pub(crate) fn scenery(&mut self) {
        self.scenery_house = self.s.lv(Upg::House);
        self.inp.w2_pole = self.s.world == 10;
    }

    /// The zeno button: everything but his favour (and achievements) is erased.
    pub fn zeno_erase(&mut self) {
        if !self.zeno_ready() {
            return;
        }
        let erased = self.tl.boss.as_ref().filter(|b| !b.dead).map(|b| b.name.clone());
        if let Some(b) = self.tl.boss.as_mut().filter(|b| !b.dead) {
            b.dead = true;
            b.fled = true;
            b.t1 = self.t;
        }
        let s = &mut self.s;
        s.levels = [0; Upg::ALL.len()];
        (s.pl, s.granted, s.multi, s.multi_t, s.uni_best, s.run_best, s.seeds, s.ascend, s.form, s.wishes, s.saga) =
            (Big::ZERO, 0, false, 0.0, 0, Big::ZERO, Big::ZERO, Big::ZERO, 0, 0, 0);
        (s.uni, s.uni_stage, s.ustars, s.mastery, s.godki, s.god_total, s.gods, s.auto_at, s.orbs7, s.namek_at) = (0, 0, 0, 0, 0, 0, 0, 0, 0, Big::ZERO);
        s.tree = [0; TREE.len()];
        s.angel = [false; ANGELS.len()];
        s.trips = 0;
        s.foes.clear();
        s.zeno += 1;
        self.tl.zen = None;
        self.tl.preview = None;
        self.tl.next_boss = 60.0;
        self.ach("zeno");
        self.top_sync();
        if self.s.world != 0 {
            self.travel(0);
            self.tl.namek_until = 0.0;
        }
        if self.tl.itx_trip {
            self.itx_home();
        }
        self.flash_t = 1.0;
        self.shake = 1.0;
        self.cue(Cue::Form(self.s.form));
        self.cue(Cue::Fx("super"));
        self.cue(Cue::Sfx("boom"));
        self.scenery();
        let favour = crate::num::fmt_short(Big::from(ZENO_X).pow(self.s.zeno as f64));
        self.event_ms(
            match erased {
                Some(v) => format!("zeno: \"{v}? he's no fun.\" — {v} is erased. then so is everything else. zeno's favour ×{favour}"),
                None => format!("zeno: \"friend! let's play again!\" — the whole universe is erased. zeno's favour ×{favour}"),
            },
            6000.0,
        );
        self.after(6.2, crate::timeline::Later::Event("villains line up to meet zeno's friend — the sagas come 4× as often".into(), 4000.0));
        self.sync_shop();
    }

    // ── the fixed step

    /// One frame of game.js `frame(t, dt)`, then the `setTimeout`s that came due: `t` is the new time, `dt` the step.
    pub fn frame(&mut self, t: f64, dt: f64) {
        self.sim(t, dt);
        self.timeouts();
    }

    /// `frame(t, dt)` up to its drawing. A renderer draws between this and `timeouts()`, as game.js draws inside
    /// `frame()` before the page runs the timeouts that came due (follow-up wishes, the summon after a goal).
    pub fn sim(&mut self, t: f64, dt: f64) {
        self.t = t;
        let rdt = dt;
        let dt = if self.b.chamber_until > t && self.s.world != 3 { dt * 0.3 } else { dt };
        if self.playing {
            self.session += dt;
        }
        self.cam.bob = if self.shake > 0.0 && !self.calm { js_round((self.rng.random() - 0.5) * 5.0 * self.shake) } else { 0.0 };
        self.shake = (self.shake - dt).max(0.0);
        self.spin(dt);

        // running: sprint decays; laps pay
        self.sprint = (self.sprint - dt * 0.8).max(0.0);
        self.riding = (self.riding - dt).max(0.0);
        let snake = self.s.world == 3;
        let pace = (0.42 + self.sprint * if snake { 0.45 } else { 0.9 })
            * (1.0 + 0.1 * self.lvl(N_STRIDE))
            * if self.wished(WishBuff::Fast) { 1.5 } else { 1.0 }
            * if self.riding > 0.0 { 1.6 } else { 1.0 }
            * if self.wished(WishBuff::Fly) && !snake { 1.8 } else { 1.0 }
            * if self.b.pose_until > t || self.tl.struggle.is_some() { 0.0 } else { 1.0 };
        self.phi += pace * dt;
        if self.phi - self.last_lap > LAP {
            self.last_lap += LAP;
            if self.playing {
                self.laps += 1;
                self.s.laps_total += 1;
                self.ach("lap1");
                if self.s.laps_total >= 100 {
                    self.ach("laps100");
                }
                self.goal("laps", 1.0, false);
                self.cue(Cue::Fx("lap"));
                let n = self.lap_value()
                    * 4.0
                    * self.mult(false)
                    * (1.0 + 0.5 * self.lvl(N_LEGS))
                    * if self.wished(WishBuff::Laps) { 3.0 } else { 1.0 }
                    * if self.tl.bz.is_some() { 3.0 } else { 1.0 }
                    * if self.b.whis_until > t { 2.0 } else { 1.0 };
                let k = self.kid_screen(20.0);
                self.gain_at(n, k.x, k.y - 14.0, true);
            }
        }
        if self.tl.jump_h > 0.0 || self.tl.jump_v > 0.0 {
            self.tl.jump_v -= 520.0 * dt;
            self.tl.jump_h = (self.tl.jump_h + self.tl.jump_v * dt).max(0.0);
            if self.tl.jump_h == 0.0 {
                self.tl.jump_v = 0.0;
            }
        }
        if self.playing {
            // income: lap steps + passive, once a second
            self.acc += rdt;
            if self.acc >= 1.0 {
                self.acc -= 1.0;
                let n = (self.lap_value() / 7.0 + self.passive()) * self.mult(false);
                self.gain(n);
            }
            // the timeline: all of it waits while you're on snake way
            if self.s.world == 3 {
                self.update_snake(t, dt);
            } else {
                self.timeline_home(t, dt);
            }
            self.timeline_trips(t);
            if self.s.world != 3 {
                self.timeline_fights(t, dt);
            }
            // a wished form wears off: back to the best form your held power earns
            if let Some((from, until)) = self.tl.preview.filter(|p| t > p.1) {
                let _ = until;
                let mut f = from;
                while f < self.forms.len() - 1 && self.s.pl >= self.forms[f + 1].at {
                    f += 1;
                }
                if f < self.s.form {
                    self.s.form = f;
                    self.cue(Cue::Form(f));
                    let msg = format!("the wish wears off — back to {}", self.forms[f].name);
                    self.event(msg);
                } else {
                    let msg = format!("the wish wears off — but you're holding enough power: {} stays", self.forms[self.s.form].name);
                    self.event(msg);
                }
                self.tl.preview = None;
            }
            // a form you hold the power for comes as a scene (xform.rs) and stays for the run
            self.xform_step(t, dt);
            self.limits_step(t, dt);
            let last = self.forms.len() - 1;
            if self.s.form == last && self.tl.preview.is_none() && self.s.pl >= self.forms[last].at * Big::from(LATE_STEP).pow(self.s.mastery as f64 + 1.0) {
                self.s.mastery += 1;
                self.ach("mastery");
                let next = crate::num::fmt_short(self.forms[last].at * Big::from(LATE_STEP).pow(self.s.mastery as f64 + 1.0));
                self.event_ms(format!("mastery star {} — ×{MASTERY_X} on everything, kept through god ki. next at {next}", self.s.mastery), 3500.0);
                self.cue(Cue::Fx("star"));
            }
            if self.ang(A_AUTO)
                && self.s.auto_at > 0
                && self.s.pl >= AUTO_AT[self.s.auto_at]
                && self.tl.boss.is_none()
                && self.tl.shen.is_none()
                && self.s.world == self.home()
            {
                self.ascend();
            }
            self.timeline_after(t, dt);
            if t - self.last_click > 0.9 {
                self.combo = 1.0;
            }
            self.timeline_moon(t, dt);
            self.hud = self.hud_now();
            self.sync_shop();
        }
        self.shen_step(t);
        self.draw_step(t, dt);
    }

    /// The `setTimeout`s due by now.
    pub fn timeouts(&mut self) {
        self.run_later(self.t);
    }

    /// The timeouts due by `t`, run while the game's clock still reads the last frame's time, as a page's setTimeouts
    /// fire between animation frames.
    pub fn timeouts_by(&mut self, t: f64) {
        self.run_later(t);
    }

    /// Test hook: sets one save field by its game.js name, the way the JS harness pokes `S` (`world` also lays it out).
    pub fn debug_set(&mut self, key: &str, v: f64) {
        let s = &mut self.s;
        match key {
            "pl" => s.pl = v.into(),
            "best" => s.best = v.into(),
            "wishes" => s.wishes = v as u32,
            "ascend" => s.ascend = v.into(),
            "seeds" => s.seeds = v.into(),
            "multi" => s.multi = v != 0.0,
            "multiT" => s.multi_t = v,
            "world" => s.world = v as u32,
            "uniBest" => s.uni_best = v as u32,
            "ustars" => s.ustars = v as u32,
            "godTotal" => s.god_total = v as u32,
            "zeno" => s.zeno = v as u32,
            "mastery" => s.mastery = v as u32,
            "autoAt" => s.auto_at = v as usize,
            "gods" => s.gods = v as u32,
            "uni" => s.uni = v as u32,
            "krillinDead" => s.krillin_dead = v != 0.0,
            "orbs7" => s.orbs7 = v as u32,
            "trips" => s.trips = v as u32,
            "form" => s.form = v as usize,
            _ => {
                if let Some(a) = key.strip_prefix("angel.").and_then(angel_ix) {
                    s.angel[a] = v != 0.0;
                } else if let Some(u) = Upg::from_key(key) {
                    s.levels[u as usize] = v as u32;
                } else {
                    panic!("debug_set: unknown key {key}");
                }
            }
        }
        // a poked power lands the forms it holds at once, as a preview hook's does: it sets a scene up, it isn't a form-up
        if self.xf.is_none() && self.form_owed() {
            self.xf_instant = true;
        }
        if key == "world" {
            // as the harness's poke does: the world's layout and scenery, without the trip
            self.cam.layout(self.s.world);
            self.scenery();
        }
    }
}

impl Game {
    /// game.js's preview hooks (`#raid`, `#namek`, `#form12`, `#hero-goku`, ...), applied as `mount()` applies them,
    /// after the world is set up and before `start()`. Of those that wait on a `setTimeout`, `#shenron` is ported;
    /// `#flight`, `#chamber` and `#cvlink` aren't.
    pub fn preview(&mut self, hash: &str) {
        use crate::data::{Hero, Upg, LADDER, MUI_FORM, VILLAINS};
        let has = |k: &str| hash.contains(k);
        let num = |k: &str| -> Option<usize> {
            let i = hash.find(k)? + k.len();
            let d: String = hash[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
            d.parse().ok()
        };
        let best = |g: &mut Game, b: f64| g.s.best = g.s.best.max(b);
        let lv = |g: &mut Game, u: Upg, n: u32| g.s.levels[u as usize] = g.s.levels[u as usize].max(n);
        let trip = |g: &mut Game, b: f64, trips: u32| {
            g.s.best = g.s.best.max(b);
            g.s.namek_at = b.into();
            g.s.trips = trips;
        };
        let sagas = VILLAINS.len() as u32;
        if has("ape") {
            self.session = 41.0;
            self.tl.next_moon = 0.0;
        }
        if has("ozaru") {
            self.session = 41.0;
            self.tl.next_moon = 0.0;
            self.s.ach.insert("ape");
            best(self, 5e4);
            lv(self, Upg::Tail, 3);
            if has("giant") {
                self.s.big_ape = true;
            }
        }
        if has("ginyu") {
            self.session = 151.0;
            best(self, 5e4);
            self.tl.next_ginyu = 0.0;
        }
        if has("visitors") {
            self.session = 61.0;
            self.tl.next_greg = 0.0;
            self.tl.next_kai = 0.0;
        }
        if has("raid") {
            self.session = 111.0;
            self.tl.next_boss = 0.0;
            self.tl.next_saiba = 0.0;
        }
        if let Some(n) = num("saga") {
            self.s.saga = (n as i64 - 1).max(0) as u32;
            self.session = 111.0;
            self.tl.next_boss = 0.0;
        }
        if has("roshi") {
            self.session = 91.0;
            self.tl.next_roshi = 0.0;
        }
        for (k, b, trips) in
            [("paozu", 7e6, 6), ("westcity", 5e6, 4), ("kame", 6e6, 5), ("lookout", 5e6, 8), ("kaiworld", 5e6, 9), ("yardrat", 5e6, 10), ("future", 5e6, 7)]
        {
            if has(k) {
                trip(self, b, trips);
            }
        }
        if has("androids") {
            self.session = 201.0;
            best(self, 2e5);
            self.tl.next_andro = 0.0;
        }
        if has("date") {
            self.session = 161.0;
            self.tl.next_date = 0.0;
        }
        if has("gero") {
            self.session = 141.0;
            self.tl.next_gero = 0.0;
        }
        if has("sixteen") {
            self.session = 101.0;
            self.tl.next_a16 = 0.0;
        }
        if has("beerus") {
            trip(self, 4e6, 3);
        }
        if has("krillin") {
            self.session = 131.0;
            self.tl.next_kril = 0.0;
        }
        if has("seeds") {
            self.s.seeds = 40.0.into();
        }
        if has("galick") {
            lv(self, Upg::Galick, if has("galick5") { 5 } else { 1 });
        }
        if let Some(n) = num("universe") {
            self.s.saga = sagas;
            self.s.multi = true;
            self.s.uni = LADDER.iter().position(|l| l.0 as usize == n).unwrap_or(0) as u32;
            self.s.uni_stage = 2;
            self.s.ustars = self.s.uni;
            self.session = 111.0;
            self.tl.next_boss = 0.0;
        }
        if has("mui") {
            self.s.pl = 2.1e14.into();
            self.s.best = 2.1e14.into();
            self.s.form = MUI_FORM - 1;
        }
        if let Some(n) = num("form") {
            let i = n.min(self.forms.len() - 1);
            self.s.pl = (self.forms[i].at * 1.05).into();
            self.s.best = self.s.pl;
            self.s.form = i.max(1) - 1;
        }
        if has("godki") {
            self.s.pl = 2.1e15.into();
            self.s.best = 2.1e15.into();
            self.s.form = MUI_FORM;
            self.s.godki = 6;
            self.s.god_total = 3;
            self.s.gods = 1;
            self.s.multi = true;
            self.s.saga = sagas;
            self.s.uni = 2;
            self.s.ustars = 2;
        }
        if has("zeno") {
            self.s.gods = 2;
            self.s.saga = sagas;
            self.s.multi = true;
            self.session = 111.0;
        }
        if has("fusion") {
            lv(self, Upg::Partner, 3);
            best(self, 5e4);
            self.tl.next_fuse = 2.0;
        }
        if has("namek") || has("porunga") {
            trip(self, 1e6, 0);
        }
        if has("foes") {
            self.s.foes = ["raditz", "cell", "broly", "goku black"].map(String::from).to_vec();
        }
        if has("kaio") {
            lv(self, Upg::Kaio, 1);
        }
        if has("vegeta") {
            trip(self, 2e6, 1);
        }
        if let Some(i) = hash.find("hero-") {
            let id: String = hash[i + 5..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if let Some(h) = [("vegeta", Hero::Vegeta), ("goku", Hero::Goku), ("gohan", Hero::Gohan), ("trunks", Hero::Trunks)].iter().find(|h| h.0 == id) {
                self.s.hero = h.1;
            }
        }
        if has("snake") {
            trip(self, 3e6, 2);
            self.snake_way = true;
        }
        self.snake_nimbus = has("nimbus");
        if has("door") {
            lv(self, Upg::Chamber, 1);
            self.tl.next_door = 0.0;
        }
        if has("car") {
            self.session = 71.0;
            self.tl.next_car = 0.0;
        }
        if has("pod") {
            self.session = 13.0;
            self.tl.next_star = 0.0;
        }
        if has("storm") {
            self.session = 121.0;
            self.tl.next_storm = 0.0;
        }
        if has("spirit") {
            self.s.spirit = self.s.spirit.max(1);
            self.tl.spirit = 0.97;
        }
        if has("struggle") {
            self.session = 111.0;
            self.tl.next_boss = 0.0;
            lv(self, Upg::Galick, 1);
            self.tl.struggle_force = true;
        }
        if has("frieza") {
            self.session = 151.0;
            self.tl.next_ship = 0.0;
        }
        if let Some(m) = ["cooler", "janemba", "bojack"].into_iter().find(|m| has(m)) {
            self.tl.movie_force = m;
            self.session = 241.0;
            self.tl.home_since = self.session;
            best(self, 3e5);
            self.tl.next_movie = 0.0;
        }
        if has("yamcha") {
            self.session = 101.0;
            self.tl.next_yam = 0.0;
        }
        if has("cling") {
            self.session = 151.0;
            self.tl.next_cling = 0.0;
        }
        if has("dance") {
            self.session = 201.0;
            self.tl.next_dance = 0.0;
            best(self, 2e5);
        }
        if has("gregory") {
            self.session = 61.0;
            self.tl.next_greg = 0.0;
        }
        if has("#top") {
            self.s.world = 9;
            self.cam.layout(9);
            self.tl.namek_until = 1e9;
        }
        if has("buffs") {
            // `setTimeout(() => { …a long buff line… }, 800)`
            self.tl.later.push((0.8, crate::timeline::Later::Buffs));
        }
        if let Some(i) = hash.find("shenron") {
            // `setTimeout(() => summon(look), 800)`
            if let Some(d) = hash[i + 7..].chars().next().and_then(|c| c.to_digit(10)) {
                self.tl.shen_look = d as usize;
            }
            self.tl.later.push((0.8, crate::timeline::Later::Summon));
        }
        if has("itx") {
            // and opens the picker 1.5s in, or once playing: `setTimeout(open, 1500)`
            lv(self, Upg::Itx, 3);
            self.tl.later.push((1.5, crate::timeline::Later::ItxPicker));
            self.s.ach.insert("yardrat");
            self.s.been = crate::data::ITX_WORLDS.iter().fold(0, |m, w| m | 1 << w);
        }
        if has("tournament") {
            self.session = 76.0;
            best(self, 3000.0);
            self.tl.next_tour = 0.0;
        }
        self.limits_preview(hash);
        self.looks_preview(hash);
        self.waves_preview(hash);
        self.forms = ladder_of(self.s.hero, self.s.limits);
        self.s.form = self.s.form.min(self.forms.len() - 1);
        if has("xform") {
            // the transformation scene from the first frame: just past super saiyan's mark and the scouter's
            self.s.form = 0;
            self.s.pl = Big::from(self.forms[1].at * 1.02);
            self.s.best = self.s.pl;
            self.s.o9k = true;
            self.s.ach.insert("o9k");
            // and super saiyan's achievement, whose toast would follow the form's own
            self.s.ach.insert("ss");
        }
        // a hook that names a form shows it at once, as it did before forms came as a scene
        self.xf_instant = !has("xform") && self.form_owed();
        if self.s.multi && self.s.world == 0 {
            self.s.world = 9;
            self.cam.layout(9);
        }
        if self.s.multi {
            self.top_sync();
        }
        self.s.run_best = self.s.run_best.max(self.s.best);
    }
}

impl Game {
    /// The canvas the game draws on, in its big pixels (`st.W`×`st.H`), and whether it's the desktop widget.
    pub fn set_view(&mut self, w: f64, h: f64, widget: bool) {
        self.cam.w = w;
        self.cam.h = h;
        self.cam.widget = widget;
        self.cam.layout(self.s.world);
    }

    /// `layout()` for a new canvas: `w`×`h` buffer pixels at `scale` CSS pixels each.
    pub fn resize(&mut self, w: f64, h: f64, scale: f64) {
        (self.cam.w, self.cam.h, self.cam.scale) = (w, h, scale);
        self.cam.layout(self.s.world);
    }
}
