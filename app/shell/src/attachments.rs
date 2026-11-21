//! Files attached to a chat message: the text is read out here so the assistant can use it.
//!
//! Supported: plain text and code (anything that is valid text), PDF (its text layer), and Word
//! `.docx`. Not run, not opened by any program: only read. Limits keep one message small: 8 MB per
//! file, 60 000 characters of text per file, 3 files per message.

use std::io::Read;

pub const MAX_FILES: usize = 3;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_TEXT: usize = 60_000;

/// A file as the UI sends it: a name and its bytes as base64.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upload {
    pub name: String,
    pub data: String,
}

/// What was read from a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extracted {
    pub name: String,
    pub text: String,
    /// True when the text was cut at [`MAX_TEXT`].
    pub truncated: bool,
}

fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let n: String = base.chars().filter(|c| !c.is_control()).take(80).collect();
    if n.trim().is_empty() {
        "file".into()
    } else {
        n
    }
}

fn finish(name: &str, text: String) -> Result<Extracted, String> {
    let text: String = text
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect();
    if text.trim().is_empty() {
        return Err(format!(
            "{name} has no text I can read (a scanned or image-only file?)."
        ));
    }
    let truncated = text.chars().count() > MAX_TEXT;
    let text = if truncated {
        text.chars().take(MAX_TEXT).collect()
    } else {
        text
    };
    Ok(Extracted {
        name: name.to_string(),
        text,
        truncated,
    })
}

fn docx_text(bytes: &[u8]) -> Result<String, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| "That is not a valid Word file.".to_string())?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|_| "That Word file has no document text.".to_string())?
        .take(40 * 1024 * 1024)
        .read_to_string(&mut xml)
        .map_err(|_| "That Word file could not be read.".to_string())?;
    // paragraphs and line breaks become newlines, tabs stay tabs, everything else in tags is dropped
    let mut out = String::new();
    let mut rest = xml.as_str();
    while let Some(lt) = rest.find('<') {
        let (text, after) = rest.split_at(lt);
        out.push_str(&unescape(text));
        let Some(gt) = after.find('>') else { break };
        let tag = &after[1..gt];
        if tag.starts_with("/w:p") || tag.starts_with("w:br") {
            out.push('\n');
        } else if tag.starts_with("w:tab") {
            out.push('\t');
        }
        rest = &after[gt + 1..];
    }
    Ok(out)
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|b| *b == 0)
}

/// Reads the text of one file.
pub fn extract(name: &str, bytes: &[u8]) -> Result<Extracted, String> {
    let name = clean_name(name);
    if bytes.len() > MAX_BYTES {
        return Err(format!("{name} is too big (8 MB at most)."));
    }
    if bytes.is_empty() {
        return Err(format!("{name} is empty."));
    }
    let lower = name.to_lowercase();
    if bytes.starts_with(b"%PDF") || lower.ends_with(".pdf") {
        // A broken PDF must not take the browser down: the reader runs inside catch_unwind.
        let data = bytes.to_vec();
        let text = std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem(&data))
            .map_err(|_| format!("{name} could not be read."))?
            .map_err(|_| format!("{name} could not be read (it may be protected or damaged)."))?;
        return finish(&name, text);
    }
    if lower.ends_with(".docx") || (bytes.starts_with(b"PK") && lower.ends_with(".docm")) {
        let text = docx_text(bytes).map_err(|e| format!("{name}: {e}"))?;
        return finish(&name, text);
    }
    if bytes.starts_with(b"PK")
        || bytes.starts_with(&[0x7f, b'E', b'L', b'F'])
        || bytes.starts_with(b"MZ")
    {
        return Err(format!("{name} is not a kind of file I can read."));
    }
    // UTF-8, with a UTF-16 byte-order mark handled, else Latin-1 as a last resort
    let utf16 = bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
    if !utf16 && looks_binary(bytes) {
        return Err(format!("{name} is not a text file."));
    }
    let text = if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let u: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16_lossy(&u)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let u: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes(*c))
            .collect();
        String::from_utf16_lossy(&u)
    } else {
        match std::str::from_utf8(bytes) {
            Ok(t) => t.trim_start_matches('\u{feff}').to_string(),
            Err(_) => bytes.iter().map(|b| *b as char).collect(),
        }
    };
    finish(&name, text)
}

