use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

use rime_core::{
    engine::Engine,
    key_event::{Action, InputEvent},
    session::Session,
};
use rime_dict::TsvDictionary;
use rime_pinyin::QuanpinPreeditor;

fn main() -> io::Result<()> {
    let dict_path = parse_args().unwrap_or_else(default_dict_path);
    let dict = TsvDictionary::from_path(&dict_path)?;
    let preeditor = QuanpinPreeditor::new();
    let engine = Engine::new(dict, preeditor).candidate_limit(9);

    let mut committed: Vec<String> = Vec::new();
    let mut session = Session::new(engine);
    repl(&mut session, &dict_path, &mut committed)
}

fn parse_args() -> Option<PathBuf> {
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--dict" {
            if let Some(p) = args.next() {
                return Some(PathBuf::from(p));
            }
        }
        if a == "--help" || a == "-h" {
            print_help();
        }
    }
    None
}

fn print_help() -> ! {
    println!("用法：rime_cli [--dict <path>]\n交互：按行提交（回车确认一行拼音），随后输入 1-9 选择候选；直接回车默认选 1；输入 0 上屏原串；输入 q 放弃本次");
    std::process::exit(0);
}

fn default_dict_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("asset").join("dict.tsv")
}

fn repl(session: &mut Session<TsvDictionary, QuanpinPreeditor>, dict_path: &PathBuf, committed: &mut Vec<String>) -> io::Result<()> {
    let mut out = io::stdout();
    let mut line = String::new();
    writeln!(out, "rime-rs demo (全拼 CLI, std-only) | dict: {}", dict_path.display())?;
    writeln!(out, "输入拼音后回车。输入 :q 退出。")?;
    (&mut out).flush()?;

    loop {
        (&mut line).clear();
        print!("pinyin>");
        out.flush()?;
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        println!("--------------------");
        println!("input:{input}");
        if input == ":q" || input == ":quit" || input == ":exit" {
            break;
        }
        let raw: String = sanitize_input(input);
        if raw.is_empty() {
            writeln!(out, "(忽略：只接受 a-z 和 ' )")?;
            continue;
        }

        // feed into session (line-base)
        (&mut *session).handle(InputEvent::Clear);
        for ch in raw.chars() {
            (&mut *session).handle(InputEvent::Char(ch));
        }

        // selection loop: may require multiple steps (confirmed advances)
        loop {
            let ui = session.ui_state();
            writeln!(out, "> {}", ui.preedit)?;
            if !ui.confirm_text.is_empty() {
                writeln!(out, "  confirmed: {} ({} / {})", ui.confirm_text, ui.confirm, ui.caret)?;
            } else {
                writeln!(out, "  confirmed: (0 / {})", ui.caret)?;
            }

            if ui.candidate_list.is_empty() {
                // 无候选：直接上屏原串并清空
                committed.push(ui.raw_input.clone());
                writeln!(out, "commit: {}", ui.raw_input)?;
                session.handle(InputEvent::Clear);
                break;
            }

            for (i, c) in ui.candidate_list.iter().enumerate() {
                let n = i + 1;
                let display_text = if ui.confirm_text.is_empty() { c.text.clone() } else { format!("{}{}", ui.confirm_text, c.text) };
                match &c.comment {
                    Some(comment) => writeln!(out, "{n}. {}\t({comment})", display_text)?,
                    None => writeln!(out, "{n}. {}", display_text)?,
                }
            }

            line.clear();
            print!("select [1-{}] (Enter=1, 0=raw, q=cancel)> ", ui.candidate_list.len().min(9));
            (&mut out).flush()?;
            if io::stdin().read_line(&mut line)? == 0 {
                return Ok(());
            }
            let sel = line.trim();
            // if sel == "q" || sel == "Q" {
            //     writeln!(out, "(cancel)")?;
            //     session.handle(InputEvent::Clear);
            //     break;
            // }
            if sel == "0" {
                let text = ui.raw_input.clone();
                committed.push(text.clone());
                writeln!(out, "commit: {text}")?;
                session.handle(InputEvent::Clear);
                break;
            }

            let idx = if sel.is_empty() { Some(0usize) } else { sel.parse::<usize>().ok().and_then(|n| (1..=9).contains(&n).then_some(n - 1)) };
            let Some(i) = idx else {
                writeln!(out, "无效选择，请输入 1-9 / 0 / q / 直接回车")?;
                continue;
            };

            let (_ui2, actions) = session.handle(InputEvent::Select(i));
            let mut committed_now = None;
            for a in actions {
                let Action::Commit(s) = a;
                committed_now = Some(s);
            }
            if let Some(s) = committed_now {
                committed.push(s.clone());
                writeln!(out, "commit: {s}")?;
                break;
            }
        }
    }

    Ok(())
}

fn sanitize_input(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphabetic() || ch == '\'' {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}
