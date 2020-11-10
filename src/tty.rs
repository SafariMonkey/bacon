use {
    crate::*,
    anyhow::*,
    std::io::Write,
    vte,
};

pub const CSI_RESET: &str = "\u{1b}[0m\u{1b}[0m";
pub const CSI_BOLD: &str = "\u{1b}[1m";
pub const CSI_BOLD_RED: &str = "\u{1b}[1m\u{1b}[38;5;9m";
pub const CSI_BOLD_YELLOW: &str = "\u{1b}[1m\u{1b}[33m";
pub const CSI_BOLD_BLUE: &str = "\u{1b}[1m\u{1b}[38;5;12m";

/// a simple representation of a colored and styled string
#[derive(Debug, Default)]
pub struct TString {
    pub csi: String,
    pub raw: String,
}
impl TString {
    pub fn push_csi(&mut self, params: &[i64], action: char) {
        self.csi.push('\u{1b}');
        self.csi.push('[');
        for (idx, p) in params.iter().enumerate() {
            self.csi.push_str(&format!("{}", p));
            if idx < params.len() - 1 {
                self.csi.push(';');
            }
        }
        self.csi.push(action);
    }
    pub fn draw(&self, w: &mut W) -> Result<()> {
        if self.csi.is_empty() {
            write!(w, "{}", &self.raw)?;
        } else {
            write!(
                w,
                "{}{}{}",
                &self.csi,
                &self.raw,
                CSI_RESET,
            )?;
        }
        Ok(())
    }
    pub fn starts_with(&self, csi: &str, raw: &str) -> bool {
        self.csi == csi && self.raw.starts_with(raw)
    }
    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            csi: self.csi.clone(),
            raw: self.raw.split_off(at),
        }
    }
}

/// a simple representation of a line made of homogeneous parts.
///
/// Note that this does only manages CSI and SGR components
/// and isn't a suitable representation for an arbitrary
/// terminal input or output.
/// I recommend you to NOT try to reuse this hack in another
/// project unless you perfectly understand it.
#[derive(Debug, Default)]
pub struct TLine {
    pub strings: Vec<TString>,
}

impl TLine {
    pub fn from_tty(tty: &str) -> Self {
        let mut parser = vte::Parser::new();
        let mut builder = TLineBuilder::default();
        for byte in tty.bytes() {
            parser.advance(&mut builder, byte);
        }
        builder.to_tline()
    }
    pub fn draw(&self, w: &mut W) -> Result<()> {
        for ts in &self.strings {
            ts.draw(w)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct TLineBuilder {
    cur: Option<TString>,
    strings: Vec<TString>,
}
impl TLineBuilder {
    pub fn to_tline(mut self) -> TLine {
        if let Some(cur) = self.cur {
            self.strings.push(cur);
        }
        TLine {
            strings: self.strings,
        }
    }
}
impl vte::Perform for TLineBuilder {
    fn print(&mut self, c: char) {
        self.cur
            .get_or_insert_with(TString::default)
            .raw
            .push(c);
    }
    fn csi_dispatch(&mut self, params: &[i64], _intermediates: &[u8], _ignore: bool, action: char) {
        if *params == [0] {
            if let Some(cur) = self.cur.take() {
                self.strings.push(cur);
            }
            return;
        }
        if let Some(cur) = self.cur.as_mut() {
            if cur.raw.is_empty() {
                cur.push_csi(params, action);
                return;
            }
        }
        if let Some(cur) = self.cur.take() {
            self.strings.push(cur);
        }
        let mut cur = TString::default();
        cur.push_csi(params, action);
        self.cur = Some(cur);
    }
    fn execute(&mut self, _byte: u8) {}
    fn hook(&mut self, _params: &[i64], _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}
