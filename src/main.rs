//! typo — terminal touch-typing tutor. Part of the Fe2O3 suite.
//!
//! Strict tutor: the drill only advances on the correct key; wrong keys
//! count as errors. Everything is event-driven — no timers, no polling,
//! zero idle cost. WPM, accuracy and rhythm are recomputed on each
//! keypress, and per-key timing is plain bookkeeping in the same handler.
//!
//! What the research asks for, and where it lives here:
//!   * gradual key unlocking, words built from unlocked keys → `gradual_lines`
//!   * per-key and per-pair timing, weakest fed back → `Stats::weak`, `weak_lines`
//!   * accuracy before speed → `Stats::gate`, which hides wpm until it passes
//!   * even rhythm as one number → `evenness`
//!   * eyes ahead of the fingers → the next word is emphasised, never the current
//!   * a fixed daily test with a trend → `daily_lines`, `trend_text`

use crust::style;
use crust::{Crust, Input, Pane};
use std::collections::HashMap;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const DONE_FG: u8 = 71; // typed chars: green
const TODO_FG: u8 = 246; // untyped chars: gray
const NEXT_FG: u8 = 253; // the word after the cursor: bright, to pull the eyes on
const ERR_FG: u8 = 196; // current char after a miss: red
const SEL_BG: u8 = 81; // menu selection bar

/// Accuracy a round must reach to count as clean.
const CLEAN_ACC: f64 = 97.0;
/// Clean rounds in a row before speed is worth talking about, and before
/// gradual mode hands out another key.
const CLEAN_ROUNDS: u32 = 3;
/// Keys unlocked when gradual mode starts: f j d k.
const START_KEYS: usize = 4;
/// A gap longer than this is the typist thinking or being interrupted, not
/// a keystroke interval. Left out of every timing figure.
const MAX_GAP_MS: f64 = 2000.0;
/// Times a key must have been seen before its average means anything.
const MIN_SAMPLES: u32 = 5;

struct Lesson {
    name: &'static str,
    lines: &'static [&'static str],
}

struct Layout {
    code: &'static str,
    label: &'static str,
    lessons: &'static [Lesson],
    /// Order gradual mode hands out keys in: home row, then the index
    /// reaches, then top row, then bottom row.
    unlock: &'static str,
    /// The daily test. Same text every time, so the trend means something.
    daily: &'static [&'static str],
}

const LAYOUTS: &[Layout] = &[
    Layout {
        code: "us",
        label: "US",
        lessons: LESSONS_US,
        unlock: "fjdksla;gheirutyowpqvnmcxzb,.",
        daily: DAILY_US,
    },
    Layout {
        code: "no",
        label: "Norwegian",
        lessons: LESSONS_NO,
        unlock: "fjdkslaøægheirutyowpqåvnmcxzb,.",
        daily: DAILY_NO,
    },
];

const DAILY_US: &[&str] = &[
    "the quick brown fox jumps over the lazy dog",
    "pack my box with five dozen liquor jugs",
    "how vexingly quick daft zebras jump",
    "sphinx of black quartz, judge my vow",
];

const DAILY_NO: &[&str] = &[
    "høvdingens kjære squaw får litt pizza i mexico by",
    "en rask brun rev hopper over den late hunden",
    "syv sære menn ba om whisky og quiz i taxi",
    "blåbærsyltetøy er godt på vafler og brød",
];

const LESSONS_US: &[Lesson] = &[
    Lesson {
        name: "Home row",
        lines: &[
            "fff jjj fjf jfj fj fj jf jf",
            "ddd kkk dkd kdk dk dk kd kd",
            "sss lll sls lsl sl sl ls ls",
            "aaa ;;; a;a ;a; a; a; ;a ;a",
            "asdf jkl; asdf jkl; fdsa ;lkj",
        ],
    },
    Lesson {
        name: "Home row words (g h)",
        lines: &[
            "ggg hhh ghg hgh fg fg jh jh",
            "as all ask add lad sad fad fall lass",
            "gas has had hall glad flag flash half",
            "a sad lad; a glad lass; all halls had flags",
        ],
    },
    Lesson {
        name: "Top row",
        lines: &[
            "rrr uuu rur uru fr fr ju ju",
            "eee iii eie iei de de ki ki",
            "ttt yyy tyt yty ft ft jy jy",
            "www ooo wow owo sw sw lo lo",
            "qqq ppp qpq pqp aq aq ;p ;p",
            "we try to type quiet words; your pretty eyes",
        ],
    },
    Lesson {
        name: "Bottom row",
        lines: &[
            "nnn mmm nmn mnm jn jn jm jm",
            "vvv bbb vbv bvb fv fv fb fb",
            "ccc xxx cxc xcx dc dc sx sx",
            "zzz ,,, z,z ,z, az az k, k,",
            "man cave; zinc box. seven brave men climb back.",
        ],
    },
    Lesson {
        name: "Capitals",
        lines: &[
            "Ask Sad Lad Fall Glad Hash Dash",
            "Anna Bob Carl Dora Erik Faye Gus Hans Ivan",
            "The Lass Has A Flag. He Had Half A Glass.",
            "Type Each First Letter With The Far Shift Key.",
        ],
    },
    Lesson {
        name: "Numbers",
        lines: &[
            "111 222 333 444 555 666 777 888 999 000",
            "12 34 56 78 90 09 87 65 43 21",
            "a1 s2 d3 f4 g5 h6 j7 k8 l9 ;0",
            "1990 2026 365 24 60 100 1000 42",
        ],
    },
    Lesson {
        name: "Symbols",
        lines: &[
            "!!! @@@ ### $$$ %%% ^^^ &&& *** ((( )))",
            "a! s@ d# f$ g% h^ j& k* l( ;)",
            "- = _ + [ ] { } ' \" < > ? /",
            "(one) [two] {three} \"four\" 'five' six-seven",
            "email@example.com 100% #1 $50 *star* a_b",
        ],
    },
    Lesson {
        name: "Sentences",
        lines: &[
            "the quick brown fox jumps over the lazy dog",
            "pack my box with five dozen liquor jugs",
            "how vexingly quick daft zebras jump",
            "sphinx of black quartz, judge my vow",
            "The five boxing wizards jump quickly.",
        ],
    },
];

