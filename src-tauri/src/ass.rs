// build_ass — generate a burned-subtitle .ass track (spec §8/§9 ASS section).
use crate::models::{CaptionResult, CaptionWord, SubtitleConfig};
use crate::storage::captions_work_dir;
use std::path::PathBuf;

// "#RRGGBB" -> "BBGGRR" (ASS is BGR)
fn bgr(color: &str) -> String {
    let c = color.trim_start_matches('#');
    if c.len() >= 6 {
        format!("{}{}{}", &c[4..6], &c[2..4], &c[0..2]).to_uppercase()
    } else {
        "FFFFFF".into()
    }
}

fn tc(t: f64) -> String {
    let t = t.max(0.0);
    let h = (t / 3600.0) as u64;
    let m = ((t % 3600.0) / 60.0) as u64;
    let s = (t % 60.0) as u64;
    let cs = ((t - t.floor()) * 100.0) as u64;
    format!("{h}:{m:02}:{s:02}.{cs:02}")
}

fn esc(s: &str) -> String {
    s.replace('\n', "\\N")
}

pub fn build_ass(caption: &CaptionResult, config: &SubtitleConfig, width: u32, height: u32) -> Result<String, String> {
    let dir = captions_work_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out: PathBuf = dir.join(format!("subtitles-{}.ass", uuid::Uuid::new_v4().simple()));

    let border_style = if config.use_box { 3 } else { 1 };
    let shadow = if config.use_box { 0 } else { 1 };
    let primary = format!("&H00{}", bgr(&config.primary_color));

    let mut s = String::new();
    s.push_str("[Script Info]\n");
    s.push_str("ScriptType: v4.00+\n");
    s.push_str(&format!("PlayResX: {width}\n"));
    s.push_str(&format!("PlayResY: {height}\n"));
    s.push_str("ScaledBorderAndShadow: yes\n");
    s.push_str("WrapStyle: 2\n\n");
    s.push_str("[V4+ Styles]\n");
    s.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");
    s.push_str(&format!(
        "Style: Default,{},{},{},&H00000000,&H00000000,&HB4000000,1,0,0,0,100,100,0,0,{},{},{},5,0,0,0,1\n\n",
        config.font, config.font_size as i64, primary, border_style, config.outline, shadow
    ));
    s.push_str("[Events]\n");
    s.push_str("Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");

    let x = width / 2;
    let y = (config.position.clamp(0.05, 0.95) * height as f64).round() as u32;
    let pos = format!("{{\\an5\\pos({x},{y})}}");

    // build word list (synthesize from segments if empty)
    let words: Vec<CaptionWord> = if !caption.words.is_empty() {
        caption.words.clone()
    } else {
        let mut v = Vec::new();
        for seg in &caption.segments {
            let toks: Vec<&str> = seg.text.split_whitespace().collect();
            if toks.is_empty() { continue; }
            let dur = (seg.end - seg.start).max(0.10) / toks.len() as f64;
            for (i, t) in toks.iter().enumerate() {
                v.push(CaptionWord { word: (*t).to_string(), start: seg.start + i as f64 * dur, end: seg.start + (i as f64 + 1.0) * dur });
            }
        }
        v
    };

    let up = |w: &str| if config.uppercase { w.to_uppercase() } else { w.to_string() };

    if config.mode == "word" {
        for w in &words {
            let end = w.end.max(w.start + 0.05);
            s.push_str(&format!("Dialogue: 0,{},{},Default,,0,0,0,,{}{}\n", tc(w.start), tc(end), pos, esc(&up(&w.word))));
        }
    } else {
        // inline colour-tag form: &H + BBGGRR + trailing &  (e.g. {\c&H00E0FF&})
        let hi = format!("&H{}&", bgr(&config.highlight_color));
        let pri = format!("&H{}&", bgr(&config.primary_color));
        let mw = config.max_words.max(1) as usize;
        for group in words.chunks(mw) {
            for (i, active) in group.iter().enumerate() {
                let end = active.end.max(active.start + 0.05);
                let mut text = String::new();
                for (j, w) in group.iter().enumerate() {
                    if j > 0 { text.push(' '); }
                    if j == i {
                        text.push_str(&format!("{{\\c{hi}}}{}{{\\c{pri}}}", esc(&up(&w.word))));
                    } else {
                        text.push_str(&esc(&up(&w.word)));
                    }
                }
                s.push_str(&format!("Dialogue: 0,{},{},Default,,0,0,0,,{}{}\n", tc(active.start), tc(end), pos, text));
            }
        }
    }

    std::fs::write(&out, s).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().to_string())
}
