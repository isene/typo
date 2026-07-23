//! typo — terminal touch-typing tutor. Part of the Fe2O3 suite.
//!
//! Strict tutor: the drill only advances on the correct key; wrong keys
//! count as errors. Everything is event-driven — no timers, no polling,
//! zero idle cost. WPM and accuracy are recomputed on each keypress.

use crust::style;
use crust::{Crust, Input, Pane};
use std::collections::HashMap;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const DONE_FG: u8 = 71; // typed chars: green
const TODO_FG: u8 = 246; // untyped chars: gray
const ERR_FG: u8 = 196; // current char after a miss: red
const SEL_BG: u8 = 81; // menu selection bar

struct Lesson {
    name: &'static str,
    lines: &'static [&'static str],
}

struct Layout {
    code: &'static str,
    label: &'static str,
    lessons: &'static [Lesson],
}

const LAYOUTS: &[Layout] = &[
    Layout { code: "us", label: "US", lessons: LESSONS_US },
    Layout { code: "no", label: "Norwegian", lessons: LESSONS_NO },
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

// ---------- state (~/.typo: layout line + per-layout bests) ----------

fn stats_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".typo")
}

fn best_key(lay: usize, lesson: &Lesson) -> String {
    format!("{}:{}", LAYOUTS[lay].code, lesson.name)
}

fn load_state() -> (usize, HashMap<String, (f64, f64)>) {
    let mut lay = 0;
    let mut map = HashMap::new();
    if let Ok(data) = std::fs::read_to_string(stats_path()) {
        for line in data.lines() {
            let mut it = line.split('\t');
            match (it.next(), it.next(), it.next()) {
                (Some("layout"), Some(code), _) => {
                    if let Some(i) = LAYOUTS.iter().position(|l| l.code == code) {
                        lay = i;
                    }
                }
                (Some(name), Some(w), Some(a)) => {
                    if let (Ok(w), Ok(a)) = (w.parse(), a.parse()) {
                        map.insert(name.to_string(), (w, a));
                    }
                }
                _ => {}
            }
        }
    }
    (lay, map)
}