const LESSONS_NO: &[Lesson] = &[
    Lesson {
        name: "Home row",
        lines: &[
            "fff jjj fjf jfj fj fj jf jf",
            "ddd kkk dkd kdk dk dk kd kd",
            "sss lll sls lsl sl sl ls ls",
            "aaa øøø aøa øaø aø aø øa øa",
            "æææ læl ælæ æl æl læ læ",
            "asdf jkløæ asdf jkløæ fdsa æølkj",
        ],
    },
    Lesson {
        name: "Home row words (g h)",
        lines: &[
            "ggg hhh ghg hgh fg fg jh jh",
            "ask aks als dal gal lag sag salg",
            "glad kald hall fall fjas skal",
            "øl gløgg løk øks sjø søl æsj høl",
            "all fjas skal ha kald gløgg",
        ],
    },
    Lesson {
        name: "Top row",
        lines: &[
            "rrr uuu rur uru fr fr ju ju",
            "eee iii eie iei de de ki ki",
            "ttt yyy tyt yty ft ft jy jy",
            "www ooo wow owo sw sw lo lo",
            "qqq ppp qpq pqp aq aq øp øp",
            "ååå påå åpå på på gå gå",
            "du eter søt kake på tur ut i dag",
        ],
    },
    Lesson {
        name: "Bottom row",
        lines: &[
            "nnn mmm nmn mnm jn jn jm jm",
            "vvv bbb vbv bvb fv fv fb fb",
            "ccc xxx cxc xcx dc dc sx sx",
            "zzz ,,, z,z ,z, az az k, k,",
            "--- ... -.- .-. l. l. ø- ø-",
            "mannen kom med en varm boks til byen, og dro.",
        ],
    },
    Lesson {
        name: "Capitals",
        lines: &[
            "Ask Sal Dag Lag Gal Hal Jag Kald",
            "Anna Bjørn Cato Dina Erik Frode Gro Hans",
            "Åse Øystein Ære Ås Øst Ærlig",
            "Han Har En Hund. Hun Har En Katt.",
            "Bruk Alltid Motsatt Skift For Stor Bokstav.",
        ],
    },
    Lesson {
        name: "Numbers",
        lines: &[
            "111 222 333 444 555 666 777 888 999 000",
            "12 34 56 78 90 09 87 65 43 21",
            "a1 s2 d3 f4 g5 h6 j7 k8 l9 ø0",
            "1990 2026 365 24 60 100 1000 42",
        ],
    },
    Lesson {
        name: "Symbols",
        lines: &[
            "!!! \"\"\" ### ¤¤¤ %%% &&& /// ((( ))) ===",
            "a! s\" d# f¤ g% h& j/ k( l) ø=",
            "+++ ??? ''' *** ;;; ::: ___",
            "(en) \"to\" 'tre' fire-fem 50% og/eller",
            "post@eksempel.no [x] {y} $50 100%",
        ],
    },
    // The right little finger already has æ ø å from lesson 1. What the
    // Norwegian layout still hides is everything behind AltGr, which is
    // where most of programming and every email address lives.
    Lesson {
        name: "AltGr (@ $ { [ ] } \\ |)",
        lines: &[
            "@@@ $$$ {{{ [[[ ]]] }}} \\\\\\ |||",
            "a@ s$ d{ f[ g] h} j\\ k|",
            "post@eksempel.no  100$  50%  a|b  c\\d",
            "if (liste[0] == 1) { svar = a | b; }",
            "C:\\bruker\\geir  {æ, ø, å}  [1, 2, 3]",
        ],
    },
    Lesson {
        name: "Sentences",
        lines: &[
            "høvdingens kjære squaw får litt pizza i mexico by",
            "en rask brun rev hopper over den late hunden",
            "syv sære menn ba om whisky og quiz i taxi",
            "Blåbærsyltetøy er godt på vafler og brød.",
        ],
    },
];

// ---------- small helpers with no crates behind them ----------

/// xorshift64. Seeded once per launch; drills need variety, not entropy.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(n | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

/// Today as YYYY-MM-DD, UTC. The daily test only needs a boundary that
/// never moves; UTC gives one without reading a timezone database.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let y = 1970 + (days * 4 + 2) / 1461;
    let doy = days - (365 * (y - 1970) + (y - 1969) / 4);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 0usize;
    let mut d = doy;
    for &md in &month_days {
        if d < md { break; }
        d -= md;
        mo += 1;
    }
    format!("{:04}-{:02}-{:02}", y, mo + 1, d + 1)
}