/// Decodes and reads the files of one message. A file that cannot be read is reported, not dropped
/// silently: the result has one entry per file, in order.
pub fn extract_all(files: &[Upload]) -> Vec<Result<Extracted, String>> {
    use base64::Engine as _;
    files
        .iter()
        .take(MAX_FILES)
        .map(|f| {
            let name = clean_name(&f.name);
            // the base64 text of a file at the limit is a third bigger
            if f.data.len() > MAX_BYTES * 4 / 3 + 16 {
                return Err(format!("{name} is too big (8 MB at most)."));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(f.data.trim())
                .map_err(|_| format!("{name} could not be read."))?;
            extract(&name, &bytes)
        })
        .collect()
}

/// The text the model sees for the attached files.
pub fn prompt_block(items: &[Extracted]) -> String {
    let mut out = String::new();
    for e in items {
        out.push_str(&format!(
            "\n\n[Attached file: {}]\n```\n{}\n```",
            e.name, e.text
        ));
        if e.truncated {
            out.push_str("\n(the file is longer; only the start is shown)");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    fn b64(b: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(b)
    }

    #[test]
    fn text_and_code_are_read() {
        let e = extract("C:\\x\\notes.md", "# Title\nSome Käse text".as_bytes()).unwrap();
        assert_eq!(e.name, "notes.md");
        assert!(e.text.contains("Käse") && !e.truncated);
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("hi there".encode_utf16().flat_map(|u| u.to_le_bytes()))
            .collect();
        assert_eq!(extract("a.txt", &utf16).unwrap().text, "hi there");
        assert_eq!(
            extract("l1.txt", &[0x63, 0x61, 0x66, 0xE9]).unwrap().text,
            "café",
            "Latin-1 last resort"
        );
    }

    #[test]
    fn long_text_is_cut_and_junk_is_refused() {
        let long = "x".repeat(MAX_TEXT + 50);
        let e = extract("big.txt", long.as_bytes()).unwrap();
        assert!(e.truncated && e.text.chars().count() == MAX_TEXT);
        assert!(extract("empty.txt", b"").is_err());
        assert!(
            extract("blank.txt", b"   \n\n")
                .unwrap_err()
                .contains("no text")
        );
        assert!(extract("p.exe", b"MZ\x90\x00\x03").is_err());
        assert!(
            extract("data.bin", &[1, 2, 0, 3, 4])
                .unwrap_err()
                .contains("not a text file")
        );
        assert!(
            extract("x.txt", &vec![b'a'; MAX_BYTES + 1])
                .unwrap_err()
                .contains("too big")
        );
        // control characters never reach the model
        assert_eq!(extract("c.txt", b"a\x07b\nc").unwrap().text, "ab\nc");
    }

    #[test]
    fn a_word_file_gives_its_paragraphs() {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            z.start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            z.write_all(b"<w:document><w:body><w:p><w:r><w:t>Hello &amp; welcome</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t></w:r><w:tab/><w:r><w:t>line</w:t></w:r></w:p></w:body></w:document>").unwrap();
            z.finish().unwrap();
        }
        let e = extract("memo.docx", &buf.into_inner()).unwrap();
        assert_eq!(e.text.trim(), "Hello & welcome\nSecond\tline");
        assert!(extract("fake.docx", b"not a zip").is_err());
    }

    #[test]
    fn a_pdf_file_is_read_and_a_broken_one_never_crashes() {
        // a minimal one-page PDF with the text "Hello PDF" (offsets worked out so it is well formed)
        let objs = [
            "<</Type/Catalog/Pages 2 0 R>>".to_string(),
            "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_string(),
            "<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 100]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>".to_string(),
            {
                let c = "BT /F1 18 Tf 20 50 Td (Hello PDF) Tj ET";
                format!("<</Length {}>>\nstream\n{c}\nendstream", c.len())
            },
            "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>".to_string(),
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
        }
        let xref = pdf.len();
        pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
        for off in &offsets {
            pdf.extend(format!("{off:010} 00000 n \n").bytes());
        }
        pdf.extend(
            format!(
                "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n",
                objs.len() + 1
            )
            .bytes(),
        );
        let pdf = pdf.as_slice();
        match extract("doc.pdf", pdf) {
            Ok(e) => assert!(e.text.contains("Hello PDF"), "{:?}", e.text),
            Err(e) => panic!("a simple PDF should be readable: {e}"),
        }
        assert!(extract("broken.pdf", b"%PDF-1.4 garbage garbage").is_err());
        assert!(extract("junk.pdf", &[0x25, 0x50, 0x44, 0x46, 0xff, 0xff, 0x00]).is_err());
    }

    #[test]
    fn a_batch_reports_each_file_and_limits_the_count() {
        let files: Vec<Upload> = (0..5)
            .map(|i| Upload {
                name: format!("f{i}.txt"),
                data: b64(format!("file {i}").as_bytes()),
            })
            .chain([Upload {
                name: "bad.txt".into(),
                data: "!!!not base64".into(),
            }])
            .collect();
        let r = extract_all(&files);
        assert_eq!(r.len(), MAX_FILES);
        assert!(r.iter().all(|x| x.is_ok()));
        let r2 = extract_all(&[Upload {
            name: "bad.txt".into(),
            data: "!!!".into(),
        }]);
        assert!(r2[0].is_err());
        let block = prompt_block(&[Extracted {
            name: "a.txt".into(),
            text: "hello".into(),
            truncated: true,
        }]);
        assert!(block.contains("[Attached file: a.txt]") && block.contains("only the start"));
    }
}