fn save_state(lay: usize, best: &HashMap<String, (f64, f64)>) {
    let mut out = format!("layout\t{}\n", LAYOUTS[lay].code);
    for (li, layout) in LAYOUTS.iter().enumerate() {
        for lesson in layout.lessons {
            let key = best_key(li, lesson);
            if let Some((w, a)) = best.get(&key) {
                out.push_str(&format!("{}\t{:.1}\t{:.1}\n", key, w, a));
            }
        }
    }
    let _ = std::fs::write(stats_path(), out);
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

fn menu_text(lay: usize, sel: usize, best: &HashMap<String, (f64, f64)>) -> String {
    let mut out = String::from("\n");
    out.push_str(&format!(
        "{}   (layout: {})\n\n",
        style::bold("Select a lesson"),
        LAYOUTS[lay].label
    ));
    for (i, l) in LAYOUTS[lay].lessons.iter().enumerate() {
        let score = match best.get(&best_key(lay, l)) {
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

fn push_cell(out: &mut String, ch: char, st: CellState, err: bool) {
    let s = ch.to_string();
    match st {
        CellState::Done => out.push_str(&style::fg(&s, DONE_FG)),
        CellState::Todo => out.push_str(&style::fg(&s, TODO_FG)),
        CellState::Current => {
            if err {
                out.push_str(&format!("\x1b[38;5;{};7m{}\x1b[27;39m", ERR_FG, s));
            } else {
                out.push_str(&format!("\x1b[7m{}\x1b[27m", s));
            }
        }
    }
}

fn drill_text(lines: &[Vec<char>], cli: usize, cci: usize, err: bool) -> String {
    let mut out = String::from("\n");
    for (li, line) in lines.iter().enumerate() {
        for (ci, &ch) in line.iter().enumerate() {
            push_cell(&mut out, ch, pos_state(li, ci, cli, cci), err);
        }
        if li + 1 < lines.len() {
            // ENTER advances to the next drill line
            push_cell(&mut out, '⏎', pos_state(li, line.len(), cli, cci), err);
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
}

fn run_drill(lay: usize, sel: usize) -> Option<DrillResult> {
    let lesson = &LAYOUTS[lay].lessons[sel];
    let lines: Vec<Vec<char>> = lesson.lines.iter().map(|l| l.chars().collect()).collect();
    let mut ui = fresh_ui();
    let (mut li, mut ci) = (0usize, 0usize);
    let mut typed: u32 = 0;
    let mut errors: u32 = 0;
    let mut err_flash = false;
    let mut start: Option<Instant> = None;

    loop {
        let (wpm, acc) = live_stats(typed, errors, start);
        draw(
            &mut ui,
            &format!(" TYPO · {}. {}", sel + 1, lesson.name),
            &drill_text(&lines, li, ci, err_flash),
            &format!(" wpm {:>3.0} · accuracy {:>3.0}% · errors {} · ESC back", wpm, acc, errors),
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
        if start.is_none() {
            start = Some(Instant::now());
        }

        let expect = if ci < lines[li].len() {
            lines[li][ci]
        } else {
            '\n'
        };
        if p == expect {
            typed += 1;
            err_flash = false;
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
                return Some(DrillResult { wpm, acc, errors, secs });
            }
        } else {
            errors += 1;
            err_flash = true;
        }
    }
}

// ---------- result ----------

fn show_result(
    lay: usize,
    sel: usize,
    r: &DrillResult,
    best: &mut HashMap<String, (f64, f64)>,
) -> bool {
    let lesson = &LAYOUTS[lay].lessons[sel];
    let key = best_key(lay, lesson);
    let prev = best.get(&key).copied();
    let record = prev.map(|(w, _)| r.wpm > w).unwrap_or(true);
    if record {
        best.insert(key, (r.wpm, r.acc));
        save_state(lay, best);
    }
    let mut ui = fresh_ui();
    loop {
        let mut txt = String::from("\n");
        txt.push_str(&format!(
            "{}\n\n",
            style::bold(&format!("Lesson {}: {}", sel + 1, lesson.name))
        ));
        txt.push_str(&format!("Speed     {:.0} wpm\n", r.wpm));
        txt.push_str(&format!("Accuracy  {:.0}%\n", r.acc));
        txt.push_str(&format!("Errors    {}\n", r.errors));
        txt.push_str(&format!("Time      {:.0}s\n", r.secs));
        if record {
            txt.push_str(&format!("\n{}\n", style::fg("New personal best!", 220)));
        } else if let Some((w, a)) = prev {
            txt.push_str(&format!("\nBest      {:.0} wpm  {:.0}%\n", w, a));
        }
        draw(
            &mut ui,
            &format!(" TYPO · {}. {}", sel + 1, lesson.name),
            &txt,
            " r retry · any other key back to menu",
        );
        match Input::getchr(None).as_deref() {
            Some("RESIZE") => ui = fresh_ui(),
            Some("r") => return true,
            Some(_) => return false,
            None => {}
        }
    }
}

fn run_lesson(lay: usize, sel: usize, best: &mut HashMap<String, (f64, f64)>) {
    loop {
        match run_drill(lay, sel) {
            None => return,
            Some(r) => {
                if !show_result(lay, sel, &r, best) {
                    return;
                }
            }
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
                println!("Menu keys:  j/k or arrows move · 1-8 jump straight in");
                println!("            l toggle layout (US/Norwegian) · ENTER start · q quit");
                println!("Drill keys: type what you see · ⏎ = press ENTER · ESC back");
                println!();
                println!("Layout and personal bests are kept in ~/.typo");
                return;
            }
            _ => {
                eprintln!("typo: unknown argument '{}'", arg);
                std::process::exit(1);
            }
        }
    }

    let (mut lay, mut best) = load_state();
    Crust::init();
    Crust::set_app_identity("Typo");
    let mut ui = fresh_ui();
    let mut sel: usize = 0;

    loop {
        draw(
            &mut ui,
            &format!(" TYPO v{} · touch typing tutor · {}", VERSION, LAYOUTS[lay].label),
            &menu_text(lay, sel, &best),
            " j/k move · 1-8 jump · ENTER start · l layout · q quit",
        );
        let Some(key) = Input::getchr(None) else { continue };
        match key.as_str() {
            "q" | "Q" | "ESC" => break,
            "UP" | "k" => sel = sel.saturating_sub(1),
            "DOWN" | "j" => {
                if sel + 1 < LAYOUTS[lay].lessons.len() {
                    sel += 1;
                }
            }
            "l" => {
                lay = (lay + 1) % LAYOUTS.len();
                save_state(lay, &best);
            }
            "RESIZE" => ui = fresh_ui(),
            "ENTER" => {
                run_lesson(lay, sel, &mut best);
                ui = fresh_ui();
            }
            d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() => {
                let n = (d.as_bytes()[0] - b'0') as usize;
                if n >= 1 && n <= LAYOUTS[lay].lessons.len() {
                    sel = n - 1;
                    run_lesson(lay, sel, &mut best);
                    ui = fresh_ui();
                }
            }
            _ => {}
        }
    }
    Crust::cleanup();
}