/// One number for rhythm: how even the gaps between keystrokes were.
/// 100 is a metronome, 0 is all bursts and stalls.
fn evenness(iv: &[f64]) -> f64 {
    if iv.len() < 3 { return 0.0; }
    let n = iv.len() as f64;
    let mean = iv.iter().sum::<f64>() / n;
    if mean <= 0.0 { return 0.0; }
    let var = iv.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n;
    let cv = var.sqrt() / mean;
    (100.0 * (1.0 - cv)).clamp(0.0, 100.0)
}

/// Spaces have to be visible when a key name is printed on its own.
fn key_label(c: char) -> String {
    match c {
        ' ' => "␣".to_string(),
        '\n' => "⏎".to_string(),
        _ => c.to_string(),
    }
}

// ---------- state (~/.typo) ----------

/// Everything kept between runs. One tab-separated record per line, tagged
/// by its first field, so old files load and new fields can be added
/// without a migration.
struct Stats {
    layout: usize,
    /// "<code>:<lesson>" → (best wpm, accuracy at that best)
    best: HashMap<String, (f64, f64)>,
    /// Per layout: how many keys gradual mode has handed out.
    unlocked: Vec<usize>,
    /// Per layout: clean rounds in a row. Gates the speed display.
    clean: Vec<u32>,
    /// Per layout: clean gradual rounds in a row. Gates the next key.
    gclean: Vec<u32>,
    /// Per layout: char → (total ms between it and the key before, times seen)
    keys: Vec<HashMap<char, (f64, u32)>>,
    /// Per layout: two chars → (total ms across the pair, times seen)
    pairs: Vec<HashMap<String, (f64, u32)>>,
    /// (layout, date, wpm, accuracy, rhythm), one per daily test run.
    tests: Vec<(usize, String, f64, f64, f64)>,
}

impl Stats {
    fn new() -> Self {
        let n = LAYOUTS.len();
        Stats {
            layout: 0,
            best: HashMap::new(),
            unlocked: vec![START_KEYS; n],
            clean: vec![0; n],
            gclean: vec![0; n],
            keys: vec![HashMap::new(); n],
            pairs: vec![HashMap::new(); n],
            tests: Vec::new(),
        }
    }

    /// Has this layout earned the right to be told about speed?
    fn gate(&self, lay: usize) -> bool {
        self.clean[lay] >= CLEAN_ROUNDS
    }

    /// Slowest keys and slowest pairs, worst first. Only what has been seen
    /// often enough for the average to be worth acting on.
    fn weak(&self, lay: usize) -> (Vec<(char, f64)>, Vec<(String, f64)>) {
        let mut k: Vec<(char, f64)> = self.keys[lay]
            .iter()
            .filter(|(_, (_, n))| *n >= MIN_SAMPLES)
            .map(|(c, (t, n))| (*c, t / *n as f64))
            .collect();
        k.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut p: Vec<(String, f64)> = self.pairs[lay]
            .iter()
            .filter(|(_, (_, n))| *n >= MIN_SAMPLES)
            .map(|(s, (t, n))| (s.clone(), t / *n as f64))
            .collect();
        p.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        (k, p)
    }

    fn unlock_chars(&self, lay: usize) -> Vec<char> {
        LAYOUTS[lay].unlock.chars().take(self.unlocked[lay]).collect()
    }

    fn all_unlocked(&self, lay: usize) -> bool {
        self.unlocked[lay] >= LAYOUTS[lay].unlock.chars().count()
    }
}

fn stats_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".typo")
}

fn best_key(lay: usize, lesson: &Lesson) -> String {
    format!("{}:{}", LAYOUTS[lay].code, lesson.name)
}

fn layout_of(code: &str) -> Option<usize> {
    LAYOUTS.iter().position(|l| l.code == code)
}

fn load_state() -> Stats {
    let mut st = Stats::new();
    let Ok(data) = std::fs::read_to_string(stats_path()) else { return st };
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["layout", code, ..] => {
                if let Some(i) = layout_of(code) { st.layout = i; }
            }
            ["unlocked", code, n, ..] => {
                if let (Some(i), Ok(n)) = (layout_of(code), n.parse::<usize>()) {
                    st.unlocked[i] = n.max(START_KEYS);
                }
            }
            ["clean", code, n, ..] => {
                if let (Some(i), Ok(n)) = (layout_of(code), n.parse()) { st.clean[i] = n; }
            }
            ["gclean", code, n, ..] => {
                if let (Some(i), Ok(n)) = (layout_of(code), n.parse()) { st.gclean[i] = n; }
            }
            ["key", code, c, total, n, ..] => {
                if let (Some(i), Some(c), Ok(t), Ok(n)) =
                    (layout_of(code), c.chars().next(), total.parse(), n.parse())
                {
                    st.keys[i].insert(c, (t, n));
                }
            }
            ["pair", code, p, total, n, ..] => {
                if let (Some(i), Ok(t), Ok(n)) = (layout_of(code), total.parse(), n.parse()) {
                    st.pairs[i].insert(p.to_string(), (t, n));
                }
            }
            ["test", code, date, w, a, e, ..] => {
                if let (Some(i), Ok(w), Ok(a), Ok(e)) =
                    (layout_of(code), w.parse(), a.parse(), e.parse())
                {
                    st.tests.push((i, date.to_string(), w, a, e));
                }
            }
            // Personal bests, the original format: "<code>:<lesson>".
            [name, w, a, ..] if name.contains(':') => {
                if let (Ok(w), Ok(a)) = (w.parse(), a.parse()) {
                    st.best.insert(name.to_string(), (w, a));
                }
            }
            _ => {}
        }
    }
    st
}

fn save_state(st: &Stats) {
    let mut out = format!("layout\t{}\n", LAYOUTS[st.layout].code);
    for (li, layout) in LAYOUTS.iter().enumerate() {
        let code = layout.code;
        out.push_str(&format!("unlocked\t{}\t{}\n", code, st.unlocked[li]));
        out.push_str(&format!("clean\t{}\t{}\n", code, st.clean[li]));
        out.push_str(&format!("gclean\t{}\t{}\n", code, st.gclean[li]));
        for lesson in layout.lessons {
            let key = best_key(li, lesson);
            if let Some((w, a)) = st.best.get(&key) {
                out.push_str(&format!("{}\t{:.1}\t{:.1}\n", key, w, a));
            }
        }
        let mut ks: Vec<_> = st.keys[li].iter().collect();
        ks.sort_by_key(|(c, _)| **c);
        for (c, (t, n)) in ks {
            out.push_str(&format!("key\t{}\t{}\t{:.1}\t{}\n", code, c, t, n));
        }
        let mut ps: Vec<_> = st.pairs[li].iter().collect();
        ps.sort_by(|a, b| a.0.cmp(b.0));
        for (p, (t, n)) in ps {
            out.push_str(&format!("pair\t{}\t{}\t{:.1}\t{}\n", code, p, t, n));
        }
    }
    for (li, date, w, a, e) in &st.tests {
        out.push_str(&format!(
            "test\t{}\t{}\t{:.1}\t{:.1}\t{:.1}\n",
            LAYOUTS[*li].code, date, w, a, e
        ));
    }
    let _ = std::fs::write(stats_path(), out);
}

// ---------- drill text generation ----------

/// Every word the layout's own lessons use. No second word list to keep in
/// step with the layout, and everything in it is known to be typeable.
fn word_pool(lay: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for lesson in LAYOUTS[lay].lessons {
        for line in lesson.lines {
            for raw in line.split_whitespace() {
                let w: String = raw
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .flat_map(|c| c.to_lowercase())
                    .collect();
                if w.chars().count() >= 2 && !out.contains(&w) {
                    out.push(w);
                }
            }
        }
    }
    out
}

/// Drill lines for gradual mode: real words when the unlocked keys spell
/// enough of them, made-up ones when they don't. Either way the newest key
/// shows up in about half the words, because that is the one being learnt.
fn gradual_lines(lay: usize, st: &Stats, rng: &mut Rng) -> Vec<String> {
    let letters = st.unlock_chars(lay);
    let newest = *letters.last().unwrap_or(&'f');
    let pool: Vec<String> = word_pool(lay)
        .into_iter()
        .filter(|w| w.chars().all(|c| letters.contains(&c)))
        .collect();
    let with_new: Vec<&String> = pool.iter().filter(|w| w.contains(newest)).collect();

    let mut lines = Vec::new();
    for _ in 0..5 {
        let mut words: Vec<String> = Vec::new();
        for i in 0..8 {
            // Real words need a pool worth drawing from; below that the
            // letters simply don't spell anything.
            if pool.len() >= 12 {
                let want_new = i % 2 == 0 && !with_new.is_empty();
                let w = if want_new {
                    with_new[rng.below(with_new.len())].clone()
                } else {
                    pool[rng.below(pool.len())].clone()
                };
                words.push(w);
            } else {
                let len = 2 + rng.below(3);
                let mut w = String::new();
                for j in 0..len {
                    if j == 0 && i % 2 == 0 {
                        w.push(newest);
                    } else {
                        w.push(letters[rng.below(letters.len())]);
                    }
                }
                words.push(w);
            }
        }
        lines.push(words.join(" "));
    }
    lines
}

/// Drill lines built from what the typist is slowest at: three keys and two
/// pairs, then words carrying them so the practice isn't only drum patterns.
fn weak_lines(lay: usize, st: &Stats, rng: &mut Rng) -> Option<Vec<String>> {
    let (keys, pairs) = st.weak(lay);
    if keys.len() < 3 { return None; }
    let k: Vec<char> = keys.iter().take(3).map(|(c, _)| *c).collect();
    let p: Vec<String> = pairs.iter().take(2).map(|(s, _)| s.clone()).collect();

    let mut lines = Vec::new();
    let trip = |c: char| format!("{}{}{}", c, c, c);
    lines.push(format!(
        "{} {} {} {} {} {}",
        trip(k[0]), trip(k[1]), trip(k[2]), trip(k[0]), trip(k[1]), trip(k[2])
    ));
    lines.push(format!("{}{} {}{} {}{} {}{}", k[0], k[1], k[1], k[2], k[2], k[0], k[0], k[2]));
    if !p.is_empty() {
        let mut row = String::new();
        for s in &p {
            for _ in 0..3 {
                row.push_str(s);
                row.push(' ');
            }
        }
        lines.push(row.trim_end().to_string());
    }
    // Words that actually contain the weak keys, if the layout has any.
    let pool = word_pool(lay);
    let hits: Vec<&String> = pool.iter().filter(|w| k.iter().any(|c| w.contains(*c))).collect();
    for _ in 0..2 {
        if hits.len() >= 8 {
            let mut words = Vec::new();
            for _ in 0..8 {
                words.push(hits[rng.below(hits.len())].clone());
            }
            lines.push(words.join(" "));
        } else {
            lines.push(format!(
                "{}{}{} {}{}{} {}{}{}",
                k[0], k[1], k[2], k[1], k[2], k[0], k[2], k[0], k[1]
            ));
        }
    }
    Some(lines)
}

fn daily_lines(lay: usize) -> Vec<String> {
    LAYOUTS[lay].daily.iter().map(|s| s.to_string()).collect()
}

// ---------- UI ----------

struct Ui {
    header: Pane,
    main: Pane,
    footer: Pane,
}

fn layout_ui() -> Ui {
    let (cols, rows) = Crust::terminal_size();
    let mut header = Pane::new(1, 1, cols, 1, 255, 236);
    header.scroll = false;
    let mut main = Pane::new(3, 3, cols.saturating_sub(4), rows.saturating_sub(4), 252, 0);
    main.scroll = false;
    let mut footer = Pane::new(1, rows, cols, 1, 248, 236);
    footer.scroll = false;
    Ui { header, main, footer }
}

fn fresh_ui() -> Ui {
    Crust::clear_screen();
    layout_ui()
}

fn draw(ui: &mut Ui, header: &str, main: &str, footer: &str) {
    ui.header.set_text(header);
    ui.header.refresh();
    ui.main.set_text(main);
    ui.main.refresh();
    ui.footer.set_text(footer);
    ui.footer.refresh();
}

// ---------- menu ----------

fn menu_text(st: &Stats, sel: usize) -> String {
    let lay = st.layout;
    let mut out = String::from("\n");
    out.push_str(&format!(
        "{}   (layout: {})\n\n",
        style::bold("Select a lesson"),
        LAYOUTS[lay].label
    ));
    for (i, l) in LAYOUTS[lay].lessons.iter().enumerate() {
        let score = match st.best.get(&best_key(lay, l)) {
            Some((w, a)) => format!("best {:>3.0} wpm  {:>3.0}%", w, a),
            None => String::new(),
        };
        let row = format!(" {}  {:<24} {}", i + 1, l.name, score);
        if i == sel {
            out.push_str(&style::fb(&row, 232, SEL_BG));
        } else {
            out.push_str(&row);
        }
        out.push('\n');
    }

    let (keys, _) = st.weak(lay);
    let slowest = if keys.is_empty() {
        "type a little first".to_string()
    } else {
        keys.iter()
            .take(3)
            .map(|(c, _)| key_label(*c))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let today_test = st
        .tests
        .iter()
        .filter(|(l, d, _, _, _)| *l == lay && *d == today())
        .map(|(_, _, w, _, _)| format!("today {:.0} wpm", w))
        .last()
        .unwrap_or_else(|| "not taken today".to_string());

    out.push_str("\n");
    out.push_str(&format!(
        " g  {:<24} {} of {} keys, {}/{} clean\n",
        "Gradual mode",
        st.unlocked[lay],
        LAYOUTS[lay].unlock.chars().count(),
        st.gclean[lay],
        CLEAN_ROUNDS
    ));
    out.push_str(&format!(" w  {:<24} slowest: {}\n", "Weak keys", slowest));
    out.push_str(&format!(" t  {:<24} {}\n", "Daily test", today_test));
    out.push_str(&format!(
        " T  {:<24} {} runs logged\n",
        "Trend",
        st.tests.iter().filter(|(l, ..)| *l == lay).count()
    ));

    if !st.gate(lay) {
        out.push_str(&format!(
            "\n {}\n",
            style::fg(
                &format!(
                    "Accuracy first: {} of {} rounds above {:.0}%. Speed is hidden until then.",
                    st.clean[lay], CLEAN_ROUNDS, CLEAN_ACC
                ),
                220
            )
        ));
    }
    out
}

// ---------- drill ----------

enum CellState {
    Done,
    Current,
    Todo,
}

fn pos_state(li: usize, ci: usize, cli: usize, cci: usize) -> CellState {
    if li < cli || (li == cli && ci < cci) {
        CellState::Done
    } else if li == cli && ci == cci {
        CellState::Current
    } else {
        CellState::Todo
    }
}

fn push_cell(out: &mut String, ch: char, st: CellState, err: bool, emph: bool) {
    let s = ch.to_string();
    match st {
        CellState::Done => out.push_str(&style::fg(&s, DONE_FG)),
        CellState::Todo => {
            if emph {
                out.push_str(&style::styled(&s, Some(NEXT_FG), None, "b"));
            } else {
                out.push_str(&style::fg(&s, TODO_FG));
            }
        }
        CellState::Current => {
            if err {
                out.push_str(&style::styled(&s, Some(ERR_FG), None, "r"));
            } else {
                out.push_str(&style::reverse(&s));
            }
        }
    }
}

/// Where the word after the cursor starts and ends on the current line.
/// Eyes belong a word ahead of the fingers, so that word is the one that
/// gets emphasised — never the one being typed.
fn next_word_span(line: &[char], cci: usize) -> Option<(usize, usize)> {
    let mut i = cci;
    while i < line.len() && line[i] != ' ' { i += 1; }
    while i < line.len() && line[i] == ' ' { i += 1; }
    if i >= line.len() { return None; }
    let start = i;
    while i < line.len() && line[i] != ' ' { i += 1; }
    Some((start, i))
}

fn drill_text(lines: &[Vec<char>], cli: usize, cci: usize, err: bool) -> String {
    let span = lines.get(cli).and_then(|l| next_word_span(l, cci));
    let mut out = String::from("\n");
    for (li, line) in lines.iter().enumerate() {
        for (ci, &ch) in line.iter().enumerate() {
            let emph = li == cli && span.map(|(a, b)| ci >= a && ci < b).unwrap_or(false);
            push_cell(&mut out, ch, pos_state(li, ci, cli, cci), err, emph);
        }
        if li + 1 < lines.len() {
            // ENTER advances to the next drill line
            push_cell(&mut out, '⏎', pos_state(li, line.len(), cli, cci), err, false);
        }
        out.push_str("\n\n");
    }
    out
}

fn live_stats(typed: u32, errors: u32, start: Option<Instant>) -> (f64, f64) {
    let acc = if typed + errors == 0 {
        100.0
    } else {
        100.0 * typed as f64 / (typed + errors) as f64
    };
    let wpm = match start {
        Some(s) => {
            let m = s.elapsed().as_secs_f64() / 60.0;
            if m > 0.0005 { (typed as f64 / 5.0) / m } else { 0.0 }
        }
        None => 0.0,
    };
    (wpm, acc)
}

struct DrillResult {
    wpm: f64,
    acc: f64,
    errors: u32,
    secs: f64,
    rhythm: f64,
}

/// One drill, whatever produced the lines. Timing bookkeeping rides along
/// on the keypress that is already being handled: one `Instant::now()` per
/// correct key, two hash lookups, no timer and nothing polled.
fn run_drill(lay: usize, title: &str, text: &[String], st: &mut Stats) -> Option<DrillResult> {
    let lines: Vec<Vec<char>> = text.iter().map(|l| l.chars().collect()).collect();
    if lines.is_empty() { return None; }
    let mut ui = fresh_ui();
    let (mut li, mut ci) = (0usize, 0usize);
    let mut typed: u32 = 0;
    let mut errors: u32 = 0;
    let mut err_flash = false;
    let mut start: Option<Instant> = None;
    let mut last: Option<Instant> = None;
    let mut prev_char: Option<char> = None;
    let mut intervals: Vec<f64> = Vec::new();
    let mut keys: HashMap<char, (f64, u32)> = HashMap::new();
    let mut pairs: HashMap<String, (f64, u32)> = HashMap::new();
    let gate = st.gate(lay);

    loop {
        let (wpm, acc) = live_stats(typed, errors, start);
        let speed = if gate {
            format!("wpm {:>3.0} · rhythm {:>3.0}", wpm, evenness(&intervals))
        } else {
            "speed hidden".to_string()
        };
        draw(
            &mut ui,
            &format!(" TYPO · {}", title),
            &drill_text(&lines, li, ci, err_flash),
            &format!(" accuracy {:>3.0}% · errors {} · {} · ESC back", acc, errors, speed),
        );

        let Some(key) = Input::getchr(None) else { continue };
        let pressed: Option<char> = match key.as_str() {
            "ESC" => return None,
            "RESIZE" => {
                ui = fresh_ui();
                continue;
            }
            "ENTER" => Some('\n'),
            k if k.chars().count() == 1 => k.chars().next(),
            _ => None,
        };
        let Some(p) = pressed else { continue };
        let now = Instant::now();
        if start.is_none() {
            start = Some(now);
        }

        let expect = if ci < lines[li].len() { lines[li][ci] } else { '\n' };
        if p == expect {
            typed += 1;
            err_flash = false;
            // A gap only counts when the previous key was also correct and
            // the typist didn't stop to think.
            if p != '\n' {
                if let (Some(t0), Some(pc)) = (last, prev_char) {
                    let ms = now.duration_since(t0).as_secs_f64() * 1000.0;
                    if ms <= MAX_GAP_MS && pc != '\n' {
                        intervals.push(ms);
                        let e = keys.entry(p).or_insert((0.0, 0));
                        e.0 += ms;
                        e.1 += 1;
                        let e = pairs.entry(format!("{}{}", pc, p)).or_insert((0.0, 0));
                        e.0 += ms;
                        e.1 += 1;
                    }
                }
                last = Some(now);
                prev_char = Some(p);
            } else {
                last = None;
                prev_char = None;
            }
            if p == '\n' {
                li += 1;
                ci = 0;
            } else {
                ci += 1;
            }
            if li == lines.len() - 1 && ci == lines[li].len() {
                let secs = start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
                let minutes = (secs / 60.0).max(1.0 / 600.0);
                let wpm = (typed as f64 / 5.0) / minutes;
                let acc = 100.0 * typed as f64 / (typed + errors) as f64;
                for (c, (t, n)) in keys {
                    let e = st.keys[lay].entry(c).or_insert((0.0, 0));
                    e.0 += t;
                    e.1 += n;
                }
                for (s, (t, n)) in pairs {
                    let e = st.pairs[lay].entry(s).or_insert((0.0, 0));
                    e.0 += t;
                    e.1 += n;
                }
                return Some(DrillResult {
                    wpm,
                    acc,
                    errors,
                    secs,
                    rhythm: evenness(&intervals),
                });
            }
        } else {
            errors += 1;
            err_flash = true;
        }
    }
}

// ---------- result ----------

/// What a finished round changes: the clean streak, and in gradual mode the
/// key set. Returns a line to show when a key was just handed out.
fn record_round(lay: usize, r: &DrillResult, gradual: bool, st: &mut Stats) -> Option<String> {
    let clean = r.acc >= CLEAN_ACC;
    if clean {
        st.clean[lay] = st.clean[lay].saturating_add(1);
    } else {
        st.clean[lay] = 0;
    }
    if !gradual {
        return None;
    }
    if !clean {
        st.gclean[lay] = 0;
        return None;
    }
    st.gclean[lay] += 1;
    if st.gclean[lay] < CLEAN_ROUNDS || st.all_unlocked(lay) {
        return None;
    }
    st.gclean[lay] = 0;
    st.unlocked[lay] += 1;
    let new = LAYOUTS[lay]
        .unlock
        .chars()
        .nth(st.unlocked[lay] - 1)
        .map(key_label)
        .unwrap_or_default();
    Some(format!("New key unlocked: {}", new))
}

fn result_text(lay: usize, title: &str, r: &DrillResult, extra: &[String], st: &Stats) -> String {
    let mut txt = String::from("\n");
    txt.push_str(&format!("{}\n\n", style::bold(title)));
    txt.push_str(&format!("Accuracy  {:.0}%\n", r.acc));
    txt.push_str(&format!("Errors    {}\n", r.errors));
    txt.push_str(&format!("Time      {:.0}s\n", r.secs));
    if st.gate(lay) {
        txt.push_str(&format!("Speed     {:.0} wpm\n", r.wpm));
        txt.push_str(&format!("Rhythm    {:.0}\n", r.rhythm));
    } else {
        txt.push_str(&format!(
            "\n{}\n",
            style::fg(
                &format!(
                    "Speed stays hidden: {} of {} rounds above {:.0}%.",
                    st.clean[lay], CLEAN_ROUNDS, CLEAN_ACC
                ),
                220
            )
        ));
    }
    let (keys, pairs) = st.weak(lay);
    if !keys.is_empty() {
        let k = keys
            .iter()
            .take(3)
            .map(|(c, ms)| format!("{} {:.0}ms", key_label(*c), ms))
            .collect::<Vec<_>>()
            .join("   ");
        txt.push_str(&format!("\nSlowest keys   {}\n", k));
    }
    if !pairs.is_empty() {
        let p = pairs
            .iter()
            .take(2)
            .map(|(s, ms)| format!("{} {:.0}ms", s, ms))
            .collect::<Vec<_>>()
            .join("   ");
        txt.push_str(&format!("Slowest pairs  {}\n", p));
    }
    for line in extra {
        txt.push_str(&format!("\n{}\n", style::fg(line, 220)));
    }
    txt
}

/// Result screen. Returns true to run the same drill again.
fn show_result(lay: usize, title: &str, r: &DrillResult, extra: &[String], st: &Stats) -> bool {
    let mut ui = fresh_ui();
    loop {
        draw(
            &mut ui,
            &format!(" TYPO · {}", title),
            &result_text(lay, title, r, extra, st),
            " r retry · w drill my weak keys · any other key back to menu",
        );
        match Input::getchr(None).as_deref() {
            Some("RESIZE") => ui = fresh_ui(),
            Some("r") => return true,
            Some(_) => return false,
            None => {}
        }
    }
}

// ---------- trend ----------

fn trend_text(lay: usize, st: &Stats) -> String {
    // One point per day: the best run of that day.
    let mut days: Vec<(String, f64, f64, f64)> = Vec::new();
    for (l, d, w, a, e) in &st.tests {
        if *l != lay { continue; }
        match days.iter_mut().find(|(dd, ..)| dd == d) {
            Some(row) => {
                if *w > row.1 { *row = (d.clone(), *w, *a, *e); }
            }
            None => days.push((d.clone(), *w, *a, *e)),
        }
    }
    days.sort_by(|a, b| a.0.cmp(&b.0));
    let show: Vec<_> = days.iter().rev().take(14).rev().cloned().collect();

    let mut txt = String::from("\n");
    txt.push_str(&format!(
        "{}   ({})\n\n",
        style::bold("Daily test trend"),
        LAYOUTS[lay].label
    ));
    if show.is_empty() {
        txt.push_str("No daily tests yet. Press t in the menu to take the first one.\n\n");
        txt.push_str("The test is the same text every time, so the line means something.\n");
        txt.push_str("Expect two to three weeks slower than your old habit, then past it.\n");
        return txt;
    }
    let max = show.iter().fold(1.0f64, |m, (_, w, _, _)| m.max(*w));
    for (d, w, a, e) in &show {
        let bar = "█".repeat(((w / max) * 30.0).round() as usize);
        txt.push_str(&format!(
            "{}  {:>3.0} wpm  {:>3.0}%  rhythm {:>3.0}  {}\n",
            d, w, a, e, style::fg(&bar, DONE_FG)
        ));
    }
    let first = show.first().map(|(_, w, ..)| *w).unwrap_or(0.0);
    let last = show.last().map(|(_, w, ..)| *w).unwrap_or(0.0);
    txt.push_str(&format!("\nAcross these {} days: {:+.0} wpm\n", show.len(), last - first));
    txt
}

fn show_trend(st: &Stats) {
    let mut ui = fresh_ui();
    loop {
        draw(
            &mut ui,
            &format!(" TYPO · trend · {}", LAYOUTS[st.layout].label),
            &trend_text(st.layout, st),
            " any key back to menu",
        );
        match Input::getchr(None).as_deref() {
            Some("RESIZE") => ui = fresh_ui(),
            Some(_) => return,
            None => {}
        }
    }
}

// ---------- drill drivers ----------

enum Drill {
    Lesson(usize),
    Gradual,
    Weak,
    Daily,
}

fn run(kind: Drill, st: &mut Stats, rng: &mut Rng) {
    loop {
        let lay = st.layout;
        let (title, text, gradual) = match kind {
            Drill::Lesson(i) => {
                let l = &LAYOUTS[lay].lessons[i];
                let t: Vec<String> = l.lines.iter().map(|s| s.to_string()).collect();
                (format!("{}. {}", i + 1, l.name), t, false)
            }
            Drill::Gradual => (
                format!("Gradual · {} keys", st.unlocked[lay]),
                gradual_lines(lay, st, rng),
                true,
            ),
            Drill::Weak => match weak_lines(lay, st, rng) {
                Some(t) => ("Weak keys".to_string(), t, false),
                None => {
                    notice(
                        "Not enough typing yet",
                        "Run a lesson or two first — weak keys need a handful of\n\
                         samples per key before the averages mean anything.",
                    );
                    return;
                }
            },
            Drill::Daily => ("Daily test".to_string(), daily_lines(lay), false),
        };

        let Some(r) = run_drill(lay, &title, &text, st) else { return };

        let mut extra = Vec::new();
        if let Some(msg) = record_round(lay, &r, gradual, st) {
            extra.push(msg);
        }
        if let Drill::Lesson(i) = kind {
            let key = best_key(lay, &LAYOUTS[lay].lessons[i]);
            let record = st.best.get(&key).map(|(w, _)| r.wpm > *w).unwrap_or(true);
            if record {
                st.best.insert(key, (r.wpm, r.acc));
                if st.gate(lay) {
                    extra.push("New personal best!".to_string());
                }
            }
        }
        if let Drill::Daily = kind {
            st.tests.push((lay, today(), r.wpm, r.acc, r.rhythm));
        }
        save_state(st);

        if !show_result(lay, &title, &r, &extra, st) {
            return;
        }
    }
}

fn notice(title: &str, body: &str) {
    let mut ui = fresh_ui();
    loop {
        draw(
            &mut ui,
            &format!(" TYPO · {}", title),
            &format!("\n{}\n\n{}\n", style::bold(title), body),
            " any key back to menu",
        );
        match Input::getchr(None).as_deref() {
            Some("RESIZE") => ui = fresh_ui(),
            Some(_) => return,
            None => {}
        }
    }
}

// ---------- main ----------

fn main() {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-v" | "--version" => {
                println!("typo {}", VERSION);
                return;
            }
            "-h" | "--help" => {
                println!("typo: terminal touch-typing tutor (Fe2O3 suite)");
                println!();
                println!("Usage: typo");
                println!();
                println!("Menu keys:  j/k or arrows move · 1-9 jump straight in");
                println!("            g gradual mode · w weak keys · t daily test · T trend");
                println!("            l toggle layout (US/Norwegian) · ENTER start · q quit");
                println!("Drill keys: type what you see · ⏎ = press ENTER · ESC back");
                println!();
                println!("Accuracy comes first: speed stays hidden until three rounds in a");
                println!("row clear 97%. Gradual mode hands out one new key per three clean");
                println!("rounds. The daily test is the same text every time, so the trend");
                println!("is worth reading.");
                println!();
                println!("Layout, bests, key timings and the test log are kept in ~/.typo");
                return;
            }
            _ => {
                eprintln!("typo: unknown argument '{}'", arg);
                std::process::exit(1);
            }
        }
    }

    let mut st = load_state();
    let mut rng = Rng::new();
    Crust::init();
    Crust::set_app_identity("Typo");
    let mut ui = fresh_ui();
    let mut sel: usize = 0;

    loop {
        draw(
            &mut ui,
            &format!(
                " TYPO v{} · touch typing tutor · {}",
                VERSION, LAYOUTS[st.layout].label
            ),
            &menu_text(&st, sel),
            " j/k move · 1-9 jump · g gradual · w weak · t test · T trend · l layout · q quit",
        );
        let Some(key) = Input::getchr(None) else { continue };
        match key.as_str() {
            "q" | "Q" | "ESC" => break,
            "UP" | "k" => sel = sel.saturating_sub(1),
            "DOWN" | "j" => {
                if sel + 1 < LAYOUTS[st.layout].lessons.len() {
                    sel += 1;
                }
            }
            "l" => {
                st.layout = (st.layout + 1) % LAYOUTS.len();
                sel = sel.min(LAYOUTS[st.layout].lessons.len() - 1);
                save_state(&st);
            }
            "RESIZE" => ui = fresh_ui(),
            "ENTER" => {
                run(Drill::Lesson(sel), &mut st, &mut rng);
                ui = fresh_ui();
            }
            "g" => {
                run(Drill::Gradual, &mut st, &mut rng);
                ui = fresh_ui();
            }
            "w" => {
                run(Drill::Weak, &mut st, &mut rng);
                ui = fresh_ui();
            }
            "t" => {
                run(Drill::Daily, &mut st, &mut rng);
                ui = fresh_ui();
            }
            "T" => {
                show_trend(&st);
                ui = fresh_ui();
            }
            d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() => {
                let n = (d.as_bytes()[0] - b'0') as usize;
                if n >= 1 && n <= LAYOUTS[st.layout].lessons.len() {
                    sel = n - 1;
                    run(Drill::Lesson(sel), &mut st, &mut rng);
                    ui = fresh_ui();
                }
            }
            _ => {}
        }
    }
    save_state(&st);
    Crust::cleanup();
}
